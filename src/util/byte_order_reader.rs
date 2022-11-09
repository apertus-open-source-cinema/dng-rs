use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use paste::paste;
use std::{
    io::{self, Read},
    ops::{Deref, DerefMut},
};

pub struct ByteOrderReader<R: Read> {
    reader: R,
    is_ilttle_endian: bool,
}
impl<R: Read> ByteOrderReader<R> {
    pub fn new(reader: R, is_ilttle_endian: bool) -> Self {
        Self {
            reader,
            is_ilttle_endian,
        }
    }
}

macro_rules! generate_read_function {
    ($kind:ty) => {
        paste! {
            #[allow(unused)]
            pub fn [<read_ $kind>](&mut self) -> Result<$kind, io::Error> {
                if self.is_ilttle_endian {
                    self.reader.[<read_ $kind>]::<LittleEndian>()
                } else {
                    self.reader.[<read_ $kind>]::<BigEndian>()
                }
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
    generate_read_function!(u16);
    generate_read_function!(i16);
    generate_read_function!(u32);
    generate_read_function!(i32);
    generate_read_function!(u64);
    generate_read_function!(i64);
    generate_read_function!(f32);
    generate_read_function!(f64);
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
