use crate::ifd::{Ifd, IfdValue};
use crate::ifd::{IfdEntry, IfdPath};
use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdValueType};
use crate::ifd_tag_data::tag_info_parser::{IfdType, IfdTypeInterpretation};
use fraction::Ratio;
use lazy_regex::regex_captures;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use yaml_peg::parser::parse;
use yaml_peg::parser::PError;
use yaml_peg::repr::RcRepr;
use yaml_peg::Node;

#[derive(Debug)]
pub enum IfdYamlParserError {
    PError(PError),
    IoError(io::Error),
    Other(u64, String),
}
impl From<PError> for IfdYamlParserError {
    fn from(e: PError) -> Self {
        Self::PError(e)
    }
}
impl From<io::Error> for IfdYamlParserError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}
impl Display for IfdYamlParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IfdYamlParserError::PError(PError::Terminate { name, msg }) => {
                f.write_fmt(format_args!("Error '{}' at:\n{}", name, msg))
            }
            IfdYamlParserError::PError(PError::Mismatch) => {
                f.write_fmt(format_args!("PError::Mismatch"))
            }
            IfdYamlParserError::Other(pos, e) => {
                f.write_fmt(format_args!("Other Error '{}' at: {}", e, pos))
            }
            IfdYamlParserError::IoError(e) => f.write_fmt(format_args!("IoError '{}'", e)),
        }
    }
}

macro_rules! err {
    ($pos:expr, $($format_args:tt)*) => {
        IfdYamlParserError::Other($pos, format!($($format_args)*))
    };
}

#[derive(Default)]
pub struct IfdYamlParser {
    path: PathBuf,
}
impl IfdYamlParser {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn parse_from_str(&self, source: &str) -> Result<Ifd, IfdYamlParserError> {
        let parsed_yaml = parse(source)?;
        self.parse_ifd(&parsed_yaml[0], IfdType::Ifd, IfdPath::default())
    }

    fn parse_ifd(
        &self,
        source: &Node<RcRepr>,
        ifd_type: IfdType,
        path: IfdPath,
    ) -> Result<Ifd, IfdYamlParserError> {
        let mut ifd = Ifd::new(ifd_type, path.clone());
        for (key, value) in source
            .as_map()
            .map_err(|pos| err!(pos, "cant read {source:?} as map (required for ifd)"))?
            .iter()
        {
            let tag = self.parse_ifd_tag(key, ifd_type)?;

            // if we have offsets we need to emit two tags (offsets and lengths), thus we need to handle this directly
            if let Some(IfdTypeInterpretation::Offsets { lengths }) =
                tag.get_known_type_interpretation()
            {
                let lengths_tag = IfdTagDescriptor::from_name(lengths, ifd_type)
                    .map_err(|e| err!(value.pos(), "{}", e.to_string()))?;

                let parse_offset_entry = |value: &Node<RcRepr>,
                                          path: IfdPath|
                 -> Result<
                    Option<(IfdEntry, IfdEntry)>,
                    IfdYamlParserError,
                > {
                    let str = value.as_str().map_err(|pos| {
                        IfdYamlParserError::Other(pos, format!("{value:?} cant be made into str"))
                    })?;
                    if let Some((_whole, file_path)) = regex_captures!("file://(.*)", str) {
                        let file_path = self.path.join(file_path);
                        let mut file = File::open(file_path)?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)?;
                        let len = buffer.len();
                        let offsets_entry = IfdEntry {
                            value: IfdValue::Offsets(Arc::new(buffer)),
                            path: path.clone(),
                            tag: tag.clone(),
                        };
                        let lengths_entry = IfdEntry {
                            value: IfdValue::Long(len as u32),
                            path: path.with_last_tag_replaced(lengths_tag.clone()),
                            tag: lengths_tag.clone(),
                        };
                        Ok(Some((offsets_entry, lengths_entry)))
                    } else {
                        Ok(None)
                    }
                };

                match value.as_seq() {
                    Ok(seq) => {
                        let mapped: Result<Vec<_>, IfdYamlParserError> = seq
                            .iter()
                            .enumerate()
                            .map(|(i, x)| parse_offset_entry(x, path.chain_list_index(i as u16)))
                            .collect();
                        let mapped = mapped?;
                        if mapped.iter().all(|x| x.is_some()) {
                            let (offsets, lengths): (Vec<_>, Vec<_>) =
                                mapped.into_iter().map(|x| x.unwrap()).unzip();
                            ifd.insert(IfdEntry {
                                value: IfdValue::List(offsets),
                                path: path.clone(),
                                tag: tag.clone(),
                            });
                            ifd.insert(IfdEntry {
                                value: IfdValue::List(lengths),
                                path: path.with_last_tag_replaced(lengths_tag),
                                tag: tag.clone(),
                            });
                            continue;
                        }
                    }
                    Err(_) => {
                        if let Some((offsets, lengths)) = parse_offset_entry(value, path.clone())? {
                            ifd.insert(offsets);
                            ifd.insert(lengths);
                            continue;
                        }
                    }
                }
            }

            ifd.insert(self.parse_ifd_entry(value, tag, path.clone(), None)?)
        }

        Ok(ifd)
    }

    fn parse_ifd_tag(
        &self,
        source: &Node<RcRepr>,
        ifd_type: IfdType,
    ) -> Result<IfdTagDescriptor, IfdYamlParserError> {
        if let Ok(i) = source.as_int() {
            Ok(IfdTagDescriptor::from_number(i as u16, ifd_type))
        } else if let Ok(str) = source.as_str() {
            if str.starts_with("0x") {
                if let Ok(tag) = u16::from_str_radix(&str[2..], 16) {
                    Ok(IfdTagDescriptor::from_number(tag, ifd_type))
                } else {
                    Err(err!(source.pos(), "couldnt parse hex string '{str}'"))
                }
            } else {
                IfdTagDescriptor::from_name(str, ifd_type)
                    .map_err(|e| IfdYamlParserError::Other(source.pos(), e))
            }
        } else {
            Err(err!(source.pos(), "couldnt parse tag '{source:?}"))
        }
    }

    fn parse_ifd_entry(
        &self,
        value: &Node<RcRepr>,
        tag: IfdTagDescriptor,
        path: IfdPath,
        parent_yaml_tag: Option<&str>,
    ) -> Result<IfdEntry, IfdYamlParserError> {
        let value = if let Ok(_) = value.as_map() {
            let ifd_type = if let Some(IfdTypeInterpretation::IfdOffset { ifd_type }) =
                tag.get_known_type_interpretation()
            {
                *ifd_type
            } else {
                IfdType::Ifd
            };
            IfdValue::Ifd(self.parse_ifd(value, ifd_type, path.chain_tag(tag.clone()))?)
        } else if let Ok(seq) = value.as_seq() {
            let result: Result<Vec<_>, _> = seq
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    self.parse_ifd_entry(
                        node,
                        tag.clone(),
                        path.chain_list_index(i as u16),
                        Some(value.tag()),
                    )
                })
                .collect();
            IfdValue::List(result?)
        } else {
            loop {
                // this is the 'well-known' loop hack
                // we try to parse the value as a file
                if let Ok(str) = value.as_str() {
                    if let Some((_whole, file_path)) = regex_captures!("file://(.*)", str) {
                        let file_path = self.path.join(file_path);
                        let mut file = File::open(file_path)?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)?;
                        break IfdValue::List(
                            buffer
                                .iter()
                                .enumerate()
                                .map(|(i, b)| IfdEntry {
                                    value: IfdValue::Byte(*b),
                                    path: path.chain_list_index(i as u16),
                                    tag: tag.clone(),
                                })
                                .collect(),
                        );
                    }
                }

                // we are dealing with a scalar
                break self.parse_ifd_scalar_value(value, tag.clone(), parent_yaml_tag)?;
            }
        };

        let path = path.chain_tag(tag.clone());
        Ok(IfdEntry { value, path, tag })
    }

    fn parse_ifd_scalar_value(
        &self,
        value: &Node<RcRepr>,
        tag: IfdTagDescriptor,
        parent_yaml_tag: Option<&str>,
    ) -> Result<IfdValue, IfdYamlParserError> {
        let yaml_tag = parent_yaml_tag.unwrap_or(value.tag());
        let dtypes: Vec<IfdValueType> = if let Ok(ty) =
            serde_plain::from_str::<IfdValueType>(yaml_tag)
        {
            Ok(vec![ty])
        } else if let Some(types) = tag.get_known_value_type() {
            Ok(types.clone())
        } else {
            Err(err!(value.pos(), "couldnt determine dtype of tag '{tag}'. if the IFD tag is unknown, the dtype must be specified explicitly with a YAML tag"))
        }?;

        match tag.get_known_type_interpretation() {
            Some(IfdTypeInterpretation::Enumerated { values }) => {
                let str = value
                    .as_str()
                    .map_err(|pos| err!(pos, "cant read {value:?} as a string"))?;
                let matching_values: Vec<_> = values
                    .iter()
                    .filter(|(_, v)| v.to_lowercase().contains(&str.to_lowercase()))
                    .collect();
                let (numeric, _) = match matching_values.len() {
                    0 => Err(err!(value.pos(), "{str} didnt match any enum variant for field {tag}.\nPossible variants are: {values:?}"))?,
                    1 => matching_values[0],
                    _ => Err(err!(value.pos(), "{str} is ambiguous for tag {tag}. Disambiguate between: {matching_values:?}"))?,
                };
                for dtype in dtypes {
                    return Ok(match dtype {
                        IfdValueType::Byte => IfdValue::Byte(*numeric as u8),
                        IfdValueType::Short => IfdValue::Short(*numeric as u16),
                        IfdValueType::Long => IfdValue::Long(*numeric as u32),
                        IfdValueType::Undefined => IfdValue::Undefined(*numeric as u8),
                        _ => unreachable!(),
                    });
                }
                Err(err!(value.pos(), "No dtype worked"))
            }
            _ => {
                for dtype in dtypes {
                    match self.parse_ifd_primitive_value(value, dtype) {
                        Ok(v) => return Ok(v),
                        Err(_err) => {} // eprintln!("{err:#?}"),
                    }
                }
                Err(err!(value.pos(), "No dtype worked for tag {tag}"))
            }
        }
    }

    fn parse_ifd_primitive_value(
        &self,
        value: &Node<RcRepr>,
        dtype: IfdValueType,
    ) -> Result<IfdValue, IfdYamlParserError> {
        let str = value.as_value().map_err(|pos| {
            IfdYamlParserError::Other(pos, format!("{value:?} is not a scalar value"))
        })?;

        macro_rules! parse_int_like {
            ($value:ident, $name:literal) => {{
                let int = $value
                    .as_int()
                    .map_err(|pos| err!(pos, "couldn't parse {str} as {}", $name))?;
                int.try_into().map_err(|e| err!(value.pos(), "{e:?}"))?
            }};
        }

        Ok(match dtype {
            IfdValueType::Byte => IfdValue::Byte(parse_int_like!(value, "BYTE")),
            IfdValueType::Ascii => IfdValue::Ascii(str.to_string()),
            IfdValueType::Short => IfdValue::Short(parse_int_like!(value, "SHORT")),
            IfdValueType::Long => IfdValue::Long(parse_int_like!(value, "LONG")),
            IfdValueType::SByte => IfdValue::SByte(parse_int_like!(value, "SBYTE")),
            IfdValueType::Undefined => IfdValue::Undefined(parse_int_like!(value, "UNDEFINED")),
            IfdValueType::SShort => IfdValue::SShort(parse_int_like!(value, "SSHORT")),
            IfdValueType::SLong => IfdValue::SLong(parse_int_like!(value, "SLONG")),

            IfdValueType::Rational => {
                if let Some((_whole, numerator, denominator)) =
                    regex_captures!("([0-9]+)\\s*/\\s*([0-9]+)", str)
                {
                    if let (Ok(numerator), Ok(denominator)) =
                        (numerator.parse(), denominator.parse())
                    {
                        IfdValue::Rational(numerator, denominator)
                    } else {
                        Err(err!(value.pos(), "couldn't parse {str} as RATIONAL"))?
                    }
                } else if let Ok(float) = str.parse::<f32>() {
                    let fraction = Ratio::<i32>::approximate_float(float).ok_or_else(|| {
                        err!(value.pos(), "couldnt find a fraction for float {float}")
                    })?;
                    IfdValue::Rational(*fraction.numer() as u32, *fraction.denom() as u32)
                } else {
                    Err(err!(value.pos(), "couldn't parse {str} as RATIONAL"))?
                }
            }
            IfdValueType::SRational => {
                if let Some((_whole, numerator, denominator)) =
                    regex_captures!("([\\-0-9]+)\\s*/\\s*([\\-0-9]+)", str)
                {
                    if let (Ok(numerator), Ok(denominator)) =
                        (numerator.parse(), denominator.parse())
                    {
                        IfdValue::SRational(numerator, denominator)
                    } else {
                        Err(err!(value.pos(), "couldn't parse {str} as SRATIONAL"))?
                    }
                } else if let Ok(float) = str.parse::<f32>() {
                    let fraction = Ratio::<i32>::approximate_float(float).ok_or_else(|| {
                        err!(value.pos(), "couldnt find a fraction for float {float}")
                    })?;
                    IfdValue::SRational(*fraction.numer(), *fraction.denom())
                } else {
                    Err(err!(value.pos(), "couldn't parse {str} as SRATIONAL"))?
                }
            }

            IfdValueType::Float => IfdValue::Float(match value.as_value() {
                Ok(v) => v
                    .parse()
                    .map_err(|_e| err!(value.pos(), "couldn't parse {str} as FLOAT"))?,
                Err(pos) => Err(err!(pos, "couldn't parse {str} as FLOAT"))?,
            }),
            IfdValueType::Double => IfdValue::Double(value.as_float().map_err(|pos| {
                IfdYamlParserError::Other(pos, format!("couldn't parse {str} as DOUBLE"))
            })?),
        })
    }
}
