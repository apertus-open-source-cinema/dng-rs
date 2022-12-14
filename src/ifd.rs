use crate::tags::{IfdType, IfdValueType, MaybeKnownIfdFieldDescriptor};
use derivative::Derivative;
use itertools::Itertools;
use std::fmt::{Debug, Display, Formatter};
use std::iter;
use std::iter::once;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
/// Represents an IFD-Tree that was read / can be written
pub struct Ifd {
    pub(crate) entries: Vec<IfdEntry>,
    pub(crate) ifd_type: IfdType,
}
impl Ifd {
    pub fn new(ifd_type: IfdType) -> Self {
        Self {
            entries: Vec::new(),
            ifd_type,
        }
    }
    pub fn insert(&mut self, value: IfdEntry) {
        self.entries.push(value)
    }
    pub fn get_entry_by_tag(&self, tag: MaybeKnownIfdFieldDescriptor) -> Option<&IfdEntry> {
        self.entries.iter().find(|x| x.tag == tag)
    }
    pub fn flat_entries(&self) -> impl Iterator<Item = &IfdEntry> {
        self.entries
            .iter()
            .flat_map(|entry| once(entry).chain(entry.value.iter_children()))
    }
    pub fn get_type(&self) -> IfdType {
        self.ifd_type
    }
}

#[derive(Clone, PartialEq, Default, Eq)]
/// The absolute path at which the entry is found in the IFD-tree
pub struct IfdPath(Vec<IfdPathElement>);
impl IfdPath {
    pub fn chain_list_index(&self, n: u16) -> Self {
        Self(
            self.0
                .iter()
                .cloned()
                .chain(once(IfdPathElement::ListIndex(n)))
                .collect(),
        )
    }
    pub fn chain_tag(&self, tag: MaybeKnownIfdFieldDescriptor) -> Self {
        Self(
            self.0
                .iter()
                .cloned()
                .chain(once(IfdPathElement::Tag(tag)))
                .collect(),
        )
    }
    pub fn parent(&self) -> Self {
        let mut new = self.0.clone();
        new.pop();
        Self(new)
    }
    pub fn string_with_separator(&self, separator: &str) -> String {
        self.0.iter().map(|x| x.to_string()).join(separator)
    }
    pub fn as_vec(&self) -> &Vec<IfdPathElement> {
        &self.0
    }
    pub fn with_last_tag_replaced(&self, replacement: MaybeKnownIfdFieldDescriptor) -> Self {
        let mut new_vec = self.as_vec().clone();
        for elem in new_vec.iter_mut().rev() {
            if matches!(elem, IfdPathElement::Tag(_)) {
                *elem = IfdPathElement::Tag(replacement);
                break;
            }
        }
        Self(new_vec)
    }
}
impl Debug for IfdPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string_with_separator("."))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A segment of an [IfdPath]
pub enum IfdPathElement {
    Tag(MaybeKnownIfdFieldDescriptor),
    ListIndex(u16),
}
impl Display for IfdPathElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IfdPathElement::Tag(tag) => f.write_fmt(format_args!("{tag}")),
            IfdPathElement::ListIndex(n) => f.write_fmt(format_args!("{n}")),
        }
    }
}

#[derive(Clone, Debug)]
/// A singular entry in an IFD (that knows its tag and path)
pub struct IfdEntry {
    pub value: IfdValue,
    pub path: IfdPath,
    pub tag: MaybeKnownIfdFieldDescriptor,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
/// A singular Value in an IFD (that doesn't know its tag or path)
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

    List(Vec<IfdEntry>),
    Ifd(Ifd),

    /// this value is not produced by the reader but rather there to insert image data into the writer
    /// The contents will be written somewhere in the file and the tag will be replaced by a [IfdValue::Long]
    /// pointing to that data. You are responsible for setting the corresponding length tag yourself.
    Offsets(#[derivative(Debug = "ignore")] Arc<dyn Deref<Target = [u8]>>),
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
    pub fn iter_children<'a>(&'a self) -> impl Iterator<Item = &'a IfdEntry> + 'a {
        match self {
            IfdValue::List(list) => Box::new(
                list.iter()
                    .flat_map(|entry| once(entry).chain(entry.value.iter_children())),
            ) as Box<dyn Iterator<Item = &'a IfdEntry> + 'a>,
            IfdValue::Ifd(ifd) => {
                Box::new(ifd.flat_entries()) as Box<dyn Iterator<Item = &'a IfdEntry> + 'a>
            }
            _ => Box::new(iter::empty()) as Box<dyn Iterator<Item = &'a IfdEntry> + 'a>,
        }
    }
    pub fn get_ifd_value_type(&self) -> IfdValueType {
        match self {
            IfdValue::Byte(_) => IfdValueType::Byte,
            IfdValue::Ascii(_) => IfdValueType::Ascii,
            IfdValue::Short(_) => IfdValueType::Short,
            IfdValue::Long(_) => IfdValueType::Long,
            IfdValue::Rational(_, _) => IfdValueType::Rational,
            IfdValue::SByte(_) => IfdValueType::SByte,
            IfdValue::Undefined(_) => IfdValueType::Undefined,
            IfdValue::SShort(_) => IfdValueType::SShort,
            IfdValue::SLong(_) => IfdValueType::SLong,
            IfdValue::SRational(_, _) => IfdValueType::SRational,
            IfdValue::Float(_) => IfdValueType::Float,
            IfdValue::Double(_) => IfdValueType::Double,
            IfdValue::List(list) => list[0].value.get_ifd_value_type(),

            // these two are made into a pointer to the actual data
            IfdValue::Ifd(_) => IfdValueType::Long,
            IfdValue::Offsets(_) => IfdValueType::Long,
        }
    }
    pub fn get_count(&self) -> u32 {
        match self {
            IfdValue::List(list) => list.len() as u32,
            IfdValue::Ascii(str) => str.len() as u32 + 1,
            _ => 1,
        }
    }
    pub fn as_list(&self) -> impl Iterator<Item = &IfdValue> {
        match self {
            Self::List(list) => {
                Box::new(list.iter().map(|x| &x.value)) as Box<dyn Iterator<Item = &IfdValue>>
            }
            _ => Box::new(once(self)) as Box<dyn Iterator<Item = &IfdValue>>,
        }
    }
}
