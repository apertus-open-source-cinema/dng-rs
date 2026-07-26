use crate::ifd::{Ifd, IfdEntryRef, IfdPath, IfdValue};
use crate::tags::{IfdTypeInterpretation, IfdValueType};
use std::sync::Arc;

/// Dumps an [Ifd] struct into a friendly human readable text-representation
#[derive(Default)]
pub struct IfdYamlDumper {
    pub dump_rational_as_float: bool,
    pub visitor: Option<Arc<dyn Fn(IfdEntryRef) -> Option<String>>>,
}
impl IfdYamlDumper {
    pub fn dump_ifd(&self, ifd: &Ifd) -> String {
        self.dump_ifd_with_path(ifd, &IfdPath::default())
    }
    pub fn dump_ifd_with_path(&self, ifd: &Ifd, path: &IfdPath) -> String {
        ifd.entries
            .iter()
            .map(|entry| {
                format!(
                    "{}: {}{}\n",
                    entry.tag,
                    self.dump_tag_if_needed(entry.build_ref(&path.chain_tag(entry.tag))),
                    self.dump_ifd_value(entry.build_ref(&path.chain_tag(entry.tag)))
                )
            })
            .collect()
    }
    pub fn dump_ifd_value(&self, entry: IfdEntryRef) -> String {
        if entry.tag.type_interpretation().is_some() {
            self.dump_ifd_value_with_type_interpretation(entry)
        } else {
            self.dump_ifd_value_plain(entry)
        }
    }
    fn dump_ifd_value_with_type_interpretation(&self, entry: IfdEntryRef) -> String {
        if let Some(s) = self.visitor.clone().and_then(|visitor| visitor(entry)) {
            return s;
        }

        match entry.tag.type_interpretation().unwrap() {
            IfdTypeInterpretation::Enumerated { values } => {
                if let Some(num) = entry.value.as_u32() {
                    if let Some((_, v)) = values.iter().find(|(k, _)| *k == num) {
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
    fn dump_ifd_value_plain(&self, entry: IfdEntryRef) -> String {
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
            IfdValue::SignedByte(x) => format!("{x}"),
            IfdValue::Undefined(x) => format!("{x:#02X}"),
            IfdValue::SignedShort(x) => format!("{x}"),
            IfdValue::SignedLong(x) => format!("{x}"),
            IfdValue::SignedRational(x, y) => {
                if self.dump_rational_as_float {
                    format!("{}", *x as f32 / *y as f32)
                } else {
                    format!("{x}/{y}")
                }
            }
            IfdValue::Float(x) => format!("{x}"),
            IfdValue::Double(x) => format!("{x}"),
            IfdValue::List(l) => {
                if let IfdValue::Ifd(_) = l[0] {
                    l.iter()
                        .enumerate()
                        .map(|(i, x)| {
                            if let IfdValue::Ifd(ifd) = &x {
                                Self::indent_yaml_list_item(self.dump_ifd_with_path(
                                    ifd,
                                    &entry.path.chain_list_index(i as u16),
                                ))
                            } else {
                                unreachable!()
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("\n")
                } else {
                    let comma_separated: String = l
                        .iter()
                        .enumerate()
                        .map(|(i, x)| {
                            self.dump_ifd_value(IfdEntryRef {
                                value: x,
                                path: &entry.path.chain_list_index(i as u16),
                                tag: entry.tag,
                            })
                        })
                        .collect::<Vec<String>>()
                        .join(", ");
                    format!("[{comma_separated}]")
                }
            }
            IfdValue::Ifd(ifd) => {
                format!(
                    "\n{}",
                    textwrap::indent(&self.dump_ifd_with_path(ifd, entry.path), "  ")
                )
            }
            IfdValue::Offsets(_) => unimplemented!(),
        }
    }
    fn dump_tag_if_needed(&self, entry: IfdEntryRef) -> String {
        if let Some(types) = entry.tag.known_value_type() {
            if types.contains(&entry.value.ifd_value_type()) {
                return "".to_string();
            }
        }
        format!(
            "!{} ",
            Self::dump_ifd_value_type(&entry.value.ifd_value_type())
        )
    }
    fn dump_ifd_value_type(v: &IfdValueType) -> &str {
        match v {
            IfdValueType::Byte => "BYTE",
            IfdValueType::Ascii => "ASCII",
            IfdValueType::Short => "SHORT",
            IfdValueType::Long => "LONG",
            IfdValueType::Rational => "RATIONAL",
            IfdValueType::SignedByte => "SBYTE",
            IfdValueType::Undefined => "UNDEFINED",
            IfdValueType::SignedShort => "SSHORT",
            IfdValueType::SignedLong => "SLONG",
            IfdValueType::SignedRational => "SRATIONAL",
            IfdValueType::Float => "FLOAT",
            IfdValueType::Double => "DOUBLE",
        }
    }
    fn indent_yaml_list_item(x: String) -> String {
        let first_line: String = x.lines().take(1).collect();
        let rest: String = x.lines().skip(1).fold(String::new(), |a, b| a + b + "\n");

        format!(
            "\n- {}\n{}",
            first_line,
            textwrap::indent(rest.trim(), "  ")
        )
    }
}
