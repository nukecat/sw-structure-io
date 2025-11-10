use std::io::{Read, Write};
use std::io;
use std::io::Result;
use byteorder::{LittleEndian, WriteBytesExt};

const ROTATION_MULTIPLIER: f32 = (u16::MAX as f32) / 360.0f32;
const ROTATION_INV: f32 = 360.0 / (u16::MAX as f32);

pub(crate) fn pack_rotation(data: [f32; 3]) -> [u16; 3] {
    let mut out = [0u16; 3];
    for (i, &angle) in data.iter().enumerate() {
        // Normalize angle into [0.0, 360.0)
        let mut a = angle % 360.0_f32;
        if a < 0.0 { a += 360.0_f32; }

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

pub(crate) struct Bounds {
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3]
}

impl Bounds {
    pub(crate) const fn new() -> Self {
        Bounds {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3]
        }
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

pub(crate) fn pack_bools(bools: &[bool]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((bools.len() + 7) / 8);
    for chunk in bools.chunks(8) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            byte |= (b as u8) << (7 - i);
        }
        bytes.push(byte);
    }
    bytes
}

pub(crate) fn unpack_bools(bytes: &[u8], count: usize) -> Vec<bool> {
    let mut bools = Vec::with_capacity(count);
    for (i, &byte) in bytes.iter().enumerate() {
        for bit in 0..8 {
            if bools.len() == count {
                return bools;
            }
            bools.push((byte >> bit) & 1 != 0);
        }
    }
    bools
}

pub(crate) fn read_7bit_encoded_int(mut r: impl Read) -> io::Result<u32> {
    let mut result = 0u32;
    let mut bits_read = 0;

    loop {
        let mut buf = [0u8];
        r.read_exact(&mut buf)?;
        let byte = buf[0];

        result |= ((byte & 0x7F) as u32) << bits_read;
        bits_read += 7;

        if bits_read > 35 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too many bytes when decoding 7-bit int."));
        }

        if (byte & 0x80) == 0 {
            break;
        }
    }

    Ok(result)
}

pub(crate) fn read_string_7bit<R: Read>(mut r: R) -> io::Result<String> {
    let len = read_7bit_encoded_int(&mut r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
}


pub(crate) fn pack_color([r, g, b]: [u8; 3]) -> u16 {
    ((r & 0xF8) as u16) << 8 |
    ((g & 0xFC) as u16) << 2 |
    ((b & 0xF8) as u16) >> 3
}

pub(crate) fn unpack_color(rgb565: u16) -> [u8; 3] {
    [
        ((rgb565 >> 8) & 0xF8) as u8,
        ((rgb565 >> 2) & 0xFC) as u8,
        ((rgb565 << 3) & 0xF8) as u8,
    ]
}

pub trait WriteUtils: Write {
    fn write_array<T: Copy>(&mut self, array: &[T], f: impl Fn(&mut Self, &T) -> Result<()>) -> Result<()> {
        for value in array.iter() {
            f(self, value)?;
        }
        Ok(())
    }

    fn write_7bit_encoded_int(&mut self, mut value: u32) -> io::Result<()> {
        while value >= 0x80 {
            self.write_all(&[((value as u8 & 0x7F) | 0x80)])?;
            value >>= 7;
        }
        self.write_all(&[value as u8])?;
        Ok(())
    }

    fn write_string_7bit(&mut self, s: &str) -> io::Result<()> {
        self.write_7bit_encoded_int(s.len() as u32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }

    fn write_gradient<E: ByteOrder>(&mut self, g: &Gradient) -> io::Result<()> {
        let color_keys = g.color_keys.borrow();
        let alpha_keys = g.alpha_keys.borrow();
        for color_key in color_keys.iter() {
            self.write_array(&color_key.color, |w, &v| w.write_f32::<E>(v))?;
        }
        self.write_array(&alpha_keys, |w, v| w.write_f32::<E>(v.value))?;
        self.write_array(&alpha_keys, |w, v| w.write_f32::<E>(v.time))?;
        
        Ok(())
    }

    /// Writes array with length. Returns error if length is bigger than N max value.
    fn write_array_with_length<N: PrimInt + Unsigned + FromPrimitive, T: Copy>(
        &mut self,
        l: impl Fn(&mut Self, &N) -> Result<()>,
        f: impl Fn(&mut Self, &T) -> Result<()>,
        array: &[T]
    ) -> Result<()> {
        let len_n = N::from_usize(array.len())
            .ok_or_else(|| Error::new(ErrorKind::Other, "Array length too big for integer type"))?;
        l(self, &len_n)?;
        self.write_array(array, f)?;
        Ok(())
    }
}

impl<W: Write + ?Sized> WriteUtils for W {} 