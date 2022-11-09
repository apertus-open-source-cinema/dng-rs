use super::byte_order_reader::ByteOrderReader;
use crate::exif::{
    tag_info_parser::{ExifFieldDescriptor, ExifTypeInterpretation, MaybeExifTypeInterpretation},
    ExifMetadata, ExifTag, ExifValue, ExifValueType,
};
use num_traits::FromPrimitive;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, PartialEq, Eq)]
pub struct Ifd {
    pub entries: Vec<IfdEntry>,
}

impl Ifd {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let count = reader.read_u16()?;
        let entries: Result<Vec<_>, _> = (0..count)
            .map(|_| IfdEntry::read(reader))
            .filter(|x| x.is_ok())
            .collect();
        Ok(Self { entries: entries? })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IfdEntry {
    pub tag: u16,
    pub dtype: ExifValueType,
    pub count: u32,
    offset: u32,
    own_offset: u32,
}

impl IfdEntry {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let own_offset = reader.seek(SeekFrom::Current(0))? as u32;
        let tag = reader.read_u16()?;
        let dtype = reader.read_u16()?;
        let dtype = ExifValueType::from_u16(dtype).ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "encountered unknown value '{}' in IFD type field (tag {:#04X})",
                dtype, tag
            ),
        ))?;
        let count = reader.read_u32()?;
        let offset = reader.read_u32()?;
        Ok(Self {
            tag,
            dtype,
            count,
            offset,
            own_offset,
        })
    }

    // if the value fits into 4 byte, it is stored inline
    fn fits_inline(&self) -> bool {
        self.count * self.dtype.needed_bytes() <= 4
    }

    pub fn get_value(
        &self,
        reader: &mut ByteOrderReader<impl Read + Seek>,
        tag: &ExifTag,
    ) -> Result<ExifValue, io::Error> {
        if self.fits_inline() {
            reader.seek(SeekFrom::Start((self.own_offset + 8) as u64))?;
        } else {
            reader.seek(SeekFrom::Start(self.offset as u64))?;
        }

        fn get_value_inner(
            dtype: ExifValueType,
            reader: &mut ByteOrderReader<impl Read + Seek>,
            tag: &ExifTag,
        ) -> Result<ExifValue, io::Error> {
            let parsed = match dtype {
                ExifValueType::Byte => ExifValue::Byte(reader.read_u8()?),
                ExifValueType::Ascii => ExifValue::Ascii((reader.read_u8()? as char).to_string()),
                ExifValueType::Short => ExifValue::Short(reader.read_u16()?),
                ExifValueType::Long => ExifValue::Long(reader.read_u32()?),
                ExifValueType::Rational => {
                    ExifValue::Rational(reader.read_u32()?, reader.read_u32()?)
                }
                ExifValueType::SByte => ExifValue::SByte(reader.read_i8()?),
                ExifValueType::Undefined => ExifValue::Undefined(reader.read_u8()?),
                ExifValueType::SShort => ExifValue::SByte(reader.read_i8()?),
                ExifValueType::SLong => ExifValue::SLong(reader.read_i32()?),
                ExifValueType::SRational => {
                    ExifValue::SRational(reader.read_i32()?, reader.read_i32()?)
                }
                ExifValueType::Float => ExifValue::Float(reader.read_f32()?),
                ExifValueType::Double => ExifValue::Double(reader.read_f64()?),
            };

            if let ExifTag::Known(ExifFieldDescriptor {
                interpretation:
                    MaybeExifTypeInterpretation::Known(ExifTypeInterpretation::IfdOffset { ifd_type }),
                ..
            }) = tag
            {
                let current = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(parsed.as_u32().unwrap() as u64))?;
                let ifd = Ifd::read(reader)?;
                let mut metadata = ExifMetadata::default();
                for entry in &ifd.entries {
                    let tag = ExifTag::from_number(entry.tag, *ifd_type);
                    metadata.insert(tag.clone(), entry.get_value(reader, &tag)?);
                }
                reader.seek(SeekFrom::Start(current))?;
                return Ok(ExifValue::Ifd(metadata));
            } else {
                Ok(parsed)
            }
        }

        if self.count == 1 {
            get_value_inner(self.dtype, reader, tag)
        } else {
            let vec: Result<Vec<_>, _> = (0..self.count)
                .map(|_| get_value_inner(self.dtype, reader, tag))
                .collect();
            if self.dtype == ExifValueType::Ascii {
                let vec = vec?;
                let len = vec.len();
                let string: String = vec
                    .iter()
                    .enumerate()
                    .filter_map(|(i, x)| {
                        if i >= len - 1 {
                            return None;
                        }
                        if let ExifValue::Ascii(s) = x {
                            Some(s.chars().next().unwrap())
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(ExifValue::Ascii(string))
            } else {
                Ok(ExifValue::List(vec?))
            }
        }
    }
}
