use crate::ifd::{Ifd, IfdValue};
use crate::ifd_tag_data::tag_info_parser::{
    IfdFieldDescriptor, IfdTypeInterpretation, MaybeIfdTypeInterpretation,
};
use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdValueType};
use crate::util::byte_order_reader::ByteOrderReader;
use num_traits::FromPrimitive;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, PartialEq, Eq)]
pub struct UnprocessedIfd {
    pub entries: Vec<UnprocessedIfdEntry>,
}
impl UnprocessedIfd {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let count = reader.read_u16()?;
        let entries: Result<Vec<_>, _> = (0..count)
            .map(|_| UnprocessedIfdEntry::read(reader))
            .filter(|x| x.is_ok())
            .collect();
        Ok(Self { entries: entries? })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnprocessedIfdEntry {
    pub tag: u16,
    pub dtype: IfdValueType,
    pub count: u32,
    offset: u32,
    own_offset: u32,
}
impl UnprocessedIfdEntry {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let own_offset = reader.seek(SeekFrom::Current(0))? as u32;
        let tag = reader.read_u16()?;
        let dtype = reader.read_u16()?;
        let dtype = IfdValueType::from_u16(dtype).ok_or(io::Error::new(
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
        tag: &IfdTagDescriptor,
    ) -> Result<IfdValue, io::Error> {
        if self.fits_inline() {
            reader.seek(SeekFrom::Start((self.own_offset + 8) as u64))?;
        } else {
            reader.seek(SeekFrom::Start(self.offset as u64))?;
        }

        fn get_value_inner(
            dtype: IfdValueType,
            reader: &mut ByteOrderReader<impl Read + Seek>,
            tag: &IfdTagDescriptor,
        ) -> Result<IfdValue, io::Error> {
            let parsed = match dtype {
                IfdValueType::Byte => IfdValue::Byte(reader.read_u8()?),
                IfdValueType::Ascii => IfdValue::Ascii((reader.read_u8()? as char).to_string()),
                IfdValueType::Short => IfdValue::Short(reader.read_u16()?),
                IfdValueType::Long => IfdValue::Long(reader.read_u32()?),
                IfdValueType::Rational => {
                    IfdValue::Rational(reader.read_u32()?, reader.read_u32()?)
                }
                IfdValueType::SByte => IfdValue::SByte(reader.read_i8()?),
                IfdValueType::Undefined => IfdValue::Undefined(reader.read_u8()?),
                IfdValueType::SShort => IfdValue::SByte(reader.read_i8()?),
                IfdValueType::SLong => IfdValue::SLong(reader.read_i32()?),
                IfdValueType::SRational => {
                    IfdValue::SRational(reader.read_i32()?, reader.read_i32()?)
                }
                IfdValueType::Float => IfdValue::Float(reader.read_f32()?),
                IfdValueType::Double => IfdValue::Double(reader.read_f64()?),
            };

            if let IfdTagDescriptor::Known(IfdFieldDescriptor {
                interpretation:
                    MaybeIfdTypeInterpretation::Known(IfdTypeInterpretation::IfdOffset { ifd_type }),
                ..
            }) = tag
            {
                let current = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(parsed.as_u32().unwrap() as u64))?;
                let ifd = UnprocessedIfd::read(reader)?;
                let mut metadata = Ifd::new(*ifd_type);
                for entry in &ifd.entries {
                    let tag = IfdTagDescriptor::from_number(entry.tag, *ifd_type);
                    metadata.insert(tag.clone(), entry.get_value(reader, &tag)?);
                }
                reader.seek(SeekFrom::Start(current))?;
                return Ok(IfdValue::Ifd(metadata));
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
            if self.dtype == IfdValueType::Ascii {
                let vec = vec?;
                let len = vec.len();
                let string: String = vec
                    .iter()
                    .enumerate()
                    .filter_map(|(i, x)| {
                        if i >= len - 1 {
                            return None;
                        }
                        if let IfdValue::Ascii(s) = x {
                            Some(s.chars().next().unwrap())
                        } else {
                            unreachable!()
                        }
                    })
                    .collect();
                Ok(IfdValue::Ascii(string))
            } else {
                Ok(IfdValue::List(vec?))
            }
        }
    }
}
