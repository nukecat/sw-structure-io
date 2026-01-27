pub mod vec;
pub mod pack;
pub mod bounds;

use num_traits::{AsPrimitive, Bounded};

use std::{io::{self, Read, Write}, num::TryFromIntError, usize};

pub struct LittleEndian;
pub struct BigEndian;

pub type LE = LittleEndian;
pub type BE = BigEndian;

pub(crate) trait NumericBytes<Endian>: Copy {
    const SIZE: usize;
    fn write_bytes<W: Write + ?Sized>(&self, writer: &mut W) -> io::Result<()>;
    fn read_bytes<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self>;
}

// Macro to implement NumericBytes for integers and floats
macro_rules! impl_numeric_bytes {
    ($($t:ty),*) => {
        $(
            impl NumericBytes<BigEndian> for $t {
                const SIZE: usize = std::mem::size_of::<$t>();
                fn write_bytes<W: Write + ?Sized>(&self, writer: &mut W) -> io::Result<()> {
                    let bytes = self.to_be_bytes();
                    writer.write_all(&bytes)
                }
                fn read_bytes<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
                    let mut bytes = [0u8; <$t as NumericBytes<BigEndian>>::SIZE];
                    reader.read_exact(&mut bytes)?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
            impl NumericBytes<LittleEndian> for $t {
                const SIZE: usize = std::mem::size_of::<$t>();
                fn write_bytes<W: Write + ?Sized>(&self, writer: &mut W) -> io::Result<()> {
                    let bytes = self.to_le_bytes();
                    writer.write_all(&bytes)
                }
                fn read_bytes<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
                    let mut bytes = [0u8; <$t as NumericBytes<LittleEndian>>::SIZE];
                    reader.read_exact(&mut bytes)?;
                    Ok(Self::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_numeric_bytes!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[derive(thiserror::Error, Debug)]
pub enum WriteVecError {
    #[error("vector length is too large for current length type")]
    TooLong,
    #[error("conversion between types failed")]
    TryIntoInt(#[from] TryFromIntError),
    #[error("I/O error")]
    IO(#[from] std::io::Error)
}

pub(crate) trait WriteUtilsExt: Write {
    fn write_num<T, E>(&mut self, val: T) -> io::Result<()>
    where
    T: NumericBytes<E>
    {
        val.write_bytes(self)
    }

    fn write_array_typed<T, E>(&mut self, array: &[T]) -> io::Result<()>
    where
    T: NumericBytes<E>
    {
        for &v in array {
            self.write_num::<T, E>(v)?;
        }
        Ok(())
    }

    fn write_vec<L, T, E>(&mut self, vec: &[T]) -> Result<(), WriteVecError>
    where   L: NumericBytes<E> + TryFrom<usize> + Bounded + AsPrimitive<usize>,
            L::Error: std::error::Error + 'static,
            T: NumericBytes<E>,
            WriteVecError: From<<L as TryFrom<usize>>::Error> {
        if vec.len() > AsPrimitive::<usize>::as_(L::max_value()) {
            return Err(WriteVecError::TooLong)
        }
        self.write_num::<L, E>(vec.len().try_into()?)?;
        self.write_array_typed::<T, E>(vec)?;
        Ok(())
    }

    fn write_7bit_encoded_int(&mut self, mut value: usize) -> io::Result<()> {
        while value >= 0x80 {
            self.write_all(&[((value as u8 & 0x7F) | 0x80)])?;
            value >>= 7;
        }
        self.write_all(&[value as u8])?;
        Ok(())
    }

    fn write_string_7bit(&mut self, s: &str) -> io::Result<()> {
        self.write_7bit_encoded_int(s.len())?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<W: Write + ?Sized> WriteUtilsExt for W {}

#[derive(thiserror::Error, Debug)]
pub enum ReadVecError {
    #[error("I/O error")]
    IO(#[from] std::io::Error)
}

pub(crate) trait ReadUtilsExt: Read {
    fn read_num<T, E>(&mut self) -> io::Result<T>
    where
    T: NumericBytes<E>
    {
        T::read_bytes(self)
    }

    fn read_array_typed<T, E, const N: usize>(&mut self) -> io::Result<[T; N]>
    where
    T: NumericBytes<E> + Default
    {
        let mut array: [T; N] = [T::default(); N];
        for i in 0..N {
            array[i] = self.read_num::<T, E>()?;
        }
        Ok(array)
    }

    fn read_vec<L, T, E>(&mut self) -> Result<Vec<T>, ReadVecError>
    where
    L: NumericBytes<E> + TryInto<usize>,
    L::Error: std::error::Error + 'static,
    T: NumericBytes<E>,
    usize: From<L>
    {
        let len: usize = self.read_num::<L, E>()?.into();
        let mut vec: Vec<T> = Vec::new();
        for _ in 0..len {
            vec.push(self.read_num::<T, E>()?);
        }
        Ok(vec)
    }

    fn read_7bit_encoded_int(&mut self) -> io::Result<usize> {
        let mut result: usize = 0;
        let mut shift: usize = 0;

        loop {
            let mut buf = [0u8];
            self.read_exact(&mut buf)?;
            let byte = buf[0];

            result |= ((byte & 0x7F) as usize) << shift;

            shift += 7;

            if shift >= usize::BITS as usize {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Too many bytes when decoding 7-bit int.",
                ));
            }

            if (byte & 0x80) == 0 {
                break;
            }
        }

        Ok(result)
    }

    fn read_string_7bit(&mut self) -> io::Result<String> {
        let len = self.read_7bit_encoded_int()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    }
}

impl<R: Read + ?Sized> ReadUtilsExt for R {}
