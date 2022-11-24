mod dng_reader;
mod dng_writer;
pub mod ifd;
mod ifd_reader;
pub mod ifd_tags;
pub mod util;
#[cfg(feature = "yaml")]
#[allow(unstable_name_collisions)]
pub mod yaml;

pub use dng_reader::DngReader;
pub use dng_writer::DngWriter;

use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum FileType {
    Dng = 42,
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
