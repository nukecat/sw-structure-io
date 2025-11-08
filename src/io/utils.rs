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

pub(crate) fn float_to_bounds(f: f32, offset: f32, scale: f32) -> i16 {
    let multiplier = (1.0f32 / scale) * i16::MAX as f32;
    ((f - offset) * multiplier).round() as i16
}

pub(crate) fn bounds_to_float(t: i16, offset: f32, scale: f32) -> f32 {
    let multiplier = (1.0f32 / scale) * i16::MAX as f32;
    (t as f32) / multiplier + offset
}

pub(crate) fn f32x3_to_bounds(f: [f32; 3], offset: [f32; 3], scale: [f32; 3]) -> [i16; 3] {
    let mut result = [0i16; 3];
    for i in 0..3 {
        result[i] = float_to_bounds(f[i], offset[i], scale[i]);
    }
    result
}

pub(crate) fn new_bounds() -> [[f32; 3]; 2] {
    [
        [f32::INFINITY; 3],
        [f32::NEG_INFINITY; 3]
    ]
}

pub(crate) fn bounds_encapsulate(bounds: &mut [[f32; 3]; 2], block_position: [f32; 3]) {
    for i in 0..3 {
        bounds[0][i] = bounds[0][i].min(block_position[i]);
        bounds[1][i] = bounds[1][i].max(block_position[i]);
    }
}

pub(crate) fn bounds_center_and_size(bounds: &[[f32; 3]; 2]) -> ([f32; 3], [f32; 3]) {
    let mut center = [0.0f32; 3];
    let mut size = [0.0f32; 3];

    for i in 0..3 {
        center[i] = (bounds[0][i] + bounds[1][i]) * 0.5;
        size[i] = (bounds[1][i] - bounds[0][i])
    }

    (center, size)
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


pub(crate) fn pack_color(data: [u8; 3]) -> u16 {
    0
}

pub(crate) fn unpack_color(data: u16) -> [u8; 3] {
    [0, 0, 0]
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
}

impl<W: Write + ?Sized> WriteUtils for W {} 