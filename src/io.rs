use std::io::{Read, Write};
use crate::error::Result;

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

pub struct Reader<'a> {
    inner: &'a mut dyn Read,
}

impl<'a> Reader<'a> {
    pub fn new(inner: &'a mut dyn Read) -> Self {
        Self { inner }
    }

    pub fn u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn i16(&mut self) -> Result<i16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn i32(&mut self) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn f32(&mut self) -> Result<f32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// C# BinaryWriter.Write(string): 7-bit LEB128 byte-length, then UTF-8 bytes.
    pub fn leb128_string(&mut self) -> Result<String> {
        let mut len: usize = 0;
        let mut shift = 0;
        loop {
            let b = self.u8()?;
            len |= ((b & 0x7F) as usize) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        let bytes = self.bytes(len)?;
        Ok(String::from_utf8(bytes)?)
    }

    /// Read a raw s4 array written via Buffer.BlockCopy (u1 count, then count*4 bytes).
    pub fn i32_array_u8_head(&mut self) -> Result<Vec<i32>> {
        let count = self.u8()? as usize;
        let bytes = self.bytes(count * 4)?;
        Ok(bytes
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
    }

    /// Read a raw s4 array written via Buffer.BlockCopy (u2 count, then count*4 bytes).
    pub fn i32_array_u16_head(&mut self) -> Result<Vec<i32>> {
        let count = self.u16()? as usize;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.i32()?);
        }
        Ok(values)
    }

    /// Read a raw f32 array written via Buffer.BlockCopy (u1 count, then count*4 bytes).
    pub fn f32_array_u8_head(&mut self) -> Result<Vec<f32>> {
        let count = self.u8()? as usize;
        let bytes = self.bytes(count * 4)?;
        Ok(bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
    }

    /// Read a bool array written via Buffer.BlockCopy (u1 count, 1 byte per bool).
    pub fn bool_array_u8_head(&mut self) -> Result<Vec<bool>> {
        let count = self.u8()? as usize;
        let bytes = self.bytes(count)?;
        Ok(bytes.into_iter().map(|b| b != 0).collect())
    }

    /// Read a standard bool array (u2 count, 1 byte per bool).
    pub fn bool_array_u16_head(&mut self) -> Result<Vec<bool>> {
        let count = self.u16()? as usize;
        let bytes = self.bytes(count)?;
        Ok(bytes.into_iter().map(|b| b != 0).collect())
    }

    /// Read packed bools (version >= 5): u1 count, then ceil(count/8) bytes MSB-first.
    pub fn packed_bool_u8_head(&mut self) -> Result<Vec<bool>> {
        let count = self.u8()? as usize;
        if count == 0 {
            return Ok(vec![]);
        }
        let byte_count = (count + 7) / 8;
        let packed = self.bytes(byte_count)?;
        let mut result = Vec::with_capacity(count);
        for b in packed {
            for bit in (0..8).rev() {
                result.push((b >> bit) & 1 != 0);
                if result.len() == count {
                    break;
                }
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

pub struct Writer<'a> {
    inner: &'a mut dyn Write,
}

impl<'a> Writer<'a> {
    pub fn new(inner: &'a mut dyn Write) -> Self {
        Self { inner }
    }

    pub fn u8(&mut self, v: u8) -> Result<()> {
        self.inner.write_all(&[v])?;
        Ok(())
    }

    pub fn i8(&mut self, v: i8) -> Result<()> {
        self.u8(v as u8)
    }

    pub fn u16(&mut self, v: u16) -> Result<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn i16(&mut self, v: i16) -> Result<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn u32(&mut self, v: u32) -> Result<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn i32(&mut self, v: i32) -> Result<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    pub fn f32(&mut self, v: f32) -> Result<()> {
        self.inner.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    /// C# BinaryWriter.Write(string): 7-bit LEB128 byte-length, then UTF-8 bytes.
    pub fn leb128_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        let mut len = bytes.len();
        loop {
            let mut b = (len & 0x7F) as u8;
            len >>= 7;
            if len > 0 {
                b |= 0x80;
            }
            self.u8(b)?;
            if len == 0 {
                break;
            }
        }
        self.inner.write_all(bytes)?;
        Ok(())
    }

    /// Write a raw s4 array via Buffer.BlockCopy style (u1 count, then count*4 bytes).
    pub fn i32_array_u8_head(&mut self, values: &[i32]) -> Result<()> {
        self.u8(values.len() as u8)?;
        for &v in values {
            self.i32(v)?;
        }
        Ok(())
    }

    /// Write a standard s4 array (u2 count, then one s4 per item).
    pub fn i32_array_u16_head(&mut self, values: &[i32]) -> Result<()> {
        self.u16(values.len() as u16)?;
        for &v in values {
            self.i32(v)?;
        }
        Ok(())
    }

    /// Write a raw f32 array via Buffer.BlockCopy style (u1 count, then count*4 bytes).
    pub fn f32_array_u8_head(&mut self, values: &[f32]) -> Result<()> {
        self.u8(values.len() as u8)?;
        for &v in values {
            self.f32(v)?;
        }
        Ok(())
    }

    /// Write a standard f32 array (u2 count, then one f32 per item).
    pub fn f32_array_u16_head(&mut self, values: &[f32]) -> Result<()> {
        self.u16(values.len() as u16)?;
        for &v in values {
            self.f32(v)?;
        }
        Ok(())
    }

    /// Write a bool array via Buffer.BlockCopy style (u1 count, 1 byte per bool).
    pub fn bool_array_u8_head(&mut self, values: &[bool]) -> Result<()> {
        self.u8(values.len() as u8)?;
        for &v in values {
            self.u8(v as u8)?;
        }
        Ok(())
    }

    /// Write a standard bool array (u2 count, 1 byte per bool).
    pub fn bool_array_u16_head(&mut self, values: &[bool]) -> Result<()> {
        self.u16(values.len() as u16)?;
        for &v in values {
            self.u8(v as u8)?;
        }
        Ok(())
    }

    /// Write packed bools (version >= 5): u1 count, then ceil(count/8) bytes MSB-first.
    pub fn packed_bool_u8_head(&mut self, values: &[bool]) -> Result<()> {
        self.u8(values.len() as u8)?;
        let mut idx = 0;
        while idx < values.len() {
            let mut byte = 0u8;
            for bit in (0..8).rev() {
                if idx < values.len() && values[idx] {
                    byte |= 1 << bit;
                }
                idx += 1;
            }
            self.u8(byte)?;
        }
        Ok(())
    }
}
