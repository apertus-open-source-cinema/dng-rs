use super::{ExifValue, ExifValueType};
use core::fmt;
use serde::{de, Deserialize, Deserializer};
use serde_hex::{SerHex, StrictPfx};
use std::{collections::HashMap, hash::Hash};

#[derive(Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct ExifFieldDescriptor {
    pub name: String,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub tag: u16,
    pub dtype: Vec<ExifValueType>,
    pub interpretation: MaybeExifTypeInterpretation,
    pub count: ExifCount,
    pub description: String,
    pub long_description: String,
    pub references: String,
}
impl Hash for ExifFieldDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.tag)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ExifCount {
    N,
    ConcreteValue(u32),
}
impl<'de> Deserialize<'de> for ExifCount {
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
pub enum MaybeExifTypeInterpretation {
    Known(ExifTypeInterpretation),
    Other(serde_json::Value),
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "UPPERCASE")]
pub enum ExifTypeInterpretation {
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
}
impl ExifTypeInterpretation {
    pub fn pretty_yaml_value(
        &self,
        value: &ExifValue,
        writer: &mut dyn fmt::Write,
    ) -> Result<(), fmt::Error> {
        match self {
            ExifTypeInterpretation::Enumerated { values } => {
                if let Some(v) = value.as_u32() {
                    if let Some(v) = values.get(&v) {
                        writer.write_str(v)?;
                    } else {
                        writer.write_str("UNKNOWN (")?;
                        value.pretty_yaml_plain(writer)?;
                        writer.write_str(")")?
                    }
                } else {
                    eprintln!(
                        "value {:?} couldn't be made into number (this is illegal for enums",
                        value
                    );
                    value.pretty_yaml_plain(writer)?;
                }
            }
            _ => value.pretty_yaml_plain(writer)?,
        };
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_exif_json() {
        parse_json_file("src/exif/exif.json")
    }

    #[test]
    fn parse_exif_ifd_json() {
        parse_json_file("src/exif/exif_ifd.json")
    }

    #[test]
    fn parse_gps_info_ifd_json() {
        parse_json_file("src/exif/gps_info_ifd.json")
    }

    fn parse_json_file(path: &str) {
        let data = fs::read_to_string(path).expect("Unable to read file");
        let json: Vec<ExifFieldDescriptor> =
            serde_json::from_str(&data).expect("JSON does not have correct format.");
        println!("{:#?}", json);
    }
}
