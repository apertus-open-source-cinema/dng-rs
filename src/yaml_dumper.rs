use crate::ifd::IfdValue;
use crate::ifd_tag_data::tag_info_parser::{
    IfdFieldDescriptor, IfdTypeInterpretation, MaybeIfdTypeInterpretation,
};
use crate::{Ifd, IfdTagDescriptor};
use itertools::Itertools;

pub struct YamlDumper {
    pub dump_rational_as_float: bool,
}
impl YamlDumper {
    pub fn dump_ifd(&self, ifd: &Ifd) -> String {
        ifd.entries
            .iter()
            .map(|(tag, value)| {
                format!(
                    "{}: {}",
                    self.dump_tag(tag),
                    self.dump_ifd_value(value, tag)
                )
            })
            .intersperse("\n".to_string())
            .collect()
    }
    pub fn dump_tag(&self, tag: &IfdTagDescriptor) -> String {
        match &tag {
            IfdTagDescriptor::Known(tag) => tag.name.to_string(),
            IfdTagDescriptor::Unknown(tag) => format!("{:#02X}", &tag),
        }
    }
    pub fn dump_ifd_value(&self, value: &IfdValue, tag: &IfdTagDescriptor) -> String {
        match tag {
            IfdTagDescriptor::Known(IfdFieldDescriptor {
                interpretation: MaybeIfdTypeInterpretation::Known(interpretation),
                ..
            }) => self.dump_ifd_value_with_type_interpretation(value, interpretation),
            _ => self.dump_ifd_value_plain(value),
        }
    }
    pub fn dump_ifd_value_with_type_interpretation(
        &self,
        value: &IfdValue,
        ifd_type_interpretation: &IfdTypeInterpretation,
    ) -> String {
        match ifd_type_interpretation {
            IfdTypeInterpretation::Enumerated { values } => {
                if let Some(v) = value.as_u32() {
                    if let Some(v) = values.get(&v) {
                        v.to_string()
                    } else {
                        format!("UNKNOWN ({})", self.dump_ifd_value_plain(value))
                    }
                } else {
                    eprintln!(
                        "value {:?} couldn't be made into number (this is illegal for enums",
                        value
                    );
                    self.dump_ifd_value_plain(value)
                }
            }
            _ => self.dump_ifd_value_plain(value),
        }
    }
    pub fn dump_ifd_value_plain(&self, value: &IfdValue) -> String {
        match &value {
            IfdValue::Byte(x) => format!("{x}"),
            IfdValue::Ascii(x) => format!("\"{x}\""),
            IfdValue::Short(x) => format!("{x}"),
            IfdValue::Long(x) => format!("{x}"),
            IfdValue::Rational(x, y) => {
                if self.dump_rational_as_float {
                    format!("{}", *x as f32 / *y as f32)
                } else {
                    format!("({x}, {y})")
                }
            }
            IfdValue::SByte(x) => format!("{x}"),
            IfdValue::Undefined(x) => format!("{x:#02X}"),
            IfdValue::SShort(x) => format!("{x}"),
            IfdValue::SLong(x) => format!("{x}"),
            IfdValue::SRational(x, y) => {
                if self.dump_rational_as_float {
                    format!("{}", *x as f32 / *y as f32)
                } else {
                    format!("({x}, {y})")
                }
            }
            IfdValue::Float(x) => format!("{x}"),
            IfdValue::Double(x) => format!("{x}"),
            IfdValue::List(l) => {
                if let IfdValue::Ifd(_) = l[0] {
                    l.iter()
                        .map(|x| {
                            if let IfdValue::Ifd(ifd) = x {
                                Self::indent_yaml_list_item(self.dump_ifd(ifd))
                            } else {
                                unreachable!()
                            }
                        })
                        .intersperse("\n".to_string())
                        .collect()
                } else {
                    let comma_separated: String = l
                        .iter()
                        .map(|x| self.dump_ifd_value_plain(x))
                        .intersperse(",".to_string())
                        .collect();
                    format!("[{comma_separated}]")
                }
            }
            IfdValue::Ifd(ifd) => {
                format!("\n{}", textwrap::indent(&self.dump_ifd(ifd), "  "))
            }
        }
    }
    fn indent_yaml_list_item(x: String) -> String {
        let first_line: String = x.lines().take(1).collect();
        let rest: String = x.lines().skip(1).fold(String::new(), |a, b| a + b + "\n");

        format!(
            "\n- {}\n{}",
            first_line,
            textwrap::indent(&rest.trim(), "  ")
        )
    }
}
