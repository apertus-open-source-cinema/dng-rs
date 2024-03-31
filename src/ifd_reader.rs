use crate::byte_order_rw::ByteOrderReader;
use crate::ifd::{Ifd, IfdValue};
use crate::tags::{IfdType, IfdTypeInterpretation, IfdValueType, MaybeKnownIfdFieldDescriptor};
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, PartialEq, Eq)]
pub struct IfdReader {
    pub entries: Vec<IfdEntryReader>,
}
impl IfdReader {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let count = reader.read_u16()?;
        let entries: Result<Vec<_>, _> = (0..count)
            .map(|_| IfdEntryReader::read(reader))
            .filter(|x| x.is_ok())
            .collect();
        Ok(Self { entries: entries? })
    }
    pub fn process(
        &self,
        ifd_type: IfdType,
        reader: &mut ByteOrderReader<impl Read + Seek>,
    ) -> Result<Ifd, io::Error> {
        let mut ifd = Ifd::new(ifd_type);
        for entry in &self.entries {
            let tag = MaybeKnownIfdFieldDescriptor::from_number(entry.tag, ifd_type);
            ifd.insert(tag, entry.process(reader, tag)?);
        }
        Ok(ifd)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IfdEntryReader {
    pub tag: u16,
    pub dtype: IfdValueType,
    pub count: u32,
    value_or_offset: u32,
    own_offset: u32,
}
impl IfdEntryReader {
    pub fn read(reader: &mut ByteOrderReader<impl Read + Seek>) -> Result<Self, io::Error> {
        let own_offset = reader.seek(SeekFrom::Current(0))? as u32;
        let tag = reader.read_u16()?;
        let dtype = reader.read_u16()?;
        let dtype = IfdValueType::from_u16(dtype).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "encountered unknown value '{}' in IFD type field (tag {:#04X})",
                    dtype, tag
                ),
            )
        })?;
        let count = reader.read_u32()?;
        let value_or_offset = reader.read_u32()?;
        Ok(Self {
            tag,
            dtype,
            count,
            value_or_offset,
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
        tag: MaybeKnownIfdFieldDescriptor,
    ) -> Result<IfdValue, io::Error> {
        if self.fits_inline() {
            reader.seek(SeekFrom::Start((self.own_offset + 8) as u64))?;
        } else {
            reader.seek(SeekFrom::Start(self.value_or_offset as u64))?;
        }

        let value = if let Some(IfdTypeInterpretation::IfdOffset { ifd_type }) =
            tag.get_type_interpretation()
        {
            assert_eq!(self.dtype, IfdValueType::Long);
            let mut read_ifd = || -> Result<IfdValue, io::Error> {
                let offset = reader.read_u32()?;
                let current = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(offset as u64))?;
                let unprocessed_ifd = IfdReader::read(reader)?;
                let ifd = unprocessed_ifd.process(*ifd_type, reader)?;
                reader.seek(SeekFrom::Start(current))?;
                Ok(IfdValue::Ifd(ifd))
            };
            match self.count {
                1 => read_ifd(),
                n => {
                    let vec: Result<Vec<_>, _> = (0..n).map(|_| read_ifd()).collect();
                    Ok(IfdValue::List(vec?))
                }
            }
        } else {
            Self::read_primitive_ifd_value(self.dtype, self.count, reader)
        }?;
        Ok(value)
    }

    fn read_primitive_ifd_value(
        dtype: IfdValueType,
        count: u32,
        reader: &mut ByteOrderReader<impl Read>,
    ) -> io::Result<IfdValue> {
        let value = if let IfdValueType::Ascii = dtype {
            let mut buf = vec![0u8; (count - 1) as usize];
            reader.read_exact(&mut buf)?;
            IfdValue::Ascii(String::from_utf8_lossy(&buf).to_string())
        } else if count > 1 {
            let vec: Result<Vec<_>, _> = (0..count)
                .map(|_| Self::read_primitive_ifd_value(dtype, 1, reader))
                .collect();
            IfdValue::List(vec?)
        } else {
            match dtype {
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
                IfdValueType::Ascii => unreachable!(),
            }
        };
        Ok(value)
    }
}
