use crate::byte_order_rw::ByteOrderWriter;
use crate::ifd::{Ifd, IfdEntry, IfdValue};
use crate::FileType;
use derivative::Derivative;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::ops::DerefMut;
use std::sync::Arc;

type PlanFn<W, T> = dyn FnOnce(&mut ByteOrderWriter<W>, &T) -> io::Result<()>;

#[derive(Derivative)]
#[derivative(Debug)]
struct WritePlanEntry<W: Write + Seek, T> {
    offset: u32,
    size: u32,
    #[derivative(Debug = "ignore")]
    write_fn: Box<PlanFn<W, T>>,
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
struct WritePlan<W: Write + Seek, T> {
    to_write: RefCell<VecDeque<WritePlanEntry<W, T>>>,
    write_ptr: RefCell<u32>,
}
impl<W: Write + Seek, T> WritePlan<W, T> {
    pub fn add_entry(
        &self,
        size: u32,
        write_fn: impl FnOnce(&mut ByteOrderWriter<W>, &T) -> io::Result<()> + 'static,
    ) -> u32 {
        let offset = (*self.write_ptr.borrow() + 3) & !3; // we align to word boundaries
        self.to_write.borrow_mut().push_back(WritePlanEntry {
            offset,
            size,
            write_fn: Box::new(write_fn),
        });
        *self.write_ptr.borrow_mut() = offset + size;
        offset
    }
    fn execute(&self, writer: &mut ByteOrderWriter<W>, additional: &T) -> io::Result<()> {
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

            (entry.write_fn)(writer, additional)?;

            let current_offset = writer.seek(SeekFrom::Current(0)).unwrap() as u32;
            if entry.offset + entry.size != current_offset {
                return Err(io::Error::new(io::ErrorKind::Other, format!("entry at {} lied about their write amount. now we are fucked (write_offset={current_offset}, expected={})", entry.offset, entry.offset + entry.size)));
            }
        }
    }
}

/// The main entrypoint for writing DNG/DCP files.
///
/// # Examples
///
/// ```
/// use std::fs::File;
/// use std::sync::Arc;
/// use dng::{DngWriter, FileType, tags};
/// use dng::ifd::{Ifd, IfdEntry, IfdValue};
/// use dng::tags::IfdType;
///
/// let mut file = File::create("/tmp/foo").unwrap();
/// let mut ifd = Ifd::new(IfdType::Ifd);
/// ifd.insert(tags::ifd::Copyright, "this is a test string");
/// ifd.insert(tags::ifd::CFAPattern, &[0u8, 1, 0, 2]);
/// ifd.insert(tags::ifd::StripOffsets, IfdValue::Offsets(Arc::new(vec![0u8, 0, 0, 0])));
/// ifd.insert(tags::ifd::StripByteCounts, 4);
/// DngWriter::write_dng(file, true, FileType::Dng, vec![ifd]).unwrap();
/// ```
#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DngWriter<W: Write + Seek> {
    is_little_endian: bool,
    plan: Arc<WritePlan<W, Self>>,
}
impl<W: Write + Seek> DngWriter<W> {
    /// Writes a DNG/DCP file given the endianness and a list of toplevel [`Ifd`]s.
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
        dng_writer.plan.add_entry(8, move |writer, dng_writer| {
            if is_little_endian {
                writer.write_all(&[0x49, 0x49])?;
            } else {
                writer.write_all(&[0x4D, 0x4D])?;
            }
            writer.write_u16(file_type.magic())?;

            let ifd_address = dng_writer.write_ifds(ifds);
            writer.write_u32(ifd_address)
        });

        let mut writer = ByteOrderWriter::new(writer, is_little_endian);
        dng_writer.plan.execute(&mut writer, &dng_writer)
    }

    fn write_ifds(&self, mut ifds: Vec<Ifd>) -> u32 {
        if ifds.is_empty() {
            return 0; // we write a nullptr to signify that the IFD chain ends
        }
        let ifd = ifds.remove(0);

        // the IFD size is:
        // * 2 byte count
        // * 12 byte for each entry
        // * 4 byte pointer to the next ifd
        let ifd_size = 2 + (ifd.entries.len() as u32 * 12) + 4;
        self.plan.add_entry(ifd_size, move |writer, dng_writer| {
            writer.write_u16(ifd.entries.len() as u16)?;
            for entry in ifd.entries {
                dng_writer.write_ifd_entry(writer, entry)?;
            }
            let next_ifd_address = dng_writer.write_ifds(ifds);
            writer.write_u32(next_ifd_address)
        })
    }
    fn write_ifd_entry(&self, writer: &mut ByteOrderWriter<W>, entry: IfdEntry) -> io::Result<()> {
        // IFD entry layout:
        // * 2 byte tag
        // * 2 byte type
        // * 4 byte count
        // * 4 byte value or pointer
        let count = entry.value.count();
        let dtype = entry.value.ifd_value_type();

        writer.write_u16(entry.tag.into())?;
        writer.write_u16(dtype.into())?;
        writer.write_u32(count)?;

        let required_bytes = count * dtype.size() as u32;
        if required_bytes <= 4 {
            Self::write_value(entry.value, writer, self)?;
            for _ in 0..(4 - required_bytes) {
                writer.write_u8(0)?;
            }
            Ok(())
        } else {
            let value_pointer = self
                .plan
                .add_entry(required_bytes, move |writer, dng_writer| {
                    Self::write_value(entry.value, writer, dng_writer)
                });
            writer.write_u32(value_pointer)
        }
    }

    fn write_value(
        value: IfdValue,
        writer: &mut ByteOrderWriter<W>,
        dng_writer: &DngWriter<W>,
    ) -> io::Result<()> {
        match value {
            IfdValue::Ifd(ifd) => {
                let ifd_offset = dng_writer.write_ifds(vec![ifd]);
                writer.write_u32(ifd_offset)
            }
            IfdValue::Offsets(blob) => {
                let size = blob.size();
                let offset = dng_writer.plan.add_entry(size, move |writer, _| {
                    blob.write(writer.deref_mut())?;
                    Ok(())
                });
                writer.write_u32(offset)
            }
            IfdValue::List(list) => {
                for v in list {
                    Self::write_value(v, writer, dng_writer)?;
                }
                Ok(())
            }
            _ => Self::write_primitive_value(&value, writer),
        }
    }

    fn write_primitive_value(value: &IfdValue, writer: &mut ByteOrderWriter<W>) -> io::Result<()> {
        match value {
            IfdValue::Byte(v) => writer.write_u8(*v),
            IfdValue::Ascii(v) => {
                for b in v.bytes() {
                    writer.write_u8(b)?;
                }
                writer.write_u8(0)
            }
            IfdValue::Short(v) => writer.write_u16(*v),
            IfdValue::Long(v) => writer.write_u32(*v),
            IfdValue::Rational(num, denom) => {
                writer.write_u32(*num)?;
                writer.write_u32(*denom)
            }
            IfdValue::SignedByte(v) => writer.write_i8(*v),
            IfdValue::Undefined(v) => writer.write_u8(*v),
            IfdValue::SignedShort(v) => writer.write_i16(*v),
            IfdValue::SignedLong(v) => writer.write_i32(*v),
            IfdValue::SignedRational(num, denom) => {
                writer.write_i32(*num)?;
                writer.write_i32(*denom)
            }
            IfdValue::Float(v) => writer.write_f32(*v),
            IfdValue::Double(v) => writer.write_f64(*v),
            IfdValue::List(list) => {
                for v in list {
                    Self::write_primitive_value(v, writer)?;
                }
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("value '{value:?} is not primitive"),
            )),
        }
    }
}
