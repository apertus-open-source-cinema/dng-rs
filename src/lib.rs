pub mod ifd;
pub mod ifd_tag_data;
mod unprocessed_ifd;
mod util;

use crate::ifd::Ifd;
use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdType};
use crate::unprocessed_ifd::UnprocessedIfd;
use crate::util::byte_order_reader::ByteOrderReader;
use derivative::Derivative;
use std::io;
use std::io::{Read, Seek, SeekFrom};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DngFile<R: Read + Seek> {
    #[derivative(Debug = "ignore")]
    reader: ByteOrderReader<R>,
    ifds: Vec<UnprocessedIfd>,
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
        if magic != 42 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid magic byte sequence (expected 42, got {}", magic),
            ));
        }

        let mut next_ifd_offset = reader.read_u32()?;
        let mut ifds = Vec::new();

        while next_ifd_offset != 0 {
            reader.seek(SeekFrom::Start(next_ifd_offset as u64))?;
            ifds.push(UnprocessedIfd::read(&mut reader)?);
            next_ifd_offset = reader.read_u32()?;
        }

        Ok(Self { reader, ifds })
    }
    pub fn read_ifd(&mut self) -> Result<Ifd, io::Error> {
        let mut metadata = Ifd::new(IfdType::Ifd);
        let ifd = &self.ifds[0];

        for entry in &ifd.entries {
            let tag = IfdTagDescriptor::from_number(entry.tag, IfdType::Ifd);
            metadata.insert(tag.clone(), entry.get_value(&mut self.reader, &tag)?);
        }
        Ok(metadata)
    }
}
