const ROTATION_MULTIPLIER: f32 = (u16::MAX as f32) / 360.0f32;
const ROTATION_INV: f32 = 360.0 / (u16::MAX as f32);

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!("[DEBUG] {}", format!($($arg)*));
        }
    };
}

pub fn pack_rotation(data: [f32; 3]) -> [u16; 3] {
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

pub fn unpack_rotation(data: [u16; 3]) -> [f32; 3] {
    [
        (data[0] as f32) * ROTATION_INV,
        (data[1] as f32) * ROTATION_INV,
        (data[2] as f32) * ROTATION_INV,
    ]
}

#[inline(always)]
pub fn float_to_bounds(f: f32, offset: f32, scale: f32) -> i16 {
    ((f - offset) * scale).round() as i16
}

#[inline(always)]
pub fn bounds_to_float(t: i16, offset: f32, scale: f32) -> f32 {
    (t as f32) / scale + offset
}

#[inline(always)]
pub fn new_bounds() -> [[f32; 3]; 2] {
    [
        [f32::INFINITY; 3],
        [f32::NEG_INFINITY; 3]
    ]
}

#[inline(always)]
pub fn bounds_encapsulate(bounds: &mut [[f32; 3]; 2], block_position: [f32; 3]) {
    for i in 0..3 {
        bounds[0][i] = bounds[0][i].min(block_position[i]);
        bounds[1][i] = bounds[1][i].max(block_position[i]);
    }
}

#[inline(always)]
pub fn bounds_center_and_size(bounds: &[[f32; 3]; 2]) -> ([f32; 3], [f32; 3]) {
    let mut center = [0.0f32; 3];
    let mut size = [0.0f32; 3];

    for i in 0..3 {
        center[i] = (bounds[0][i] + bounds[1][i]) * 0.5;
        size[i] = bounds[1][i] - bounds[0][i];
    }

    (center, size)
}

pub fn pack_bools(bools: &[bool]) -> Vec<u8> {
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

pub fn unpack_bools(bytes: &[u8], count: usize) -> Vec<bool> {
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

pub fn write_7bit_encoded_int(mut w: impl Write, mut value: u32) -> io::Result<()> {
    while value >= 0x80 {
        w.write_all(&[((value as u8 & 0x7F) | 0x80)])?;
        value >>= 7;
    }
    w.write_all(&[value as u8])?;
    Ok(())
}

pub fn read_7bit_encoded_int(mut r: impl Read) -> io::Result<u32> {
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

pub fn pack_color(data: [u8; 3]) -> u16 {
    0
}

pub fn unpack_color(data: u16) -> [u8; 3] {
    [0, 0, 0]
}