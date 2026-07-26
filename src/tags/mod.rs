use std::fmt::{Debug, Display, Formatter};

include!(concat!(env!("OUT_DIR"), "/ifd_data.rs"));

/// An enum indicating the context (and thus valid tags) of an IFD (normal/EXIF/GPSInfo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfdType {
    Ifd,
    Exif,
    GpsInfo,
}

impl IfdType {
    pub fn namespace(&self) -> &[IfdFieldDescriptor] {
        match self {
            IfdType::Ifd => &ifd::ALL,
            IfdType::Exif => &exif::ALL,
            IfdType::GpsInfo => &gps_info::ALL,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use namespace() instead"
    )]
    pub fn get_namespace(&self) -> &[IfdFieldDescriptor] {
        self.namespace()
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

/// A data structure describing one specific Field (2byte key) that can appear in an IFD.
///
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

/// An enum describing the amount of values we expect for a given Field.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum IfdCount {
    N,
    ConcreteValue(u32),
}

/// The high level interpretation of a field. (i.e. Enum variants, Bitfields, IFD-pointer, ...).
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
#[derive(Clone, Eq, Copy)]
pub enum MaybeKnownIfdFieldDescriptor {
    Known(IfdFieldDescriptor),
    Unknown(u16),
}

impl MaybeKnownIfdFieldDescriptor {
    pub fn from_number(tag: u16, ifd_kind: IfdType) -> Self {
        if let Some(description) = ifd_kind.namespace().iter().find(|x| x.tag == tag) {
            Self::Known(*description)
        } else {
            Self::Unknown(tag)
        }
    }

    pub fn from_name(name: &str, ifd_kind: IfdType) -> Result<Self, String> {
        if let Some(description) = ifd_kind.namespace().iter().find(|x| x.name == name) {
            Ok(Self::Known(*description))
        } else {
            Err(format!("No Tag named '{}' known", name))
        }
    }

    pub fn type_interpretation(&self) -> Option<&IfdTypeInterpretation> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(IfdFieldDescriptor { interpretation, .. }) => {
                Some(interpretation)
            }
            _ => None,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `type_interpretation()` instead"
    )]
    pub fn get_type_interpretation(&self) -> Option<&IfdTypeInterpretation> {
        self.type_interpretation()
    }

    pub fn known_value_type(&self) -> Option<&[IfdValueType]> {
        match self {
            MaybeKnownIfdFieldDescriptor::Known(known) => Some(known.dtype),
            MaybeKnownIfdFieldDescriptor::Unknown(_) => None,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `known_value_type()` instead"
    )]
    pub fn get_known_value_type(&self) -> Option<&[IfdValueType]> {
        self.known_value_type()
    }

    pub fn known_name(&self) -> Option<&str> {
        match self {
            Self::Known(descriptor) => Some(descriptor.name),
            Self::Unknown(_) => None,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `known_name()` instead"
    )]
    pub fn get_known_name(&self) -> Option<&str> {
        self.known_name()
    }

    #[deprecated(
        since = "1.6.0",
        note = "Use `u16::From<MaybeKnownIfdFieldDescriptor>` instead"
    )]
    pub fn numeric(&self) -> u16 {
        (*self).into()
    }
}

impl From<MaybeKnownIfdFieldDescriptor> for u16 {
    fn from(value: MaybeKnownIfdFieldDescriptor) -> Self {
        match value {
            MaybeKnownIfdFieldDescriptor::Known(descriptor) => descriptor.tag,
            MaybeKnownIfdFieldDescriptor::Unknown(tag) => tag,
        }
    }
}

impl Display for MaybeKnownIfdFieldDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            MaybeKnownIfdFieldDescriptor::Known(tag) => std::fmt::Display::fmt(&tag.name, f),
            MaybeKnownIfdFieldDescriptor::Unknown(tag) => {
                f.write_fmt(format_args!("{:#02X}", &tag))
            }
        }
    }
}

impl Debug for MaybeKnownIfdFieldDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self) // call method from Display
    }
}

impl PartialEq for MaybeKnownIfdFieldDescriptor {
    fn eq(&self, other: &Self) -> bool {
        u16::from(*self) == (*other).into()
    }
}

/// The data-type of an IFD value.
///
/// This does not include the fact that it is possible to have a list of every type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfdValueType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SignedByte,
    Undefined,
    SignedShort,
    SignedLong,
    SignedRational,
    Float,
    Double,
}

impl TryFrom<u16> for IfdValueType {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Byte),
            2 => Ok(Self::Ascii),
            3 => Ok(Self::Short),
            4 => Ok(Self::Long),
            5 => Ok(Self::Rational),
            6 => Ok(Self::SignedByte),
            7 => Ok(Self::Undefined),
            8 => Ok(Self::SignedShort),
            9 => Ok(Self::SignedLong),
            10 => Ok(Self::SignedRational),
            11 => Ok(Self::Float),
            12 => Ok(Self::Double),
            _ => Err(format!("Unknown value type: {}", value)),
        }
    }
}

impl From<IfdValueType> for u16 {
    fn from(value: IfdValueType) -> Self {
        match value {
            IfdValueType::Byte => 1,
            IfdValueType::Ascii => 2,
            IfdValueType::Short => 3,
            IfdValueType::Long => 4,
            IfdValueType::Rational => 5,
            IfdValueType::SignedByte => 6,
            IfdValueType::Undefined => 7,
            IfdValueType::SignedShort => 8,
            IfdValueType::SignedLong => 9,
            IfdValueType::SignedRational => 10,
            IfdValueType::Float => 11,
            IfdValueType::Double => 12,
        }
    }
}

impl IfdValueType {
    #[deprecated(since = "1.6.0", note = "Use `IfdValueType::TryFrom<u16>` instead")]
    pub fn from_u16(n: u16) -> Option<Self> {
        match n {
            1 => Some(Self::Byte),
            2 => Some(Self::Ascii),
            3 => Some(Self::Short),
            4 => Some(Self::Long),
            5 => Some(Self::Rational),
            6 => Some(Self::SignedByte),
            7 => Some(Self::Undefined),
            8 => Some(Self::SignedShort),
            9 => Some(Self::SignedLong),
            10 => Some(Self::SignedRational),
            11 => Some(Self::Float),
            12 => Some(Self::Double),
            _ => None,
        }
    }

    #[deprecated(since = "1.6.0", note = "Use `u16::From<IfdValueType>` instead")]
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::Byte => 1,
            Self::Ascii => 2,
            Self::Short => 3,
            Self::Long => 4,
            Self::Rational => 5,
            Self::SignedByte => 6,
            Self::Undefined => 7,
            Self::SignedShort => 8,
            Self::SignedLong => 9,
            Self::SignedRational => 10,
            Self::Float => 11,
            Self::Double => 12,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            IfdValueType::Byte => 1,
            IfdValueType::Ascii => 1,
            IfdValueType::Short => 2,
            IfdValueType::Long => 4,
            IfdValueType::Rational => 8,
            IfdValueType::SignedByte => 1,
            IfdValueType::Undefined => 1,
            IfdValueType::SignedShort => 2,
            IfdValueType::SignedLong => 4,
            IfdValueType::SignedRational => 8,
            IfdValueType::Float => 4,
            IfdValueType::Double => 8,
        }
    }

    #[deprecated(since = "1.6.0", note = "Use `size()` instead")]
    pub fn needed_bytes(&self) -> u32 {
        self.size() as _
    }
}
