use num_derive::{FromPrimitive, ToPrimitive};
use once_cell::sync::Lazy;
use serde::{de, Deserialize, Deserializer, Serialize};
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

/// An enum indicating the context (and thus valid tags) of an IFD (normal / EXIF / GPSInfo)
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

/// A data structure describing one specific Field (2byte key) that can appear in an IFD
/// Possible keys are defined in various specs, such ass the TIFF, TIFF-EP, DNG, ... spec.
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

/// An enum describing the amount of values we expect for a given Field
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

/// The maybe not accurately parsed [IfdTypeInterpretation] of a field
#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum MaybeIfdTypeInterpretation {
    Known(IfdTypeInterpretation),
    Other(serde_json::Value),
}

/// The high level interpretation of a field. (i.e. Enum variants, Bitfields, IFD-pointer, ...)
#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "UPPERCASE")]
pub enum IfdTypeInterpretation {
    Default,

    Enumerated {
        #[serde(deserialize_with = "deserialize_enumerated_values")]
        values: HashMap<u32, String>,
    },
    Bitflags {
        #[serde(deserialize_with = "deserialize_bitflags_values")]
        values: HashMap<u8, String>,
    },

    CfaPattern,

    IfdOffset {
        ifd_type: IfdType,
    },

    /// This together with LENGTHS points to a buffer in the file that contains e.g. the actual
    /// image data.
    Offsets {
        /// contains the name of the corresponding LENGTHS tag.
        lengths: String,
    },
    Lengths,

    /// this is a made-up interpretation to flag, that it might be smart to not dump the value but
    /// rather extract it to a file.
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

/// Represents a 2-byte IFD key, that is either known or unknown
#[derive(Debug, Clone, Eq)]
pub enum MaybeKnownIfdFieldDescriptor {
    Known(IfdFieldDescriptor),
    Unknown(u16),
}
impl MaybeKnownIfdFieldDescriptor {
    pub fn from_number(tag: u16, ifd_kind: IfdType) -> Self {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.tag == tag) {
            Self::Known(description.clone())
        } else {
            Self::Unknown(tag)
        }
    }
    pub fn from_name(name: &str, ifd_kind: IfdType) -> Result<Self, String> {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.name == name) {
            Ok(Self::Known(description.clone()))
        } else {
            Err(format!("No Tag named '{}' known", name))
        }
    }
    pub fn get_known_type_interpretation(&self) -> Option<&IfdTypeInterpretation> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(IfdFieldDescriptor {
                interpretation: MaybeIfdTypeInterpretation::Known(interpretation),
                ..
            }) => Some(interpretation),
            _ => None,
        }
    }
    pub fn get_known_value_type(&self) -> Option<&Vec<IfdValueType>> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(known) => Some(&known.dtype),
            MaybeKnownIfdFieldDescriptor::Unknown(_) => None,
        }
    }
    pub fn get_known_name(&self) -> Option<&str> {
        match self {
            Self::Known(descriptor) => Some(&descriptor.name),
            Self::Unknown(_) => None,
        }
    }
    pub fn numeric(&self) -> u16 {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(descriptor) => descriptor.tag,
            MaybeKnownIfdFieldDescriptor::Unknown(tag) => *tag,
        }
    }
}
impl Display for MaybeKnownIfdFieldDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            MaybeKnownIfdFieldDescriptor::Known(tag) => tag.name.fmt(f),
            MaybeKnownIfdFieldDescriptor::Unknown(tag) => {
                f.write_fmt(format_args!("{:#02X}", &tag))
            }
        }
    }
}
impl PartialEq for MaybeKnownIfdFieldDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.numeric() == other.numeric()
    }
}

/// The data-type of an IFD value
/// This does not include the fact that it is possible to have a list of every type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, FromPrimitive, ToPrimitive)]
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
            IfdValueType::SRational => 8,
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
        parse_json_file("src/ifd_tags/ifd.json")
    }

    #[test]
    fn parse_exif_json() {
        parse_json_file("src/ifd_tags/exif.json")
    }

    #[test]
    fn parse_gps_info_json() {
        parse_json_file("src/ifd_tags/gps_info.json")
    }

    fn parse_json_file(path: &str) {
        let data = fs::read_to_string(path).expect("Unable to read file");
        let json: Vec<IfdFieldDescriptor> =
            serde_json::from_str(&data).expect("JSON does not have correct format.");
        println!("{:#?}", json);
    }
}