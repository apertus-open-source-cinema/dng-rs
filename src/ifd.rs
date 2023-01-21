use crate::tags::{IfdType, IfdValueType, MaybeKnownIfdFieldDescriptor};
use derivative::Derivative;
use itertools::Itertools;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Write;
use std::iter::once;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
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
    /// Inserts all entries from another IFD overwriting previously existing entries of the same tags
    pub fn insert_from_other(&mut self, other: Ifd) {
        for entry in other.entries {
            self.insert(entry.tag, entry.value)
        }
    }
    /// Inserts an entry into the IFD, overwriting a previously existing entry of the same tag
    pub fn insert(
        &mut self,
        tag: impl Into<MaybeKnownIfdFieldDescriptor>,
        value: impl Into<IfdValue>,
    ) {
        let tag = tag.into();
        self.entries.retain(|e| e.tag != tag);
        self.entries.push(IfdEntry::new(tag, value))
    }
    /// Returns an ifd entry by path. It will return None for the empty path because we cant produce
    /// a ref with an appropriate lifetime for `self`
    pub fn get_entry_by_path<'a>(&'a self, path: &'a IfdPath) -> Option<IfdEntryRef<'a>> {
        let path_vec = path.as_vec();
        let mut current = if let Some(IfdPathElement::Tag(tag)) = path_vec.get(0) {
            self.entries
                .iter()
                .find(|x| &x.tag == tag)
                .map(|x| &x.value)
        } else {
            return None;
        };
        for element in &path_vec[1..] {
            current = current.and_then(|x| x.index_with(element.clone()))
        }
        if let (Some(value), Some(tag)) = (&current, path.last_tag()) {
            Some(IfdEntryRef { value, path, tag })
        } else {
            None
        }
    }
    pub fn find_entry(&self, predicate: impl Fn(IfdEntryRef) -> bool + Clone) -> Option<IfdPath> {
        self.find_entry_with_start_path(Default::default(), predicate)
    }
    fn find_entry_with_start_path(
        &self,
        path: IfdPath,
        predicate: impl Fn(IfdEntryRef) -> bool + Clone,
    ) -> Option<IfdPath> {
        for entry in self.entries.iter() {
            let path = path.chain_tag(entry.tag);
            let entry_ref = entry.get_ref(&path);
            if predicate(entry_ref) {
                return Some(path.clone());
            }

            if let IfdValue::List(list) = &entry.value {
                for (i, v) in list.iter().enumerate() {
                    let path = path.chain_list_index(i as u16);
                    let entry = IfdEntryRef {
                        value: v,
                        path: &path,
                        tag: &entry.tag,
                    };
                    if predicate(entry) {
                        return Some(path);
                    }
                }
            } else if let IfdValue::Ifd(ifd) = &entry.value {
                let result = ifd.find_entry_with_start_path(path, predicate.clone());
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    pub fn get_type(&self) -> IfdType {
        self.ifd_type
    }
}

#[derive(Clone, Debug)]
/// A singular entry in an IFD (that does not know its path)
pub struct IfdEntry {
    pub value: IfdValue,
    pub tag: MaybeKnownIfdFieldDescriptor,
}
impl IfdEntry {
    pub fn new(
        tag: impl Into<MaybeKnownIfdFieldDescriptor>,
        value: impl Into<IfdValue>,
    ) -> IfdEntry {
        Self {
            tag: tag.into(),
            value: value.into(),
        }
    }
    pub fn get_ref<'a>(&'a self, path: &'a IfdPath) -> IfdEntryRef<'a> {
        IfdEntryRef {
            value: &self.value,
            path,
            tag: &self.tag,
        }
    }
}

#[derive(Clone, PartialEq, Default, Eq)]
/// The absolute path at which the entry is found in the IFD-tree
pub struct IfdPath(Vec<IfdPathElement>);
impl IfdPath {
    pub fn chain_path_element(&self, element: IfdPathElement) -> Self {
        Self(self.0.iter().cloned().chain(once(element)).collect())
    }
    pub fn chain_list_index(&self, n: u16) -> Self {
        self.chain_path_element(IfdPathElement::ListIndex(n))
    }
    pub fn chain_tag(&self, tag: impl Into<MaybeKnownIfdFieldDescriptor>) -> Self {
        self.chain_path_element(IfdPathElement::Tag(tag.into()))
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
    pub fn last_tag(&self) -> Option<&MaybeKnownIfdFieldDescriptor> {
        for elem in self.as_vec().iter().rev() {
            if let IfdPathElement::Tag(tag) = elem {
                return Some(tag);
            }
        }
        None
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

#[derive(Clone, Copy, Debug)]
/// A ref to a singular entry in an IFD
pub struct IfdEntryRef<'a> {
    pub value: &'a IfdValue,
    pub tag: &'a MaybeKnownIfdFieldDescriptor,
    pub path: &'a IfdPath,
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

    List(Vec<IfdValue>),
    Ifd(Ifd),

    /// this value is not produced by the reader but rather there to insert image data into the writer
    /// The contents will be written somewhere in the file and the tag will be replaced by a [IfdValue::Long]
    /// pointing to that data. You are responsible for setting the corresponding length tag yourself.
    Offsets(#[derivative(Debug = "ignore")] Arc<dyn Offsets + Send + Sync>),
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
            IfdValue::List(list) => list[0].get_ifd_value_type(),

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
            Self::List(list) => Box::new(list.iter()) as Box<dyn Iterator<Item = &IfdValue>>,
            _ => Box::new(once(self)) as Box<dyn Iterator<Item = &IfdValue>>,
        }
    }
    pub fn index_with(&self, index: IfdPathElement) -> Option<&Self> {
        match (&self, index) {
            (Self::Ifd(ifd), IfdPathElement::Tag(tag)) => {
                ifd.entries.iter().find(|x| x.tag == tag).map(|x| &x.value)
            }
            (Self::List(list), IfdPathElement::ListIndex(index)) => list.get(index as usize),
            _ => None,
        }
    }
}

macro_rules! implement_from {
    ($rust_type:ty, $variant:expr) => {
        impl From<$rust_type> for IfdValue {
            fn from(x: $rust_type) -> Self {
                $variant(x)
            }
        }
    };
}
implement_from!(u8, IfdValue::Byte);
implement_from!(String, IfdValue::Ascii);
implement_from!(u16, IfdValue::Short);
implement_from!(u32, IfdValue::Long);
implement_from!(i8, IfdValue::SByte);
implement_from!(i16, IfdValue::SShort);
implement_from!(i32, IfdValue::SLong);

impl From<&str> for IfdValue {
    fn from(x: &str) -> Self {
        IfdValue::Ascii(x.to_string())
    }
}

impl<T: Into<IfdValue> + Clone> From<&[T]> for IfdValue {
    fn from(x: &[T]) -> Self {
        IfdValue::List(x.iter().cloned().map(|x| x.into()).collect())
    }
}
impl<T: Into<IfdValue> + Clone, const N: usize> From<[T; N]> for IfdValue {
    fn from(x: [T; N]) -> Self {
        IfdValue::List(x.iter().cloned().map(|x| x.into()).collect())
    }
}
impl<T: Into<IfdValue> + Clone, const N: usize> From<&[T; N]> for IfdValue {
    fn from(x: &[T; N]) -> Self {
        IfdValue::List(x.iter().cloned().map(|x| x.into()).collect())
    }
}

pub trait Offsets {
    fn size(&self) -> u32;
    fn write(&self, writer: &mut dyn Write) -> io::Result<()>;
}
impl<T: Deref<Target = [u8]>> Offsets for T {
    fn size(&self) -> u32 {
        self.len() as u32
    }
    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_all(self)
    }
}
