use crate::types::*;
use std::collections::HashSet;

pub const CUSTOM_BLOCK_IDS: &[u8] = &[
    109, 120, 121, 130
];

pub const NON_INTERACTABLE_BLOCK_IDS: &[u8] = &[
    00, 01, 28, 33, 34, 35, 36, 37, 38,
    59, 62, 63, 64, 65, 66, 67, 68, 69,
    70, 71, 72, 73, 74, 75, 86, 87, 88
];

// ---------------------------------------------------------------------------
// BuildingContext
// ---------------------------------------------------------------------------

/// Holds all version-specific and lookup-table state needed to read/write
/// a building. Passed by reference into every read/write function so that
/// individual functions don't need to carry these as separate arguments.
pub struct BuildingContext {
    pub version: u8,

    // Rotation lookup (version 6+ only)
    pub rotation_table: Vec<RawRotation>,
    pub single_byte_rot: bool,

    // Color lookup (version 6+ only)
    pub color_table: Vec<u16>, // RGB565 values

    // Precomputed sets for fast lookup
    custom_ids: HashSet<u8>,
    non_interactable_ids: HashSet<u8>,
}

impl BuildingContext {
    /// Create a context for reading. Call after reading the lookup tables
    /// from the stream.
    pub fn new(
        version: u8,
        color_table: Vec<u16>,
        rotation_table: Vec<RawRotation>,
        single_byte_rot: bool,
    ) -> Self {
        Self {
            version,
            color_table,
            rotation_table,
            single_byte_rot,
            custom_ids: CUSTOM_BLOCK_IDS.iter().copied().collect(),
            non_interactable_ids: NON_INTERACTABLE_BLOCK_IDS.iter().copied().collect(),
        }
    }

    /// Create a context for writing at the latest version (6).
    /// Lookup tables are populated during the write pre-pass.
    pub fn for_writing(
        version: u8,
        color_table: Vec<u16>,
        rotation_table: Vec<RawRotation>,
        single_byte_rot: bool,
    ) -> Self {
        Self::new(version, color_table, rotation_table, single_byte_rot)
    }

    pub fn is_custom_block(&self, block_id: u8) -> bool {
        self.custom_ids.contains(&block_id)
    }

    pub fn is_non_interactable(&self, block_id: u8) -> bool {
        self.non_interactable_ids.contains(&block_id)
    }

    pub fn use_color_lookup(&self) -> bool {
        self.version > 5 && !self.color_table.is_empty()
    }

    pub fn use_rotation_lookup(&self) -> bool {
        self.version > 5 && !self.rotation_table.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub center: Vec3,
    pub size: Vec3,
}

impl Bounds {
    pub fn from_points(points: impl Iterator<Item = Vec3>) -> Option<Self> {
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        let mut any = false;

        for p in points {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
            any = true;
        }

        if !any {
            return None;
        }

        let size = Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z);
        let center = Vec3::new(
            (min.x + max.x) / 2.0,
                               (min.y + max.y) / 2.0,
                               (min.z + max.z) / 2.0,
        );

        Some(Self { center, size })
    }

    pub fn pos_to_i16(&self, pos: Vec3) -> [i16; 3] {
        let mult_x = (1.0 / self.size.x) * i16::MAX as f32;
        let mult_y = (1.0 / self.size.y) * i16::MAX as f32;
        let mult_z = (1.0 / self.size.z) * i16::MAX as f32;
        [
            ((pos.x - self.center.x) * mult_x) as i16,
            ((pos.y - self.center.y) * mult_y) as i16,
            ((pos.z - self.center.z) * mult_z) as i16,
        ]
    }

    pub fn i16_to_pos(&self, raw: [i16; 3]) -> Vec3 {
        let mult_x = (1.0 / self.size.x) * i16::MAX as f32;
        let mult_y = (1.0 / self.size.y) * i16::MAX as f32;
        let mult_z = (1.0 / self.size.z) * i16::MAX as f32;
        Vec3::new(
            raw[0] as f32 / mult_x + self.center.x,
            raw[1] as f32 / mult_y + self.center.y,
            raw[2] as f32 / mult_z + self.center.z,
        )
    }
}

/// Packed rotation (raw ushort triple, as stored in the rotation lookup table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawRotation(pub u16, pub u16, pub u16);

const ROT_MULT: f32 = 65535.0 / 360.0;

impl RawRotation {
    pub fn from_degrees(x: f32, y: f32, z: f32) -> Self {
        Self(
            (x * ROT_MULT) as u16,
             (y * ROT_MULT) as u16,
             (z * ROT_MULT) as u16,
        )
    }

    pub fn to_degrees(self) -> Vec3 {
        Vec3::new(
            self.0 as f32 / ROT_MULT,
            self.1 as f32 / ROT_MULT,
            self.2 as f32 / ROT_MULT,
        )
    }
}

impl Color {
    pub fn from_rgba_bytes(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub fn to_rgba_bytes(self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    /// Decode from packed RGB565 (no alpha; alpha set to 1.0).
    pub fn from_rgb565(raw: u16) -> Self {
        let r5 = ((raw >> 11) & 0x1F) as u8;
        let g6 = ((raw >> 5) & 0x3F) as u8;
        let b5 = (raw & 0x1F) as u8;
        // Expand to 8-bit: replicate upper bits into lower bits
        let r = (r5 << 3) | (r5 >> 2);
        let g = (g6 << 2) | (g6 >> 4);
        let b = (b5 << 3) | (b5 >> 2);
        Self::from_rgba_bytes(r, g, b, 255)
    }

    /// Encode to packed RGB565 (alpha is ignored).
    pub fn to_rgb565(self) -> u16 {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let r5 = (r >> 3) as u16;
        let g6 = (g >> 2) as u16;
        let b5 = (b >> 3) as u16;
        (r5 << 11) | (g6 << 5) | b5
    }
}
