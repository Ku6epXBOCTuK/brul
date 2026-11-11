#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub fn lerp(self, to: Self, progress: f32) -> Self {
        Self {
            r: lerp_channel(self.r, to.r, progress),
            g: lerp_channel(self.g, to.g, progress),
            b: lerp_channel(self.b, to.b, progress),
            a: lerp_channel(self.a, to.a, progress),
        }
    }

    pub fn multiply(self, other: Self) -> Self {
        Self {
            r: multiply_channel(self.r, other.r),
            g: multiply_channel(self.g, other.g),
            b: multiply_channel(self.b, other.b),
            a: multiply_channel(self.a, other.a),
        }
    }
}

fn multiply_channel(left: u8, right: u8) -> u8 {
    ((u16::from(left) * u16::from(right) + 127) / 255) as u8
}

fn lerp_channel(from: u8, to: u8, progress: f32) -> u8 {
    let from = f32::from(from);
    let to = f32::from(to);
    (from + (to - from) * progress.clamp(0.0, 1.0)).round() as u8
}
