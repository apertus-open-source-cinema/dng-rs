pub mod ifd;
pub mod ifd_tag_data;
mod unprocessed_ifd;
mod util;
#[allow(unstable_name_collisions)]
pub mod yaml;

use crate::ifd::{Ifd, IfdEntry, IfdPath};
use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdType, IfdTypeInterpretation};
use crate::unprocessed_ifd::UnprocessedIfd;
use crate::util::byte_order_reader::ByteOrderReader;
use derivative::Derivative;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::io;
use std::io::{Read, Seek, SeekFrom};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DngFile<R: Read + Seek> {
    file_type: FileType,
    #[derivative(Debug = "ignore")]
    reader: RefCell<ByteOrderReader<R>>,
    ifds: Vec<Ifd>,
}
impl<R: Read + Seek> DngFile<R> {
    pub fn new(mut reader: R) -> Result<Self, io::Error> {
        // the first two bytes set the byte order
        let mut header = vec![0u8; 2];
        reader.read(&mut header)?;
        let is_little_endian = match (header[0], header[1]) {
            (0x49, 0x49) => Ok(true),
            (0x4D, 0x4D) => Ok(false),
            (_, _) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid header bytes",
            )),
        }?;
        let mut reader = ByteOrderReader::new(reader, is_little_endian);
        let magic = reader.read_u16()?;
        let file_type = FileType::from_u16(magic).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid magic byte sequence (expected 42, got {}", magic),
        ))?;

        let mut next_ifd_offset = reader.read_u32()?;
        let mut unprocessed_ifds = Vec::new();

        while next_ifd_offset != 0 {
            reader.seek(SeekFrom::Start(next_ifd_offset as u64))?;
            unprocessed_ifds.push(UnprocessedIfd::read(&mut reader)?);
            next_ifd_offset = reader.read_u32()?;
        }
        let ifds: Result<Vec<_>, _> = unprocessed_ifds
            .iter()
            .map(|ifd| ifd.process(IfdType::Ifd, &IfdPath::default(), &mut reader))
            .collect();

        Ok(Self {
            reader: RefCell::new(reader),
            ifds: ifds?,
            file_type,
        })
    }
    pub fn get_ifd0(&self) -> &Ifd {
        &self.ifds[0]
    }
    pub fn get_entry_by_path(&self, path: &IfdPath) -> Option<IfdEntry> {
        self.ifds
            .iter()
            .flat_map(|ifd| ifd.flat_entries())
            .find(|entry| &entry.path == path)
            .cloned()
    }
    pub fn needed_buffer_size_for_blob(&self, entry: &IfdEntry) -> Result<IfdEntry, io::Error> {
        if let Some(IfdTypeInterpretation::Offsets { lengths }) =
            entry.tag.get_known_type_interpretation()
        {
            let lengths_paths = entry.path.with_last_tag_replaced(
                IfdTagDescriptor::from_name(lengths)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            );
            let lengths_value = self.get_entry_by_path(&lengths_paths);
            if let Some(entry) = lengths_value {
                Ok(entry)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "length tag {lengths_paths:?} for {:?} not found",
                        entry.path
                    ),
                ))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("entry {entry:?} is not of type offsets"),
            ))
        }
    }
    pub fn read_blob_to_buffer(
        &self,
        entry: &IfdEntry,
        buffer: &mut [u8],
    ) -> Result<(), io::Error> {
        let buffer_size = self
            .needed_buffer_size_for_blob(entry)
            .unwrap()
            .value
            .as_u32()
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("entry {entry:?} cant be read into buffer. it is not a single OFFSETS"),
            ))? as usize;
        if buffer_size != buffer.len() {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "buffer has wrong size (expected {buffer_size} found {}",
                    buffer.len()
                ),
            ))
        } else {
            let mut reader = self.reader.borrow_mut();
            reader.seek(SeekFrom::Start(entry.value.as_u32().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("entry {entry:?} cant be read into buffer. it is not a single OFFSETS"),
            ))? as u64))?;
            reader.read_exact(buffer)
        }
    }
}

#[derive(FromPrimitive, Clone, Copy, Eq, PartialEq, Debug)]
pub enum FileType {
    Dng = 42,
    Dcp = 0x4352,
}
