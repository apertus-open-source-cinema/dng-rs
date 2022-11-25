//! A pure rust library for reading / writing DNG files providing access to the raw data in a zero-copy friendly way.
//!
//! It also contains code for reading / writing a human-readable YAML representation of DNG tags / the IFD structure.
//! The library also supports interacting with DCP (Dng Camera Profile) files, but that is on a best-effort basis since I
//! was unable to find official documentation on that.
//!
//! To get started, see the basic examples of [DngReader] or [DngWriter] or the more advanced usage of the library in
//! the cli tools in `src/bin/`.

mod byte_order_rw;
mod dng_reader;
mod dng_writer;
mod ifd_reader;

/// Datastructures for representing an IFD of a read / to write DNG / DCP
pub mod ifd;
/// Datastructures and Data describing the interpretation of IFD / EXIF tags
pub mod ifd_tags;
/// Code for reading / writing a human readable text representation of IFDs
#[cfg(feature = "yaml")]
#[allow(unstable_name_collisions)]
pub mod yaml;

pub use dng_reader::DngReader;
pub use dng_writer::DngWriter;

/// An enumeration over DNG / DCP files
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileType {
    /// A normal DNG / TIFF file
    Dng,
    /// A DNG Camera Profile file. This should not contain image data
    Dcp,
}
impl FileType {
    pub fn from_magic(magic: u16) -> Option<Self> {
        match magic {
            42 => Some(Self::Dng),
            0x4352 => Some(Self::Dcp),
            _ => None,
        }
    }
    pub fn magic(&self) -> u16 {
        match self {
            FileType::Dng => 42,
            FileType::Dcp => 0x4352,
        }
    }
    pub fn extension(&self) -> &str {
        match self {
            FileType::Dng => "dng",
            FileType::Dcp => "dcp",
        }
    }
}
