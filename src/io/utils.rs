use std::{io::{self, Read, Write}, usize};
use std::mem::MaybeUninit;
use num_traits::{AsPrimitive, Num};

const ROTATION_MULTIPLIER: f32 = (u16::MAX as f32) / 360.0f32;
const ROTATION_INV: f32 = 360.0 / (u16::MAX as f32);

#[derive(Clone)]
pub(crate) struct Bounds {
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

impl Bounds {
    pub(crate) const fn from_center_and_size(center: [f32; 3], size: [f32; 3]) -> Self {
        let mut min = [0.0f32; 3];
        let mut max = [0.0f32; 3];

        let mut i = 0;
        while i < 3 {
            min[i] = center[i] - size[i] * 0.5;
            max[i] = center[i] + size[i] * 0.5;
            i += 1;
        }

        Self { min, max }
    }

    pub(crate) const fn get_center_and_size(&self) -> ([f32; 3], [f32; 3]) {
        let mut center = [0.0f32; 3];
        let mut size = [0.0f32; 3];

        let mut i = 0;
        while i < 3 {
            center[i] = (self.min[i] + self.max[i]) * 0.5;
            size[i] = self.max[i] - self.min[i];
            i += 1;
        }

        (center, size)
    }

    pub(crate) fn to_inbounds(&self, f: [f32; 3]) -> [i16; 3] {
        let (center, size) = self.get_center_and_size();

        let mut result = [0i16; 3];
        for i in 0..3 {
            let multiplier = (1.0f32 / size[i]) * i16::MAX as f32;
            result[i] = ((f[i] - center[i]) * multiplier).round() as i16
        }
        result
    }

    pub(crate) fn to_global(&self, v: [i16; 3]) -> [f32; 3] {
        let (center, size) = self.get_center_and_size();

        let mut result = [0.0f32; 3];
        for i in 0..3 {
            let multiplier = size[i] / i16::MAX as f32;
            result[i] = center[i] + v[i] as f32 * multiplier;
        }
        result
    }

    pub(crate) fn encapsulate(&mut self, block_position: &[f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(block_position[i]);
            self.max[i] = self.max[i].max(block_position[i]);
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            max: [f32::NEG_INFINITY; 3],
            min: [f32::INFINITY; 3]
        }
    }
}

pub(crate) fn pack_rotation(data: [f32; 3]) -> [u16; 3] {
    let mut out = [0u16; 3];
    for (i, &angle) in data.iter().enumerate() {
        // Normalize angle into [0.0, 360.0)
        let mut a = angle % 360.0_f32;
        if a < 0.0 {
            a += 360.0_f32;
        }

        // Multiply and round to nearest. Use saturating cast to avoid overflow.
        let scaled = a * ROTATION_MULTIPLIER;
        // Clamp into [0.0, u16::MAX as f32] to be safe for extreme inputs
        let clamped = if scaled.is_finite() {
            scaled.max(0.0).min(u16::MAX as f32)
        } else {
            0.0
        };
        out[i] = clamped.round() as u16;
    }
    out
}

pub(crate) fn unpack_rotation(data: [u16; 3]) -> [f32; 3] {
    [
        (data[0] as f32) * ROTATION_INV,
        (data[1] as f32) * ROTATION_INV,
        (data[2] as f32) * ROTATION_INV,
    ]
}

pub(crate) fn pack_bools(bools: &[bool]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((bools.len() + 7) / 8);
    for chunk in bools.chunks(8) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            byte |= (b as u8) << i;
        }
        bytes.push(byte);
    }
    bytes
}

pub(crate) fn unpack_bools(bytes: &[u8], count: usize) -> Vec<bool> {
    let mut bools = Vec::with_capacity(count);
    for &byte in bytes.iter() {
        for bit in 0..8 {
            if bools.len() == count {
                return bools;
            }
            bools.push((byte >> bit) & 1 != 0);
        }
    }
    bools
}

pub(crate) fn pack_color([r, g, b]: [u8; 3]) -> u16 {
    ((r & 0xF8) as u16) << 8 | ((g & 0xFC) as u16) << 2 | ((b & 0xF8) as u16) >> 3
}

pub(crate) fn unpack_color(rgb565: u16) -> [u8; 3] {
    [
        ((rgb565 >> 8) & 0xF8) as u8,
        ((rgb565 >> 2) & 0xFC) as u8,
        ((rgb565 << 3) & 0xF8) as u8,
    ]
}

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

pub(crate) trait WriteUtilsExt: Write {
    fn write_num<T, E>(&mut self, val: T) -> io::Result<()>
    where
        T: NumericBytes<E>
    {
        val.write_bytes(self)
    }
    
    fn write_array<T, E>(&mut self, array: &[T]) -> io::Result<()> 
    where
        T: NumericBytes<E>
    {
        for &v in array {
            self.write_num::<T, E>(v)?;
        }
        Ok(())
    }

    fn write_vec<L, T, E>(&mut self, vec: &[T]) -> Result<(), Box<dyn std::error::Error>>
    where
        L: NumericBytes<E> + TryFrom<usize>,
        L::Error: std::error::Error + 'static,
        T: NumericBytes<E>,
    {
        self.write_num::<L, E>(vec.len().try_into()?)?;
        self.write_array::<T, E>(vec)?;
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

pub(crate) trait ReadUtilsExt: Read {
    fn read_num<T, E>(&mut self) -> io::Result<T>
    where
        T: NumericBytes<E>
    {
        T::read_bytes(self)
    }

    fn read_array<T, E, const N: usize>(&mut self) -> io::Result<[T; N]>
    where
        T: NumericBytes<E> + Default
    {
        let mut array: [T; N] = [T::default(); N];
        for i in 0..N {
            array[i] = self.read_num::<T, E>()?;
        }
        Ok(array)
    }

    fn read_vec<L, T, E>(&mut self) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        L: NumericBytes<E> + TryInto<usize>,
        L::Error: std::error::Error + 'static,
        T: NumericBytes<E>
    {
        let len: usize = self.read_num::<L, E>()?.try_into()?;
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

pub(crate) trait TryIntoVecExt<T: Copy>: IntoIterator<Item = T> + Sized + AsRef<[T]> {
    fn try_into_vec<U>(&self) -> Result<Vec<U>, Box<dyn std::error::Error>>
    where 
        U: TryFrom<T>,
        U::Error: std::error::Error + 'static
    {
        self
            .as_ref()
            .iter()
            .map(|&v| U::try_from(v).map_err(|e| Box::new(e) as Box<dyn std::error::Error>))
            .collect()
    }
}

pub(crate) trait IntoVecExt<T: Copy>: IntoIterator<Item = T> + Sized + AsRef<[T]> {
    fn into_vec<U>(&self) -> Vec<U>
    where
        U: From<T>
    {
        self
            .as_ref()
            .iter()
            .map(|&v| U::from(v))
            .collect()
    }
}

pub(crate) trait IntoVecLossyExt<T: Copy>: IntoIterator<Item = T> + Sized + AsRef<[T]> {
    fn into_vec_lossy<U>(&self) -> Vec<U>
    where
        U: TryFrom<T>,
        U::Error: std::error::Error + 'static
    {
        self
            .as_ref()
            .iter()
            .filter_map(|&v| U::try_from(v).ok())
            .collect()
    }
}

impl<T: Copy> TryIntoVecExt<T> for Vec<T> {}

impl<T: Copy> IntoVecExt<T> for Vec<T> {}

impl<T: Copy> IntoVecLossyExt<T> for Vec<T> {}