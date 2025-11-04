pub struct ColorKey {
    time: f32,
    color: [f32; 4]
}

pub struct AlphaKey {
    time: f32,
    alpha: f32
}

pub struct Gradient {
    color_keys: Vec<ColorKey>,
    alpha_keys: Vec<AlphaKey>
}