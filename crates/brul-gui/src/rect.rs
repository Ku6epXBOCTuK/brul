use crate::Color;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub fill: Color,
    pub border_width: u32,
    pub border_color: Color,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RectSample {
    Outside,
    Fill(Color),
    Border(Color),
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            radius: 0,
            fill: Color::TRANSPARENT,
            border_width: 0,
            border_color: Color::TRANSPARENT,
        }
    }

    pub const fn with_radius(mut self, radius: u32) -> Self {
        self.radius = radius;
        self
    }

    pub const fn with_fill(mut self, fill: Color) -> Self {
        self.fill = fill;
        self
    }

    pub const fn with_border(mut self, width: u32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn sample(&self, x: i32, y: i32) -> RectSample {
        if self.width == 0 || self.height == 0 {
            return RectSample::Outside;
        }

        let distance = self.distance(x as f32 + 0.5, y as f32 + 0.5);
        if distance > 0.0 {
            RectSample::Outside
        } else if self.border_width > 0 && distance > -(self.border_width as f32) {
            RectSample::Border(self.border_color)
        } else {
            RectSample::Fill(self.fill)
        }
    }

    fn distance(&self, x: f32, y: f32) -> f32 {
        let radius = self.radius.min(self.width.min(self.height) / 2) as f32;
        let half_width = self.width as f32 / 2.0;
        let half_height = self.height as f32 / 2.0;
        let center_x = self.x as f32 + half_width;
        let center_y = self.y as f32 + half_height;

        let qx = (x - center_x).abs() - (half_width - radius);
        let qy = (y - center_y).abs() - (half_height - radius);
        let outside = qx.max(0.0).hypot(qy.max(0.0));
        let inside = qx.max(qy).min(0.0);

        outside + inside - radius
    }
}
