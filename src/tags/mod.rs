use std::fmt::{Display, Formatter};

include!(concat!(env!("OUT_DIR"), "/ifd_data.rs"));

/// An enum indicating the context (and thus valid tags) of an IFD (normal / EXIF / GPSInfo)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfdType {
    Ifd,
    Exif,
    GpsInfo,
}
impl IfdType {
    pub fn get_namespace(&self) -> &[IfdFieldDescriptor] {
        match self {
            IfdType::Ifd => &ifd::ALL,
            IfdType::Exif => &exif::ALL,
            IfdType::GpsInfo => &gps_info::ALL,
        }
    }
    pub fn combined_namespace() -> impl Iterator<Item = &'static IfdFieldDescriptor> {
        ifd::ALL
            .iter()
            .chain(exif::ALL.iter())
            .chain(gps_info::ALL.iter())
    }
}
impl Default for IfdType {
    fn default() -> Self {
        Self::Ifd
    }
}

/// A data structure describing one specific Field (2byte key) that can appear in an IFD
/// Possible keys are defined in various specs, such ass the TIFF, TIFF-EP, DNG, ... spec.
#[derive(Debug, Copy, Clone, Eq)]
pub struct IfdFieldDescriptor {
    pub name: &'static str,
    pub tag: u16,
    pub dtype: &'static [IfdValueType],
    pub interpretation: IfdTypeInterpretation,
    pub count: IfdCount,
    pub description: &'static str,
    pub long_description: &'static str,
    pub references: &'static str,
}
impl IfdFieldDescriptor {
    pub fn as_maybe(&self) -> MaybeKnownIfdFieldDescriptor {
        MaybeKnownIfdFieldDescriptor::Known(*self)
    }
}
impl PartialEq for IfdFieldDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}
impl From<IfdFieldDescriptor> for MaybeKnownIfdFieldDescriptor {
    fn from(x: IfdFieldDescriptor) -> Self {
        MaybeKnownIfdFieldDescriptor::Known(x)
    }
}

/// An enum describing the amount of values we expect for a given Field
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum IfdCount {
    N,
    ConcreteValue(u32),
}

/// The high level interpretation of a field. (i.e. Enum variants, Bitfields, IFD-pointer, ...)
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum IfdTypeInterpretation {
    Default,

    Enumerated {
        values: &'static [(u32, &'static str)],
    },
    Bitflags {
        values: &'static [(u8, &'static str)],
    },

    CfaPattern,

    IfdOffset {
        ifd_type: IfdType,
    },

    /// This together with LENGTHS points to a buffer in the file that contains e.g. the actual
    /// image data.
    Offsets {
        /// contains the name of the corresponding LENGTHS tag.
        lengths: &'static IfdFieldDescriptor,
    },
    Lengths,

    /// this is a made-up (non spec) interpretation to flag that it might be smart to not dump the
    /// value but rather extract it to a file.
    Blob,
}

/// Represents a 2-byte IFD key, that is either known or unknown
#[derive(Debug, Clone, Eq, Copy)]
pub enum MaybeKnownIfdFieldDescriptor {
    Known(IfdFieldDescriptor),
    Unknown(u16),
}
impl MaybeKnownIfdFieldDescriptor {
    pub fn from_number(tag: u16, ifd_kind: IfdType) -> Self {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.tag == tag) {
            Self::Known(*description)
        } else {
            Self::Unknown(tag)
        }
    }
    pub fn from_name(name: &str, ifd_kind: IfdType) -> Result<Self, String> {
        if let Some(description) = ifd_kind.get_namespace().iter().find(|x| x.name == name) {
            Ok(Self::Known(*description))
        } else {
            Err(format!("No Tag named '{}' known", name))
        }
    }
    pub fn get_type_interpretation(&self) -> Option<&IfdTypeInterpretation> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(IfdFieldDescriptor { interpretation, .. }) => {
                Some(interpretation)
            }
            _ => None,
        }
    }
    pub fn get_known_value_type(&self) -> Option<&[IfdValueType]> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(known) => Some(known.dtype),
            MaybeKnownIfdFieldDescriptor::Unknown(_) => None,
        }
    }
    pub fn get_known_name(&self) -> Option<&str> {
        match self {
            Self::Known(descriptor) => Some(descriptor.name),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfdValueType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SByte,
    Undefined,
    SShort,
    SLong,
    SRational,
    Float,
    Double,
}
impl IfdValueType {
    pub fn from_u16(n: u16) -> Option<Self> {
        match n {
            1 => Some(Self::Byte),
            2 => Some(Self::Ascii),
            3 => Some(Self::Short),
            4 => Some(Self::Long),
            5 => Some(Self::Rational),
            6 => Some(Self::SByte),
            7 => Some(Self::Undefined),
            8 => Some(Self::SShort),
            9 => Some(Self::SLong),
            10 => Some(Self::SRational),
            11 => Some(Self::Float),
            12 => Some(Self::Double),
            _ => None,
        }
    }
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::Byte => 1,
            Self::Ascii => 2,
            Self::Short => 3,
            Self::Long => 4,
            Self::Rational => 5,
            Self::SByte => 6,
            Self::Undefined => 7,
            Self::SShort => 8,
            Self::SLong => 9,
            Self::SRational => 10,
            Self::Float => 11,
            Self::Double => 12,
        }
    }
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
