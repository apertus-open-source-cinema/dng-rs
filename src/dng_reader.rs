use crate::byte_order_rw::ByteOrderReader;
use crate::ifd::{Ifd, IfdEntryRef, IfdPath};
use crate::ifd_reader::IfdReader;
use crate::tags::{ifd, IfdType, IfdTypeInterpretation};
use crate::FileType;
use derivative::Derivative;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

/// The error-type produced by [DngReader]
#[derive(Error, Debug)]
pub enum DngReaderError {
    IoError(io::Error),
    FormatError(String),
    Other(String),
}
impl From<io::Error> for DngReaderError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}
impl Display for DngReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DngReaderError::IoError(e) => f.write_fmt(format_args!("IoError: '{:?}'", e)),
            DngReaderError::FormatError(e) => f.write_fmt(format_args!("FormatError: '{}'", e)),
            DngReaderError::Other(e) => f.write_fmt(format_args!("Other: '{}'", e)),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
/// The main entrypoint for reading DNG/DCP files.
///
/// # Examples
///
/// ```
/// use std::fs::File;
/// use dng::DngReader;
///
/// let file = File::open("src/testdata/test.dng").expect("couldnt find file");
/// let dng = DngReader::read(file).expect("couldnt read file as dng");
///
/// let main_ifd = dng.main_image_data_ifd_path();
/// let buffer_length = dng.needed_buffer_length_for_image_data(&main_ifd).expect("couldnt read buffer lengths");
/// let mut buffer = vec![0u8; buffer_length];
/// dng.read_image_data_to_buffer(&main_ifd, &mut buffer).expect("couldnt read to buffer");
/// println!("successfully read {} bytes into buffer", buffer.len())
/// ```
pub struct DngReader<R: Read + Seek> {
    file_type: FileType,
    #[derivative(Debug = "ignore")]
    reader: RefCell<ByteOrderReader<R>>,
    ifds: Vec<Ifd>,
}
impl<R: Read + Seek> DngReader<R> {
    /// reads and parses the DNG file IFD-tree eagerly.
    ///
    /// NOTE: OFFSETS (where the image data is located) are not yet read.
    ///
    /// For doing that, you can either use a combination of these functions for reading the data
    /// from OFFSETS entries on a low level:
    /// [get_entry_by_path][Self::get_entry_by_path],
    /// [needed_buffer_size_for_offsets][Self::needed_buffer_size_for_offsets],
    /// [read_offsets_to_buffer][Self::read_offsets_to_buffer]
    ///
    /// Or for a bit higher level direct image data access:
    /// [main_image_data_ifd_path][Self::main_image_data_ifd_path],
    /// [needed_buffer_length_for_image_data][Self::needed_buffer_length_for_image_data],
    /// [read_image_data_to_buffer][Self::read_image_data_to_buffer].
    /// (see usage example).
    pub fn read(mut reader: R) -> Result<Self, DngReaderError> {
        // the first two bytes set the byte order
        let mut header = vec![0u8; 2];
        reader.read_exact(&mut header)?;
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
        let file_type = FileType::from_magic(magic).ok_or_else(|| {
            DngReaderError::FormatError(format!(
                "invalid magic byte sequence (expected 42, got {}",
                magic
            ))
        })?;

        let mut next_ifd_offset = reader.read_u32()?;
        let mut unprocessed_ifds = Vec::new();

        while next_ifd_offset != 0 {
            reader.seek(SeekFrom::Start(next_ifd_offset as u64))?;
            unprocessed_ifds.push(IfdReader::read(&mut reader)?);
            next_ifd_offset = reader.read_u32()?;
        }
        let ifds: Result<Vec<_>, _> = unprocessed_ifds
            .iter()
            .map(|ifd| ifd.process(IfdType::Ifd, &mut reader))
            .collect();

        Ok(Self {
            reader: RefCell::new(reader),
            ifds: ifds?,
            file_type,
        })
    }

    /// Returns the first toplevel IFD of the DNG file.
    pub fn first_ifd(&self) -> &Ifd {
        &self.ifds[0]
    }

    /// Returns the first toplevel IFD of the DNG file.
    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `first_ifd()` instead"
    )]
    pub fn get_ifd0(&self) -> &Ifd {
        &self.first_ifd()
    }

    pub fn entry_by_path<'a>(&'a self, path: &'a IfdPath) -> Option<IfdEntryRef<'a>> {
        for ifd in &self.ifds {
            let result = ifd.entry_by_path(path);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    #[deprecated(
        since = "1.6.0",
        note = "`get_` prefixes are non-canonical Rust; use `entry_by_path()` instead"
    )]
    pub fn get_entry_by_path<'a>(&'a self, path: &'a IfdPath) -> Option<IfdEntryRef<'a>> {
        self.entry_by_path(path)
    }

    /// This low-level function returns the length of a single OFFSETS field.
    ///
    /// Lists are not supported (you must query the individual list member)
    pub fn needed_buffer_size_for_offsets(
        &self,
        entry: IfdEntryRef,
    ) -> Result<usize, DngReaderError> {
        if let Some(IfdTypeInterpretation::Offsets { lengths }) = entry.tag.type_interpretation() {
            let lengths_paths = entry.path.with_last_tag_replaced(lengths.as_maybe());
            let lengths_value = self.entry_by_path(&lengths_paths);
            if let Some(entry) = lengths_value {
                Ok(entry.value.as_u32().unwrap() as usize)
            } else {
                Err(DngReaderError::Other(format!(
                    "length tag {lengths_paths:?} for {:?} not found",
                    entry.path
                )))
            }
        } else {
            Err(DngReaderError::Other(format!(
                "entry {entry:?} is not of type offsets"
            )))
        }
    }
    /// This low-level function can read a single entry from an OFFSETS field to a buffer.
    ///
    /// Lists are not supported (you must query the individual list member)
    pub fn read_offsets_to_buffer(
        &self,
        entry: IfdEntryRef,
        buffer: &mut [u8],
    ) -> Result<(), DngReaderError> {
        let buffer_size = self.needed_buffer_size_for_offsets(entry)?;
        if buffer_size != buffer.len() {
            Err(DngReaderError::Other(format!(
                "buffer has wrong size (expected {buffer_size} found {}",
                buffer.len()
            )))
        } else {
            let mut reader = self.reader.borrow_mut();
            reader.seek(SeekFrom::Start(entry.value.as_u32().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("entry {entry:?} cant be read into buffer. it is not a single OFFSETS"),
                )
            })? as u64))?;
            reader.read_exact(buffer)?;
            Ok(())
        }
    }

    /// Returns the Path to the IFD in which the main image data (not a preview) is stored.
    pub fn main_image_data_ifd_path(&self) -> IfdPath {
        self.first_ifd()
            .find_entry(|entry| {
                entry.tag == &ifd::NewSubfileType.as_maybe() && entry.value.as_u32() == Some(0)
            })
            .map(|entry| entry.parent())
            .unwrap_or_default()
    }

    /// Returns the length in bytes needed for a buffer to store the image data from a given IFD.
    pub fn needed_buffer_length_for_image_data(
        &self,
        ifd_path: &IfdPath,
    ) -> Result<usize, DngReaderError> {
        if let Some(compression) = self.entry_by_path(&ifd_path.chain_tag(ifd::Compression)) {
            if compression.value.as_u32() != Some(1) {
                return Err(DngReaderError::Other(
                    "reading compressed images is not implemented".to_string(),
                ));
            }
        }

        // we try the different options one after another
        if let (Some(_offsets), Some(lengths)) = (
            self.entry_by_path(&ifd_path.chain_tag(ifd::StripOffsets)),
            self.entry_by_path(&ifd_path.chain_tag(ifd::StripByteCounts)),
        ) {
            let sum: u32 = lengths.value.as_list().map(|x| x.as_u32().unwrap()).sum();
            Ok(sum as usize)
        } else if let (Some(_offsets), Some(_lengths)) = (
            self.entry_by_path(&ifd_path.chain_tag(ifd::TileOffsets)),
            self.entry_by_path(&ifd_path.chain_tag(ifd::TileByteCounts)),
        ) {
            Err(DngReaderError::Other(
                "reading tiled images is not implemented".to_string(),
            ))
        } else {
            Err(DngReaderError::Other(
                "No image data was found in the specified IFD or the path didnt lead to an IFD"
                    .to_string(),
            ))
        }
    }

    /// Reads the image data from a given IFD into o given buffer.
    pub fn read_image_data_to_buffer(
        &self,
        ifd_path: &IfdPath,
        buffer: &mut [u8],
    ) -> Result<(), DngReaderError> {
        if let Some(compression) = self.entry_by_path(&ifd_path.chain_tag(ifd::Compression)) {
            if compression.value.as_u32() != Some(1) {
                return Err(DngReaderError::Other(
                    "reading compressed images is not implemented".to_string(),
                ));
            }
        }

        // we try the different options one after another
        if let (Some(offsets), Some(lengths)) = (
            self.entry_by_path(&ifd_path.chain_tag(ifd::StripOffsets)),
            self.entry_by_path(&ifd_path.chain_tag(ifd::StripByteCounts)),
        ) {
            let mut reader = self.reader.borrow_mut();
            let count = offsets.value.count();
            if count != lengths.value.count() {
                return Err(DngReaderError::FormatError(
                    "the counts of OFFSETS and LENGTHS must be the same".to_string(),
                ));
            }
            let mut buffer_offset = 0;
            for (offset, length) in offsets.value.as_list().zip(lengths.value.as_list()) {
                let offset = offset.as_u32().unwrap();
                let length = length.as_u32().unwrap();

                reader.seek(SeekFrom::Start(offset as u64))?;
                let buffer_slice =
                    &mut buffer[(buffer_offset as usize)..((buffer_offset + length) as usize)];
                reader.read_exact(buffer_slice)?;

                buffer_offset += length;
            }
            Ok(())
        } else if let (Some(_offsets), Some(_lengths)) = (
            self.entry_by_path(&ifd_path.chain_tag(ifd::TileOffsets)),
            self.entry_by_path(&ifd_path.chain_tag(ifd::TileByteCounts)),
        ) {
            Err(DngReaderError::Other(
                "reading tiled images is not implemented".to_string(),
            ))
        } else {
            Err(DngReaderError::Other(
                "No image data was found in the specified IFD or the path didnt lead to an IFD"
                    .to_string(),
            ))
        }
    }
}
