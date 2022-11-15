use crate::ifd::{Ifd, IfdEntry, IfdValue};
use crate::util::byte_order_writer::ByteOrderWriter;
use crate::FileType;
use derivative::Derivative;
use num_traits::ToPrimitive;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WritePlanEntry<W: Write + Seek> {
    offset: u32,
    size: u32,
    #[derivative(Debug = "ignore")]
    write_fn: Box<dyn FnOnce(&mut ByteOrderWriter<W>) -> io::Result<()>>,
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct WritePlan<W: Write + Seek> {
    to_write: RefCell<VecDeque<WritePlanEntry<W>>>,
    write_ptr: RefCell<u32>,
}
impl<W: Write + Seek> WritePlan<W> {
    pub fn add_entry(
        &self,
        size: u32,
        write_fn: impl FnOnce(&mut ByteOrderWriter<W>) -> io::Result<()> + 'static,
    ) -> u32 {
        let offset = *self.write_ptr.borrow() + 3 & !3; // we align to word boundaries
        self.to_write.borrow_mut().push_back(WritePlanEntry {
            offset,
            size,
            write_fn: Box::new(write_fn),
        });
        *self.write_ptr.borrow_mut() = offset + size;
        offset
    }
    fn execute(&self, writer: &mut ByteOrderWriter<W>) -> io::Result<()> {
        loop {
            let entry = if let Some(entry) = self.to_write.borrow_mut().pop_front() {
                entry
            } else {
                return Ok(());
            };
            let current_offset = writer.seek(SeekFrom::Current(0)).unwrap() as u32;
            if entry.offset < current_offset {
                return Err(io::Error::new(io::ErrorKind::Other, format!("someone before lied about their write amount. now we are fucked (write_offset={current_offset}, expected={})", entry.offset)));
            }
            // add padding if required
            for _ in 0..(entry.offset - current_offset) {
                writer.write_u8(0)?;
            }

            (entry.write_fn)(writer)?;

            let current_offset = writer.seek(SeekFrom::Current(0)).unwrap() as u32;
            if entry.offset + entry.size != current_offset {
                return Err(io::Error::new(io::ErrorKind::Other, format!("entry at {} lied about their write amount. now we are fucked (write_offset={current_offset}, expected={})", entry.offset, entry.offset + entry.size)));
            }
        }
    }
}

#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DngWriter<W: Write + Seek> {
    is_little_endian: bool,
    plan: Arc<WritePlan<W>>,
}
impl<W: Write + Seek + 'static> DngWriter<W> {
    pub fn write_dng(
        writer: W,
        is_little_endian: bool,
        file_type: FileType,
        ifds: Vec<Ifd>,
    ) -> io::Result<()> {
        let plan = Arc::new(WritePlan::default());
        let dng_writer = Self {
            is_little_endian,
            plan,
        };
        let dng_writer_clone = dng_writer.clone();
        dng_writer.plan.add_entry(8, move |writer| {
            if is_little_endian {
                writer.write(&[0x49, 0x49])?;
            } else {
                writer.write(&[0x4D, 0x4D])?;
            }
            writer.write_u16(file_type.to_u16().unwrap())?;

            let ifd_address = dng_writer_clone.write_ifds(ifds);
            writer.write_u32(ifd_address)
        });

        let mut writer = ByteOrderWriter::new(writer, is_little_endian);
        dng_writer.plan.execute(&mut writer)
    }

    pub fn write_ifds(&self, mut ifds: Vec<Ifd>) -> u32 {
        if ifds.is_empty() {
            return 0; // we write a nullptr to signify that the IFD chain ends
        }
        let ifd = ifds.remove(0);

        // the IFD size is:
        // * 2 byte count
        // * 12 byte for each entry
        // * 4 byte pointer to the next ifd
        let ifd_size = 2 + (ifd.entries.len() as u32 * 12) + 4;
        let self_clone = self.clone();
        self.plan.add_entry(ifd_size, move |writer| {
            writer.write_u16(ifd.entries.len() as u16)?;
            for entry in ifd.entries {
                self_clone.write_ifd_entry(writer, entry)?;
            }
            let next_ifd_address = self_clone.write_ifds(ifds);
            writer.write_u32(next_ifd_address)
        })
    }
    pub fn write_ifd_entry(
        &self,
        writer: &mut ByteOrderWriter<W>,
        entry: IfdEntry,
    ) -> io::Result<()> {
        // IFD entry layout:
        // * 2 byte tag
        // * 2 byte type
        // * 4 byte count
        // * 4 byte value or pointer
        let count = entry.value.get_count();
        let dtype = entry.value.get_ifd_value_type();

        writer.write_u16(entry.tag.numeric())?;
        writer.write_u16(dtype.to_u16().unwrap())?;
        writer.write_u32(count)?;

        fn write_value<W: Write + Seek + 'static>(
            value: IfdValue,
            writer: &mut ByteOrderWriter<W>,
            dng_writer: &DngWriter<W>,
        ) -> io::Result<()> {
            match value {
                IfdValue::Byte(v) => writer.write_u8(v),
                IfdValue::Ascii(v) => {
                    for b in v.bytes() {
                        writer.write_u8(b)?;
                    }
                    writer.write_u8(0)
                }
                IfdValue::Short(v) => writer.write_u16(v),
                IfdValue::Long(v) => writer.write_u32(v),
                IfdValue::Rational(num, denom) => {
                    writer.write_u32(num)?;
                    writer.write_u32(denom)
                }
                IfdValue::SByte(v) => writer.write_i8(v),
                IfdValue::Undefined(v) => writer.write_u8(v),
                IfdValue::SShort(v) => writer.write_i16(v),
                IfdValue::SLong(v) => writer.write_i32(v),
                IfdValue::SRational(num, denom) => {
                    writer.write_i32(num)?;
                    writer.write_i32(denom)
                }
                IfdValue::Float(v) => writer.write_f32(v),
                IfdValue::Double(v) => writer.write_f64(v),
                IfdValue::List(list) => {
                    for v in list {
                        write_value(v.value, writer, dng_writer)?;
                    }
                    Ok(())
                }
                IfdValue::Ifd(ifd) => {
                    let ifd_offset = dng_writer.write_ifds(vec![ifd]);
                    writer.write_u32(ifd_offset)
                }
            }
        }

        let required_bytes = count * dtype.needed_bytes();
        if required_bytes <= 4 {
            write_value(entry.value, writer, self)?;
            for _ in 0..(4 - required_bytes) {
                writer.write_u8(0)?;
            }
            Ok(())
        } else {
            let self_clone = self.clone();
            let value_pointer = self.plan.add_entry(required_bytes, move |writer| {
                write_value(entry.value, writer, &self_clone)
            });
            writer.write_u32(value_pointer)
        }
    }
}
