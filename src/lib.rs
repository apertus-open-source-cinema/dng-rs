pub mod dng_reader;
pub mod dng_writer;
pub mod ifd;
pub mod ifd_tag_data;
#[allow(unstable_name_collisions)]
pub mod yaml;

mod ifd_reader;
mod util;

use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum FileType {
    Dng = 42,
    Dcp = 0x4352,
}
