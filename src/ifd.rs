use crate::tags::{IfdType, IfdValueType, MaybeKnownIfdFieldDescriptor};
use derivative::Derivative;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Write;
use std::iter::once;
use std::mem;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
/// Represents an IFD-Tree that was read/can be written.
pub struct Ifd {
    pub(crate) entries: Vec<IfdEntry>,
    pub(crate) ifd_type: IfdType,
}

impl Ifd {
    /// Create a new Image File Directory (IFD) of the given type.
    pub fn new(ifd_type: IfdType) -> Self {
        Self {
            entries: Vec::new(),
            ifd_type,
        }
    }

    /// Inserts all entries from another IFD overwriting previously existing entries of the same tags.
    pub fn insert_from_other(&mut self, other: Ifd) {
        for entry in other.entries {
            self.insert(entry.tag, entry.value)
        }
    }

    /// Inserts an entry into the IFD, overwriting a previously existing entry of the same tag.
    pub fn insert(
        &mut self,
        tag: impl Into<MaybeKnownIfdFieldDescriptor>,
        value: impl Into<IfdValue>,
    ) {
        let tag = tag.into();
        self.entries.retain(|e| e.tag != tag);
        self.entries.push(IfdEntry::new(tag, value))
    }

    /// Inserts an entry into the IFD at the given path, overwriting a previously existing entry
    /// there.
    ///
    /// Returns the previous value if it existed, does nothing otherwise.
    pub fn replace_by_path(
        &mut self,
        path: &IfdPath,
        value: impl Into<IfdValue>,
    ) -> Option<IfdValue> {
        let path_vec = path.as_ref();
        let mut current = if let Some(IfdPathElement::Tag(tag)) = path_vec.first() {
            self.entries
                .iter_mut()
                .find(|x| &x.tag == tag)
                .map(|x| &mut x.value)
        } else {
            return None;
        };
        for element in &path_vec[1..] {
            current = current.and_then(|x| x.get_mut(element.clone()));
        }
        if let Some(v) = current {
            let mut value = value.into();
            mem::swap(v, &mut value);
            Some(value)
        } else {
            None
        }
    }

    /// Returns an IFD entry by path.
    ///
    /// It will return `None` for the empty path because we cant produce a ref with an appropriate
    /// lifetime for `self`.
    pub fn entry_by_path<'a>(&'a self, path: &'a IfdPath) -> Option<IfdEntryRef<'a>> {
        let path_vec = path.as_ref();
        let mut current = if let Some(IfdPathElement::Tag(tag)) = path_vec.first() {
            self.entries
                .iter()
                .find(|x| &x.tag == tag)
                .map(|x| &x.value)
        } else {
            return None;
        };
        for element in &path_vec[1..] {
            current = current.and_then(|x| x.get(element.clone()))
        }
        if let (Some(value), Some(tag)) = (&current, path.last_tag()) {
            Some(IfdEntryRef { value, path, tag })
        } else {
            None
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use entry_by_path() instead"
    )]
    pub fn get_entry_by_path<'a>(&'a self, path: &'a IfdPath) -> Option<IfdEntryRef<'a>> {
        self.entry_by_path(path)
    }

    /// Return the first entry satisfying the given predicate.
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
            let entry_ref = entry.build_ref(&path);
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

    /// Find all entries satisfying the given predicate.
    pub fn find_entries(&self, predicate: impl Fn(IfdEntryRef) -> bool + Clone) -> Vec<IfdPath> {
        self.find_entries_with_start_path(Default::default(), predicate)
    }

    fn find_entries_with_start_path(
        &self,
        path: IfdPath,
        predicate: impl Fn(IfdEntryRef) -> bool + Clone,
    ) -> Vec<IfdPath> {
        let mut entries = Vec::new();
        for entry in self.entries.iter() {
            let path = path.chain_tag(entry.tag);
            let entry_ref = entry.build_ref(&path);
            if predicate(entry_ref) {
                entries.push(path.clone());
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
                        entries.push(path);
                    }
                }
            } else if let IfdValue::Ifd(ifd) = &entry.value {
                entries.extend(ifd.find_entries_with_start_path(path, predicate.clone()));
            }
        }
        entries
    }

    pub fn ifd_type(&self) -> IfdType {
        self.ifd_type
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `ifd_type()` instead"
    )]
    pub fn get_type(&self) -> IfdType {
        self.ifd_type()
    }

    pub fn entries(&self) -> &[IfdEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug)]
/// A singular entry in an IFD (that does not know its path).
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

    pub fn build_ref<'a>(&'a self, path: &'a IfdPath) -> IfdEntryRef<'a> {
        IfdEntryRef {
            value: &self.value,
            path,
            tag: &self.tag,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `build_ref()` instead"
    )]
    pub fn get_ref<'a>(&'a self, path: &'a IfdPath) -> IfdEntryRef<'a> {
        self.build_ref(path)
    }
}

#[derive(Clone, PartialEq, Default, Eq)]
/// The absolute path at which the entry is found in the IFD-tree.
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
        self.0
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(separator)
    }

    #[deprecated(since = "1.6.0", note = "Use `as_ref()` instead")]
    pub fn as_vec(&self) -> &Vec<IfdPathElement> {
        &self.0
    }

    pub fn with_last_tag_replaced(&self, replacement: MaybeKnownIfdFieldDescriptor) -> Self {
        let mut new_vec = self.as_ref().clone();
        for elem in new_vec.iter_mut().rev() {
            if matches!(elem, IfdPathElement::Tag(_)) {
                *elem = IfdPathElement::Tag(replacement);
                break;
            }
        }

        Self(new_vec)
    }

    pub fn last_tag(&self) -> Option<&MaybeKnownIfdFieldDescriptor> {
        for elem in self.as_ref().iter().rev() {
            if let IfdPathElement::Tag(tag) = elem {
                return Some(tag);
            }
        }
        None
    }
}

impl AsRef<Vec<IfdPathElement>> for IfdPath {
    fn as_ref(&self) -> &Vec<IfdPathElement> {
        &self.0
    }
}

impl Debug for IfdPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string_with_separator("."))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A segment of an [`IfdPath`].
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
/// A ref to a singular entry in an IFD.
pub struct IfdEntryRef<'a> {
    pub value: &'a IfdValue,
    pub tag: &'a MaybeKnownIfdFieldDescriptor,
    pub path: &'a IfdPath,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
/// A singular Value in an IFD (that doesn't know its tag or path).
pub enum IfdValue {
    Byte(u8),
    Ascii(String),
    Short(u16),
    Long(u32),
    Rational(u32, u32),
    SignedByte(i8),
    Undefined(u8),
    SignedShort(i16),
    SignedLong(i32),
    SignedRational(i32, i32),
    Float(f32),
    Double(f64),

    List(Vec<IfdValue>),
    Ifd(Ifd),

    /// This value is not produced by the reader but rather there to insert image data into the writer.
    /// The contents will be written somewhere in the file and the tag will be replaced by a [`IfdValue::Long`]
    /// pointing to that data.
    ///
    /// You are responsible for setting the corresponding length tag yourself.
    Offsets(#[derivative(Debug = "ignore")] Arc<dyn Offsets + Send + Sync>),
}

impl IfdValue {
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            IfdValue::Byte(x) => Some(*x as u32),
            IfdValue::Short(x) => Some(*x as u32),
            IfdValue::Long(x) => Some(*x),
            IfdValue::SignedByte(x) => Some(*x as u32),
            IfdValue::Undefined(x) => Some(*x as u32),
            IfdValue::SignedShort(x) => Some(*x as u32),
            IfdValue::SignedLong(x) => Some(*x as u32),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            IfdValue::SignedRational(x, y) => Some(*x as f64 / *y as f64),
            IfdValue::Rational(x, y) => Some(*x as f64 / *y as f64),
            IfdValue::Float(f) => Some(*f as f64),
            IfdValue::Double(f) => Some(*f),
            _ => self.as_u32().map(|x| x as f64),
        }
    }

    pub fn ifd_value_type(&self) -> IfdValueType {
        match self {
            IfdValue::Byte(_) => IfdValueType::Byte,
            IfdValue::Ascii(_) => IfdValueType::Ascii,
            IfdValue::Short(_) => IfdValueType::Short,
            IfdValue::Long(_) => IfdValueType::Long,
            IfdValue::Rational(_, _) => IfdValueType::Rational,
            IfdValue::SignedByte(_) => IfdValueType::SignedByte,
            IfdValue::Undefined(_) => IfdValueType::Undefined,
            IfdValue::SignedShort(_) => IfdValueType::SignedShort,
            IfdValue::SignedLong(_) => IfdValueType::SignedLong,
            IfdValue::SignedRational(_, _) => IfdValueType::SignedRational,
            IfdValue::Float(_) => IfdValueType::Float,
            IfdValue::Double(_) => IfdValueType::Double,
            IfdValue::List(list) => {
                let ty = list[0].ifd_value_type();
                for elem in list {
                    assert_eq!(elem.ifd_value_type(), ty)
                }
                ty
            }

            // these two are made into a pointer to the actual data
            IfdValue::Ifd(_) => IfdValueType::Long,
            IfdValue::Offsets(_) => IfdValueType::Long,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `ifd_value_type()` instead"
    )]
    pub fn get_ifd_value_type(&self) -> IfdValueType {
        self.ifd_value_type()
    }

    pub fn count(&self) -> u32 {
        match self {
            IfdValue::List(list) => list.len() as u32,
            IfdValue::Ascii(str) => str.len() as u32 + 1,
            _ => 1,
        }
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `count()` instead"
    )]
    pub fn get_count(&self) -> u32 {
        self.count()
    }

    pub fn as_list(&self) -> impl Iterator<Item = &IfdValue> {
        match self {
            Self::List(list) => Box::new(list.iter()) as Box<dyn Iterator<Item = &IfdValue>>,
            _ => Box::new(once(self)) as Box<dyn Iterator<Item = &IfdValue>>,
        }
    }

    pub fn get(&self, index: IfdPathElement) -> Option<&Self> {
        match (&self, index) {
            (Self::Ifd(ifd), IfdPathElement::Tag(tag)) => {
                ifd.entries.iter().find(|x| x.tag == tag).map(|x| &x.value)
            }
            (Self::List(list), IfdPathElement::ListIndex(index)) => list.get(index as usize),
            _ => None,
        }
    }

    #[deprecated(since = "1.6.0", note = "Use `get()` instead")]
    pub fn index_with(&self, index: IfdPathElement) -> Option<&Self> {
        self.get(index)
    }

    pub fn get_mut(&mut self, index: IfdPathElement) -> Option<&mut Self> {
        match (self, index) {
            (Self::Ifd(ifd), IfdPathElement::Tag(tag)) => ifd
                .entries
                .iter_mut()
                .find(|x| x.tag == tag)
                .map(|x| &mut x.value),
            (Self::List(list), IfdPathElement::ListIndex(index)) => list.get_mut(index as usize),
            _ => None,
        }
    }

    #[deprecated(since = "1.6.0", note = "Use `get_mut()` instead")]
    pub fn index_with_mut(&mut self, index: IfdPathElement) -> Option<&mut Self> {
        self.get_mut(index)
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
implement_from!(i8, IfdValue::SignedByte);
implement_from!(i16, IfdValue::SignedShort);
implement_from!(i32, IfdValue::SignedLong);

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
