#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackedColor(u16);

impl PackedColor { 
    // TODO: make it actually convert to something.
    pub fn pack_from_u8rgb(r: u8, g: u8, b: u8) -> Self {
        return Self(0);
    }
    pub fn pack_from_u8x3(data: [u8; 3]) -> Self {
        return Self(0);
    }
    pub fn pack_from_floatrgb(r: f32, g: f32, b: f32) -> Self {
        return Self(0);
    }
    pub fn unpack_to_u8x3() -> [u8; 3] {
        return [0, 0, 0];
    }
    pub fn unpack_to_floatx3() -> [f32; 3] {
        return [0.0, 0.0, 0.0];
    }
}