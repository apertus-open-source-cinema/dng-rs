pub mod tag_info_parser;

use self::tag_info_parser::MaybeExifTypeInterpretation;
use num_derive::FromPrimitive;
use once_cell::sync::Lazy;
use serde::{de, Deserialize, Serialize};
use std::fmt;
use tag_info_parser::ExifFieldDescriptor;

const IFD_JSON: &str = include_str!("ifd.json");
const EXIF_JSON: &str = include_str!("exif.json");
const GPS_INFO_JSON: &str = include_str!("gps_info.json");

static IFD_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&IFD_JSON).unwrap());
static EXIF_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&EXIF_JSON).unwrap());
static GPS_INFO_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&GPS_INFO_JSON).unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IfdType {
    Ifd,
    Exif,
    GpsInfo,
}
impl IfdType {
    pub fn get_namespace(&self) -> &Vec<ExifFieldDescriptor> {
        match self {
            IfdType::Ifd => &IFD_TAGS,
            IfdType::Exif => &EXIF_TAGS,
            IfdType::GpsInfo => &GPS_INFO_TAGS,
        }
    }
    pub fn combined_namespace() -> impl Iterator<Item = &'static ExifFieldDescriptor> {
        IFD_TAGS
            .iter()
            .chain(EXIF_TAGS.iter())
            .chain(GPS_INFO_TAGS.iter())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(transparent)]
pub struct ExifMetadata(Vec<(ExifTag, ExifValue)>);
impl ExifMetadata {
    pub fn pretty_yaml(&self, writer: &mut dyn fmt::Write) -> Result<(), fmt::Error> {
        for (tag, value) in self.0.iter() {
            tag.pretty_yaml(writer)?;
            writer.write_str(": ")?;
            tag.pretty_yaml_value(value, writer)?;
            writer.write_str("\n")?;
        }
        Ok(())
    }
    pub fn insert(&mut self, tag: ExifTag, value: ExifValue) {
        self.0.push((tag, value))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ExifTag {
    Known(ExifFieldDescriptor),
    Unknown(u16),
}
impl ExifTag {
    pub fn from_number(tag: u16, ifd_kind: IfdType) -> Self {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.tag == tag) {
            Self::Known(description.clone())
        } else {
            Self::Unknown(tag)
        }
    }
    pub fn from_name<'de, D>(name: &str) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if let Some(description) = IfdType::combined_namespace().find(|x| x.name == name) {
            Ok(Self::Known(description.clone()))
        } else {
            Err(de::Error::custom(format!("No Tag named '{}' known", name)))
        }
    }
    fn pretty_yaml(&self, writer: &mut dyn fmt::Write) -> Result<(), fmt::Error> {
        match &self {
            ExifTag::Known(tag) => writer.write_str(&tag.name),
            ExifTag::Unknown(tag) => writer.write_fmt(format_args!("{:#02X}", &tag)),
        }
    }
    pub fn pretty_yaml_value(
        &self,
        value: &ExifValue,
        writer: &mut dyn fmt::Write,
    ) -> Result<(), fmt::Error> {
        match self {
            ExifTag::Known(ExifFieldDescriptor {
                interpretation: MaybeExifTypeInterpretation::Known(interpretation),
                ..
            }) => interpretation.pretty_yaml_value(value, writer),
            _ => value.pretty_yaml_plain(writer),
        }
    }
}

impl<'de> Deserialize<'de> for ExifTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.starts_with("0x") {
            let without_prefix = s.trim_start_matches("0x");
            Ok(Self::from_number(
                u16::from_str_radix(without_prefix, 16).map_err(de::Error::custom)?,
                IfdType::Ifd, // TODO: this might be wrong in the future; additional context is needed here
            ))
        } else {
            Self::from_name::<D>(&s)
        }
    }
}

impl Serialize for ExifTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self {
            ExifTag::Known(tag) => serializer.serialize_str(&tag.name),
            ExifTag::Unknown(tag) => serializer.serialize_str(&tag.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExifValue {
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

    List(Vec<ExifValue>),
    Ifd(ExifMetadata),
}
impl ExifValue {
    fn pretty_yaml_plain(&self, writer: &mut dyn fmt::Write) -> Result<(), fmt::Error> {
        match &self {
            ExifValue::Byte(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::Ascii(x) => writer.write_fmt(format_args!("\"{x}\""))?,
            ExifValue::Short(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::Long(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::Rational(x, y) => writer.write_fmt(format_args!("({x}, {y})"))?,
            ExifValue::SByte(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::Undefined(x) => writer.write_fmt(format_args!("{x:#02X}"))?,
            ExifValue::SShort(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::SLong(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::SRational(x, y) => writer.write_fmt(format_args!("({x}, {y})"))?,
            ExifValue::Float(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::Double(x) => writer.write_fmt(format_args!("{x}"))?,
            ExifValue::List(l) => {
                if let ExifValue::Ifd(_) = l[0] {
                    for x in l.iter() {
                        if let ExifValue::Ifd(ifd) = x {
                            let mut string = String::new();
                            ifd.pretty_yaml(&mut string)?;
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
                        x.pretty_yaml_plain(writer)?;
                        if i < l.len() - 1 {
                            writer.write_str(", ")?;
                        }
                    }
                    writer.write_char(']')?;
                }
            }
            ExifValue::Ifd(ifd) => {
                let mut string = String::new();
                ifd.pretty_yaml(&mut string)?;
                writer.write_fmt(format_args!("\n{}", textwrap::indent(&string.trim(), "  ")))?;
            }
        };
        Ok(())
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            ExifValue::Byte(x) => Some(*x as u32),
            ExifValue::Short(x) => Some(*x as u32),
            ExifValue::Long(x) => Some(*x as u32),
            ExifValue::SByte(x) => Some(*x as u32),
            ExifValue::Undefined(x) => Some(*x as u32),
            ExifValue::SShort(x) => Some(*x as u32),
            ExifValue::SLong(x) => Some(*x as u32),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, FromPrimitive, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExifValueType {
    Byte = 1,
    Ascii = 2,
    Short = 3,
    Long = 4,
    Rational = 5,
    SByte = 6,
    Undefined = 7,
    SShort = 8,
    SLong = 9,
    SRational = 10,
    Float = 11,
    Double = 12,
}
impl ExifValueType {
    pub fn needed_bytes(&self) -> u32 {
        match self {
            ExifValueType::Byte => 1,
            ExifValueType::Ascii => 1,
            ExifValueType::Short => 2,
            ExifValueType::Long => 4,
            ExifValueType::Rational => 8,
            ExifValueType::SByte => 1,
            ExifValueType::Undefined => 1,
            ExifValueType::SShort => 2,
            ExifValueType::SLong => 4,
            ExifValueType::SRational => 4,
            ExifValueType::Float => 4,
            ExifValueType::Double => 8,
        }
    }
}
