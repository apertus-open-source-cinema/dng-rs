use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdType};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Ifd {
    pub entries: Vec<(IfdTagDescriptor, IfdValue)>,
    #[serde(skip)]
    pub ifd_type: IfdType,
}
impl Ifd {
    pub fn new(ifd_type: IfdType) -> Self {
        Self {
            entries: Vec::new(),
            ifd_type,
        }
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
