use std::{
    io::{self, Read},
    mem::size_of,
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
            let mut bytes = [0u8; size_of::<$kind>()];
            self.reader.read_exact(&mut bytes)?;
            if self.is_little_endian {
                Ok(<$kind>::from_le_bytes(bytes))
            } else {
                Ok(<$kind>::from_be_bytes(bytes))
            }
        }
    };
}

impl<R: Read> ByteOrderReader<R> {
    generate_read_function!(read_u8, u8);
    generate_read_function!(read_i8, i8);
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
