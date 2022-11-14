use crate::ifd::{Ifd, IfdEntry, IfdValue};
use crate::ifd_tag_data::tag_info_parser::IfdTypeInterpretation;
use itertools::Itertools;
use std::sync::Arc;

#[derive(Default)]
pub struct IfdYamlDumper {
    pub dump_rational_as_float: bool,
    pub visitor: Option<Arc<dyn Fn(IfdEntry) -> Option<String>>>,
}
impl IfdYamlDumper {
    pub fn dump_ifd(&self, ifd: &Ifd) -> String {
        ifd.entries
            .iter()
            .map(|entry| {
                format!(
                    "{}: {}{}\n",
                    entry.tag,
                    self.dump_tag_if_needed(&entry),
                    self.dump_ifd_value(&entry)
                )
            })
            .collect()
    }
    pub fn dump_ifd_value(&self, entry: &IfdEntry) -> String {
        if entry.tag.get_known_type_interpretation().is_some() {
            self.dump_ifd_value_with_type_interpretation(entry)
        } else {
            self.dump_ifd_value_plain(entry)
        }
    }
    fn dump_ifd_value_with_type_interpretation(&self, entry: &IfdEntry) -> String {
        if let Some(s) = self
            .visitor
            .clone()
            .and_then(|visitor| visitor(entry.clone()))
        {
            return s.clone();
        }

        match entry.tag.get_known_type_interpretation().unwrap() {
            IfdTypeInterpretation::Enumerated { values } => {
                if let Some(v) = entry.value.as_u32() {
                    if let Some(v) = values.get(&v) {
                        v.to_string()
                    } else {
                        format!("UNKNOWN ({})", self.dump_ifd_value_plain(entry))
                    }
                } else {
                    unreachable!()
                }
            }
            _ => self.dump_ifd_value_plain(entry),
        }
    }
    fn dump_ifd_value_plain(&self, entry: &IfdEntry) -> String {
        match &entry.value {
            IfdValue::Byte(x) => format!("{x}"),
            IfdValue::Ascii(x) => format!("\"{x}\""),
            IfdValue::Short(x) => format!("{x}"),
            IfdValue::Long(x) => format!("{x}"),
            IfdValue::Rational(x, y) => {
                if self.dump_rational_as_float {
                    format!("{}", *x as f32 / *y as f32)
                } else {
                    format!("{x}/{y}")
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
                    format!("{x}/{y}")
                }
            }
            IfdValue::Float(x) => format!("{x}"),
            IfdValue::Double(x) => format!("{x}"),
            IfdValue::List(l) => {
                if let IfdValue::Ifd(_) = l[0].value {
                    l.iter()
                        .enumerate()
                        .map(|(_i, x)| {
                            if let IfdValue::Ifd(ifd) = &x.value {
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
                        .map(|x| self.dump_ifd_value(x))
                        .intersperse(", ".to_string())
                        .collect();
                    format!("[{comma_separated}]")
                }
            }
            IfdValue::Ifd(ifd) => {
                format!("\n{}", textwrap::indent(&self.dump_ifd(ifd), "  "))
            }
        }
    }
    fn dump_tag_if_needed(&self, entry: &IfdEntry) -> String {
        if let Some(types) = entry.tag.get_known_value_type() {
            if types.contains(&entry.value.get_ifd_value_type()) {
                return "".to_string();
            }
        }
        format!(
            "!{} ",
            serde_plain::to_string(&entry.value.get_ifd_value_type()).unwrap()
        )
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
