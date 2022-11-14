use crate::ifd::{Ifd, IfdEntry, IfdPath, IfdValue};
use crate::ifd_tag_data::tag_info_parser::IfdTypeInterpretation;
use crate::ifd_tag_data::tag_info_parser::{IfdTagDescriptor, IfdValueType};
use crate::util::byte_order_reader::ByteOrderReader;
use crate::IfdType;
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
    pub fn process(
        &self,
        ifd_type: IfdType,
        path: &IfdPath,
        reader: &mut ByteOrderReader<impl Read + Seek>,
    ) -> Result<Ifd, io::Error> {
        let mut ifd = Ifd::new(ifd_type, path.clone());
        for entry in &self.entries {
            let tag = IfdTagDescriptor::from_number(entry.tag, ifd_type);
            ifd.insert(entry.process(reader, &tag, path)?);
        }
        Ok(ifd)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnprocessedIfdEntry {
    pub tag: u16,
    pub dtype: IfdValueType,
    pub count: u32,
    value_or_offset: u32,
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
            value_or_offset: offset,
            own_offset,
        })
    }

    // if the value fits into 4 byte, it is stored inline
    fn fits_inline(&self) -> bool {
        self.count * self.dtype.needed_bytes() <= 4
    }

    pub fn process(
        &self,
        reader: &mut ByteOrderReader<impl Read + Seek>,
        tag: &IfdTagDescriptor,
        path: &IfdPath,
    ) -> Result<IfdEntry, io::Error> {
        let path = path.chain_tag(tag.clone());

        if self.fits_inline() {
            reader.seek(SeekFrom::Start((self.own_offset + 8) as u64))?;
        } else {
            reader.seek(SeekFrom::Start(self.value_or_offset as u64))?;
        }

        let dtype = self.dtype;
        let mut get_value = |path: &IfdPath| -> Result<IfdValue, io::Error> {
            let parsed = match dtype {
                IfdValueType::Byte => IfdValue::Byte(reader.read_u8()?),
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
                IfdValueType::Ascii => unreachable!(), // lists of ascii are not a thing
            };

            if let Some(IfdTypeInterpretation::IfdOffset { ifd_type }) =
                tag.get_known_type_interpretation()
            {
                let current = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(parsed.as_u32().unwrap() as u64))?;
                let unprocessed_ifd = UnprocessedIfd::read(reader)?;
                let ifd = unprocessed_ifd.process(*ifd_type, path, reader)?;
                reader.seek(SeekFrom::Start(current))?;
                return Ok(IfdValue::Ifd(ifd));
            } else {
                Ok(parsed)
            }
        };

        let value = if self.count == 1 {
            get_value(&path)?
        } else if self.dtype == IfdValueType::Ascii {
            let mut buf = vec![0u8; (self.count - 1) as usize];
            reader.read_exact(&mut buf)?;
            IfdValue::Ascii(String::from_utf8_lossy(&buf).to_string())
        } else {
            let vec: Result<Vec<_>, _> = (0..self.count)
                .map(|i| -> Result<_, io::Error> {
                    let path = path.chain_list_index(i as u16);
                    Ok(IfdEntry {
                        value: get_value(&path)?,
                        path: path.clone(),
                        tag: tag.clone(),
                    })
                })
                .collect();
            IfdValue::List(vec?)
        };
        Ok(IfdEntry {
            value,
            tag: tag.clone(),
            path: path.clone(),
        })
    }
}
