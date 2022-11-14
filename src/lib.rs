pub mod dng_reader;
pub mod ifd;
pub mod ifd_tag_data;
#[allow(unstable_name_collisions)]
pub mod yaml;

mod ifd_reader;
mod util;

use num_derive::FromPrimitive;

#[derive(FromPrimitive, Clone, Copy, Eq, PartialEq, Debug)]
pub enum FileType {
    Dng = 42,
    Dcp = 0x4352,
}
