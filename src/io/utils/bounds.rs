#[derive(Clone, Debug)]
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
