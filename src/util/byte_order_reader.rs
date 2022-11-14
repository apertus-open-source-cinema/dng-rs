use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::{
    io::{self, Read},
    ops::{Deref, DerefMut},
};

pub struct ByteOrderReader<R: Read> {
    reader: R,
    is_little_endian: bool,
}
impl<R: Read> ByteOrderReader<R> {
    pub fn new(reader: R, is_little_endian: bool) -> Self {
        Self {
            reader,
            is_little_endian,
        }
    }
}

macro_rules! generate_read_function {
    ($name:ident, $kind:ty) => {
        #[allow(unused)]
        pub fn $name(&mut self) -> Result<$kind, io::Error> {
            if self.is_little_endian {
                self.reader.$name::<LittleEndian>()
            } else {
                self.reader.$name::<BigEndian>()
            }
        }
    };
}
impl<R: Read> ByteOrderReader<R> {
    pub fn read_u8(&mut self) -> Result<u8, io::Error> {
        self.reader.read_u8()
    }
    pub fn read_i8(&mut self) -> Result<i8, io::Error> {
        self.reader.read_i8()
    }
    generate_read_function!(read_u16, u16);
    generate_read_function!(read_i16, i16);
    generate_read_function!(read_u32, u32);
    generate_read_function!(read_i32, i32);
    generate_read_function!(read_u64, u64);
    generate_read_function!(read_i64, i64);
    generate_read_function!(read_f32, f32);
    generate_read_function!(read_f64, f64);
}

impl<R: Read> Deref for ByteOrderReader<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl<R: Read> DerefMut for ByteOrderReader<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}
