use crate::exif::{tag_info_parser::ExifFieldDescriptor, ExifMetadata, ExifTag};
use std::{
    collections::HashMap,
    io::{self, Read, Seek, SeekFrom},
};

use self::{byte_order_reader::ByteOrderReader, ifd::Ifd};
use derivative::Derivative;

mod byte_order_reader;
mod ifd;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TiffFile<R: Read + Seek> {
    #[derivative(Debug = "ignore")]
    reader: ByteOrderReader<R>,
    ifds: Vec<Ifd>,
}
impl<R: Read + Seek> TiffFile<R> {
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
            ifds.push(Ifd::read(&mut reader)?);
            next_ifd_offset = reader.read_u32()?;
        }

        Ok(Self { reader, ifds })
    }

    pub fn get_exif_metadata(&mut self) -> Result<ExifMetadata, io::Error> {
        let mut metadata = HashMap::new();
        let idf = &self.ifds[0];

        for entry in &idf.entries {
            let tag = ExifTag::from_number(entry.tag);
            metadata.insert(tag, entry.get_value(&mut self.reader)?);
        }
        Ok(ExifMetadata(metadata))
    }
}
