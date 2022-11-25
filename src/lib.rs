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

use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, Eq, PartialEq, FromPrimitive, ToPrimitive)]
/// An enumeration over DNG / DCP files
pub enum FileType {
    /// A normal DNG / TIFF file
    Dng = 42,
    /// A DNG Camera Profile file. This should not contain image data
    Dcp = 0x4352,
}
impl FileType {
    pub fn get_extension(&self) -> &str {
        match self {
            FileType::Dng => "dng",
            FileType::Dcp => "dcp",
        }
    }
}
