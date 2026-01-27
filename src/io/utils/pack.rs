const ROTATION_MULTIPLIER: f32 = (u16::MAX as f32) / 360.0f32;
const ROTATION_INV: f32 = 360.0 / (u16::MAX as f32);

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
