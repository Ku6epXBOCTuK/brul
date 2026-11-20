use crate::{Color, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Color(Color),
    Gradient(Gradient),
}

impl Default for Paint {
    fn default() -> Self {
        Self::Color(Color::TRANSPARENT)
    }
}

impl Paint {
    pub fn at(&self, rect: &Rect, x: i32, y: i32) -> Color {
        match self {
            Self::Color(color) => *color,
            Self::Gradient(gradient) => gradient.at(rect, x, y),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gradient {
    Linear { angle: f32, from: Color, to: Color },
}

impl Gradient {
    pub fn at(&self, rect: &Rect, x: i32, y: i32) -> Color {
        let Self::Linear { angle, from, to } = *self;
        let (sin, cos) = angle.to_radians().sin_cos();
        let width = rect.width as f32;
        let height = rect.height as f32;
        let length = (width * sin.abs() + height * cos.abs()).max(f32::EPSILON);
        let center_x = rect.x as f32 + width / 2.0;
        let center_y = rect.y as f32 + height / 2.0;
        let dx = x as f32 + 0.5 - center_x;
        let dy = y as f32 + 0.5 - center_y;

        from.mix(to, ((dx * sin - dy * cos) / length + 0.5).clamp(0.0, 1.0))
    }
}
