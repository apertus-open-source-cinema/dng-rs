pub mod exif_yaml;
pub mod tag_info_parser;

use num_derive::FromPrimitive;
use once_cell::sync::Lazy;
use serde::{de, Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use tag_info_parser::ExifFieldDescriptor;

use self::tag_info_parser::MaybeExifTypeInterpretation;

const EXIF_JSON: &str = include_str!("exif.json");
const EXIF_IFD_JSON: &str = include_str!("exif_ifd.json");
const GPS_INFO_IFD_JSON: &str = include_str!("gps_info_ifd.json");

static EXIF_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&EXIF_JSON).unwrap());
static EXIF_IFD_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&EXIF_IFD_JSON).unwrap());
static GPS_INFO_IFD_TAGS: Lazy<Vec<ExifFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&GPS_INFO_IFD_JSON).unwrap());

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExifMetadata(pub HashMap<ExifTag, ExifValue>);
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
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ExifTag {
    Known(ExifFieldDescriptor),
    Unknown(u16),
}
impl ExifTag {
    pub fn from_number(tag: u16) -> Self {
        if let Some(description) = EXIF_TAGS.iter().find(|x| x.tag == tag) {
            Self::Known(description.clone())
        } else {
            Self::Unknown(tag)
        }
    }
    pub fn from_name<'de, D>(name: &str) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if let Some(description) = EXIF_TAGS.iter().find(|x| x.name == name) {
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
}
impl ExifValue {
    fn pretty_yaml_plain(&self, writer: &mut dyn fmt::Write) -> Result<(), fmt::Error> {
        match &self {
            ExifValue::Byte(x) => writer.write_fmt(format_args!("{x:#02X}"))?,
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
                writer.write_char('[')?;
                for (i, x) in l.iter().enumerate() {
                    x.pretty_yaml_plain(writer)?;
                    if i < l.len() - 1 {
                        writer.write_str(", ")?;
                    }
                }
                writer.write_char(']')?;
            }
        };
        Ok(())
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            ExifValue::Byte(x) => Some(*x as u32),
            ExifValue::Ascii(_) => None,
            ExifValue::Short(x) => Some(*x as u32),
            ExifValue::Long(x) => Some(*x as u32),
            ExifValue::Rational(_, _) => None,
            ExifValue::SByte(x) => Some(*x as u32),
            ExifValue::Undefined(x) => Some(*x as u32),
            ExifValue::SShort(x) => Some(*x as u32),
            ExifValue::SLong(x) => Some(*x as u32),
            ExifValue::SRational(_, _) => None,
            ExifValue::Float(_) => None,
            ExifValue::Double(_) => None,
            ExifValue::List(_) => None,
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
