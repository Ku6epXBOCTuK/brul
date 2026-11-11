use crate::{Color, Paint};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub fill: Paint,
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
            fill: Paint::Color(Color::TRANSPARENT),
            border_width: 0,
            border_color: Color::TRANSPARENT,
        }
    }

    pub const fn with_radius(mut self, radius: u32) -> Self {
        self.radius = radius;
        self
    }

    pub const fn with_fill(mut self, fill: Paint) -> Self {
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
            RectSample::Fill(self.fill.at(self, x, y))
        }
    }

    fn distance(&self, x: f32, y: f32) -> f32 {
        rounded_distance(
            x - self.x as f32,
            y - self.y as f32,
            self.width,
            self.height,
            self.radius,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shadow {
    pub x: i32,
    pub y: i32,
    pub blur: u32,
    pub color: Color,
}

impl Shadow {
    pub(crate) fn coverage(&self, x: i32, y: i32, rect: &Rect) -> f32 {
        if rect.width == 0 || rect.height == 0 || self.color.a == 0 {
            return 0.0;
        }
        let distance = rounded_distance(
            x as f32 + 0.5 - rect.x.saturating_add(self.x) as f32,
            y as f32 + 0.5 - rect.y.saturating_add(self.y) as f32,
            rect.width,
            rect.height,
            rect.radius,
        );
        if self.blur == 0 {
            return if distance <= 0.0 { 1.0 } else { 0.0 };
        }
        let t = (0.5 - distance / (2.0 * self.blur as f32)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub radius: u32,
}

impl ClipRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32, radius: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            radius,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        if self.width == 0 || self.height == 0 {
            return false;
        }
        rounded_distance(
            x as f32 + 0.5 - self.x as f32,
            y as f32 + 0.5 - self.y as f32,
            self.width,
            self.height,
            self.radius,
        ) <= 0.0
    }
}

fn rounded_distance(x: f32, y: f32, width: u32, height: u32, radius: u32) -> f32 {
    let radius = radius.min(width.min(height) / 2) as f32;
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;

    let qx = (x - half_width).abs() - (half_width - radius);
    let qy = (y - half_height).abs() - (half_height - radius);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    let inside = qx.max(qy).min(0.0);

    outside + inside - radius
}
