use super::byte_order_reader::ByteOrderReader;
use crate::exif::{ExifValue, ExifValueType};
use num_traits::FromPrimitive;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, PartialEq, Eq)]
pub struct Ifd {
    pub entries: Vec<IfdEntry>,
}

impl Ifd {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let count = reader.read_u16()?;
        let entries: Result<Vec<_>, _> = (0..count).map(|_| IfdEntry::read(reader)).collect();
        Ok(Self { entries: entries? })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IfdEntry {
    pub tag: u16,
    pub ty: ExifValueType,
    pub count: u32,
    offset: u32,
    own_offset: u32,
}

impl IfdEntry {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let own_offset = reader.seek(SeekFrom::Current(0))? as u32;
        Ok(Self {
            tag: reader.read_u16()?,
            ty: ExifValueType::from_u16(reader.read_u16()?).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "encountered value in IFD tye field",
            ))?,
            count: reader.read_u32()?,
            offset: reader.read_u32()?,
            own_offset,
        })
    }

    // if the value fits into 4 byte, it is stored inline
    fn fits_inline(&self) -> bool {
        self.count * self.ty.needed_bytes() <= 4
    }

    pub fn get_value(
        &self,
        reader: &mut ByteOrderReader<impl Read + Seek>,
    ) -> Result<ExifValue, io::Error> {
        if self.fits_inline() {
            reader.seek(SeekFrom::Start((self.own_offset + 8) as u64))?;
        } else {
            reader.seek(SeekFrom::Start(self.offset as u64))?;
        }

        fn get_value_inner(
            ty: ExifValueType,
            reader: &mut ByteOrderReader<impl Read + Seek>,
        ) -> Result<ExifValue, io::Error> {
            Ok(match ty {
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
            })
        }

        if self.count == 1 {
            get_value_inner(self.ty, reader)
        } else {
            let vec: Result<Vec<_>, _> = (0..self.count)
                .map(|_| get_value_inner(self.ty, reader))
                .collect();
            if self.ty == ExifValueType::Ascii {
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
