pub mod dng_reader;
pub mod dng_writer;
pub mod ifd;
pub mod ifd_reader;
pub mod ifd_tag_data;
pub mod util;

#[cfg(feature = "yaml")]
#[allow(unstable_name_collisions)]
pub mod yaml;

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
