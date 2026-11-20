use brul_style::Color as StyleColor;

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

    pub fn mix(self, to: Self, progress: f32) -> Self {
        brul_style::mix(
            self.into(),
            to.into(),
            f64::from(progress.clamp(0.0, 1.0)) * 100.0,
        )
        .into()
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

impl From<Color> for StyleColor {
    fn from(value: Color) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

impl From<StyleColor> for Color {
    fn from(value: StyleColor) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

fn multiply_channel(left: u8, right: u8) -> u8 {
    ((u16::from(left) * u16::from(right) + 127) / 255) as u8
}
