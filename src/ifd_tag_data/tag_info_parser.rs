use num_derive::FromPrimitive;
use once_cell::sync::Lazy;
use serde::{de, Deserialize, Deserializer};
use serde_hex::{SerHex, StrictPfx};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

const IFD_JSON: &str = include_str!("ifd.json");
const EXIF_JSON: &str = include_str!("exif.json");
const GPS_INFO_JSON: &str = include_str!("gps_info.json");

static IFD_TAGS: Lazy<Vec<IfdFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&IFD_JSON).unwrap());
static EXIF_TAGS: Lazy<Vec<IfdFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&EXIF_JSON).unwrap());
static GPS_INFO_TAGS: Lazy<Vec<IfdFieldDescriptor>> =
    Lazy::new(|| serde_json::from_str(&GPS_INFO_JSON).unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IfdType {
    Ifd,
    Exif,
    GpsInfo,
}
impl IfdType {
    pub fn get_namespace(&self) -> &Vec<IfdFieldDescriptor> {
        match self {
            IfdType::Ifd => &IFD_TAGS,
            IfdType::Exif => &EXIF_TAGS,
            IfdType::GpsInfo => &GPS_INFO_TAGS,
        }
    }
    pub fn combined_namespace() -> impl Iterator<Item = &'static IfdFieldDescriptor> {
        IFD_TAGS
            .iter()
            .chain(EXIF_TAGS.iter())
            .chain(GPS_INFO_TAGS.iter())
    }
}

#[derive(Deserialize, Debug, Clone, Eq)]
pub struct IfdFieldDescriptor {
    pub name: String,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub tag: u16,
    pub dtype: Vec<IfdValueType>,
    pub interpretation: MaybeIfdTypeInterpretation,
    pub count: IfdCount,
    pub description: String,
    pub long_description: String,
    pub references: String,
}
impl PartialEq for IfdFieldDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum IfdCount {
    N,
    ConcreteValue(u32),
}
impl<'de> Deserialize<'de> for IfdCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s == "N" {
            Ok(Self::N)
        } else {
            Ok(if let Ok(x) = u32::from_str_radix(&s, 10) {
                Self::ConcreteValue(x)
            } else {
                Self::N
            })
        }
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum MaybeIfdTypeInterpretation {
    Known(IfdTypeInterpretation),
    Other(serde_json::Value),
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "UPPERCASE")]
pub enum IfdTypeInterpretation {
    Enumerated {
        #[serde(deserialize_with = "deserialize_enumerated_values")]
        values: HashMap<u32, String>,
    },
    Bitflags {
        #[serde(deserialize_with = "deserialize_bitflags_values")]
        values: HashMap<u8, String>,
    },
    CfaPattern,
    Default,
    IfdOffset {
        ifd_type: IfdType,
    },
    Offsets {
        lengths: String,
    },
    Blob,
}
fn deserialize_enumerated_values<'de, D>(deserializer: D) -> Result<HashMap<u32, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let original = HashMap::<String, String>::deserialize(deserializer)?;
    let mapped: Result<HashMap<_, _>, _> = original
        .iter()
        .map(|(value, key)| {
            let key = if key.starts_with("0x") {
                u32::from_str_radix(&key[2..], 16).map_err(de::Error::custom)?
            } else {
                u32::from_str_radix(key, 10).map_err(de::Error::custom)?
            };
            Ok((key, value.to_string()))
        })
        .collect();
    Ok(mapped?)
}
fn deserialize_bitflags_values<'de, D>(deserializer: D) -> Result<HashMap<u8, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let original = HashMap::<String, String>::deserialize(deserializer)?;
    let mapped: Result<HashMap<_, _>, _> = original
        .iter()
        .map(|(value, key)| {
            Ok((
                u8::from_str_radix(&key[4..], 10).map_err(de::Error::custom)?,
                value.to_string(),
            ))
        })
        .collect();
    Ok(mapped?)
}

#[derive(Debug, Clone, Eq)]
pub enum IfdTagDescriptor {
    Known(IfdFieldDescriptor),
    Unknown(u16),
}
impl IfdTagDescriptor {
    pub fn from_number(tag: u16, ifd_kind: IfdType) -> Self {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.tag == tag) {
            Self::Known(description.clone())
        } else {
            Self::Unknown(tag)
        }
    }
    pub fn from_name(name: &str) -> Result<Self, String> {
        if let Some(description) = IfdType::combined_namespace().find(|x| x.name == name) {
            Ok(Self::Known(description.clone()))
        } else {
            Err(format!("No Tag named '{}' known", name))
        }
    }
    pub fn get_known_type_interpretation(&self) -> Option<&IfdTypeInterpretation> {
        match self {
            IfdTagDescriptor::Known(IfdFieldDescriptor {
                interpretation: MaybeIfdTypeInterpretation::Known(interpretation),
                ..
            }) => Some(interpretation),
            _ => None,
        }
    }
    pub fn get_known_name(&self) -> Option<&str> {
        match self {
            Self::Known(descriptor) => Some(&descriptor.name),
            Self::Unknown(_) => None,
        }
    }
    pub fn get_tag(&self) -> u16 {
        match self {
            IfdTagDescriptor::Known(descriptor) => descriptor.tag,
            IfdTagDescriptor::Unknown(tag) => *tag,
        }
    }
}
impl Display for IfdTagDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            IfdTagDescriptor::Known(tag) => tag.name.fmt(f),
            IfdTagDescriptor::Unknown(tag) => f.write_fmt(format_args!("{:#02X}", &tag)),
        }
    }
}
impl PartialEq for IfdTagDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.get_tag() == other.get_tag()
    }
}
impl<'de> Deserialize<'de> for IfdTagDescriptor {
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
            Self::from_name(&s).map_err(serde::de::Error::custom)
        }
    }
}

#[derive(Clone, Copy, Debug, FromPrimitive, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IfdValueType {
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
impl IfdValueType {
    pub fn needed_bytes(&self) -> u32 {
        match self {
            IfdValueType::Byte => 1,
            IfdValueType::Ascii => 1,
            IfdValueType::Short => 2,
            IfdValueType::Long => 4,
            IfdValueType::Rational => 8,
            IfdValueType::SByte => 1,
            IfdValueType::Undefined => 1,
            IfdValueType::SShort => 2,
            IfdValueType::SLong => 4,
            IfdValueType::SRational => 4,
            IfdValueType::Float => 4,
            IfdValueType::Double => 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_ifd_json() {
        parse_json_file("src/ifd_tag_data/ifd.json")
    }

    #[test]
    fn parse_exif_json() {
        parse_json_file("src/ifd_tag_data/exif.json")
    }

    #[test]
    fn parse_gps_info_json() {
        parse_json_file("src/ifd_tag_data/gps_info.json")
    }

    fn parse_json_file(path: &str) {
        let data = fs::read_to_string(path).expect("Unable to read file");
        let json: Vec<IfdFieldDescriptor> =
            serde_json::from_str(&data).expect("JSON does not have correct format.");
        println!("{:#?}", json);
    }
}
