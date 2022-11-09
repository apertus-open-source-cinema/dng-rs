use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdType};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Ifd {
    entries: Vec<(IfdTagDescriptor, IfdValue)>,
    #[serde(skip)]
    ifd_type: IfdType,
}
impl Ifd {
    pub fn new(ifd_type: IfdType) -> Self {
        Self {
            entries: Vec::new(),
            ifd_type,
        }
    }
    pub fn pretty_yaml(
        &self,
        writer: &mut dyn fmt::Write,
        dump_rational_as_float: bool,
    ) -> Result<(), fmt::Error> {
        for (tag, value) in self.entries.iter() {
            tag.pretty_yaml(writer)?;
            writer.write_str(": ")?;
            tag.pretty_yaml_value(value, writer, dump_rational_as_float)?;
            writer.write_str("\n")?;
        }
        Ok(())
    }
    pub fn insert(&mut self, tag: IfdTagDescriptor, value: IfdValue) {
        self.entries.push((tag, value))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum IfdValue {
    Byte(u8),
    Ascii(String),
    Short(u16),
    Long(u32),
    Rational(u32, u32),
    SByte(i8),
    Undefined(u8),
    SShort(i16),
    SLong(i32),
    SRational(i32, i32),
    Float(f32),
    Double(f64),

    List(Vec<IfdValue>),
    Ifd(Ifd),
}
impl IfdValue {
    pub fn pretty_yaml_plain(
        &self,
        writer: &mut dyn fmt::Write,
        dump_rational_as_float: bool,
    ) -> Result<(), fmt::Error> {
        match &self {
            IfdValue::Byte(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::Ascii(x) => writer.write_fmt(format_args!("\"{x}\""))?,
            IfdValue::Short(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::Long(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::Rational(x, y) => {
                if dump_rational_as_float {
                    writer.write_fmt(format_args!("{}", *x as f32 / *y as f32))?
                } else {
                    writer.write_fmt(format_args!("({x}, {y})"))?
                }
            }
            IfdValue::SByte(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::Undefined(x) => writer.write_fmt(format_args!("{x:#02X}"))?,
            IfdValue::SShort(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::SLong(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::SRational(x, y) => {
                if dump_rational_as_float {
                    writer.write_fmt(format_args!("{}", *x as f32 / *y as f32))?
                } else {
                    writer.write_fmt(format_args!("({x}, {y})"))?
                }
            }
            IfdValue::Float(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::Double(x) => writer.write_fmt(format_args!("{x}"))?,
            IfdValue::List(l) => {
                if let IfdValue::Ifd(_) = l[0] {
                    for x in l.iter() {
                        if let IfdValue::Ifd(ifd) = x {
                            let mut string = String::new();
                            ifd.pretty_yaml(&mut string, dump_rational_as_float)?;
                            let first_line: String = string.lines().take(1).collect();
                            let rest: String = string
                                .lines()
                                .skip(1)
                                .fold(String::new(), |a, b| a + b + "\n");

                            writer.write_fmt(format_args!(
                                "\n- {}\n{}",
                                first_line,
                                textwrap::indent(&rest.trim(), "  ")
                            ))?;
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    writer.write_char('[')?;
                    for (i, x) in l.iter().enumerate() {
                        x.pretty_yaml_plain(writer, dump_rational_as_float)?;
                        if i < l.len() - 1 {
                            writer.write_str(", ")?;
                        }
                    }
                    writer.write_char(']')?;
                }
            }
            IfdValue::Ifd(ifd) => {
                let mut string = String::new();
                ifd.pretty_yaml(&mut string, dump_rational_as_float)?;
                writer.write_fmt(format_args!("\n{}", textwrap::indent(&string.trim(), "  ")))?;
            }
        };
        Ok(())
    }
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            IfdValue::Byte(x) => Some(*x as u32),
            IfdValue::Short(x) => Some(*x as u32),
            IfdValue::Long(x) => Some(*x as u32),
            IfdValue::SByte(x) => Some(*x as u32),
            IfdValue::Undefined(x) => Some(*x as u32),
            IfdValue::SShort(x) => Some(*x as u32),
            IfdValue::SLong(x) => Some(*x as u32),
            _ => None,
        }
    }
}
