use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::io::Write;
use std::{
    io::{self},
    ops::{Deref, DerefMut},
};

pub struct ByteOrderWriter<W: Write> {
    writer: W,
    is_little_endian: bool,
}
impl<W: Write> ByteOrderWriter<W> {
    pub fn new(writer: W, is_little_endian: bool) -> Self {
        Self {
            writer,
            is_little_endian,
        }
    }
}

macro_rules! generate_write_function {
    ($name:ident, $kind:ty) => {
        #[allow(unused)]
        pub fn $name(&mut self, value: $kind) -> Result<(), io::Error> {
            if self.is_little_endian {
                self.writer.$name::<LittleEndian>(value)
            } else {
                self.writer.$name::<BigEndian>(value)
            }
        }
    };
}
impl<W: Write> ByteOrderWriter<W> {
    pub fn write_u8(&mut self, value: u8) -> Result<(), io::Error> {
        self.writer.write_u8(value)
    }
    pub fn write_i8(&mut self, value: i8) -> Result<(), io::Error> {
        self.writer.write_i8(value)
    }
    generate_write_function!(write_u16, u16);
    generate_write_function!(write_i16, i16);
    generate_write_function!(write_u32, u32);
    generate_write_function!(write_i32, i32);
    generate_write_function!(write_u64, u64);
    generate_write_function!(write_i64, i64);
    generate_write_function!(write_f32, f32);
    generate_write_function!(write_f64, f64);
}

impl<W: Write> Deref for ByteOrderWriter<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}
impl<W: Write> DerefMut for ByteOrderWriter<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}
