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
            let bytes = if self.is_little_endian {
                <$kind>::to_le_bytes(value)
            } else {
                <$kind>::to_be_bytes(value)
            };
            self.writer.write_all(&bytes)
        }
    };
}

impl<W: Write> ByteOrderWriter<W> {
    generate_write_function!(write_u8, u8);
    generate_write_function!(write_i8, i8);
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
