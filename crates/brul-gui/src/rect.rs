use crate::{BorderWidths, Color, Paint};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub fill: Paint,
    pub border_width: BorderWidths,
    pub border_color: Color,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectSample {
    pub fill_coverage: f32,
    pub border_coverage: f32,
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
            border_width: BorderWidths::all(0),
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

    pub const fn with_border(mut self, widths: BorderWidths, color: Color) -> Self {
        self.border_width = widths;
        self.border_color = color;
        self
    }

    pub fn sample(&self, x: i32, y: i32) -> RectSample {
        if self.width == 0 || self.height == 0 {
            return RectSample::default();
        }

        let outer = (0.5 - self.distance(x, y)).clamp(0.0, 1.0);
        let widths = self.border_width;
        if widths == BorderWidths::all(0) {
            return RectSample {
                fill_coverage: outer,
                border_coverage: 0.0,
            };
        }

        let inner_width = self
            .width
            .saturating_sub(widths.left.saturating_add(widths.right));
        let inner_height = self
            .height
            .saturating_sub(widths.top.saturating_add(widths.bottom));
        if inner_width == 0 || inner_height == 0 {
            return RectSample {
                fill_coverage: 0.0,
                border_coverage: outer,
            };
        }
        let inner_radius = self.radius.saturating_sub(
            widths
                .left
                .max(widths.top)
                .max(widths.right)
                .max(widths.bottom),
        );
        let inner_distance = rounded_distance(
            x,
            y,
            self.x.saturating_add(widths.left as i32),
            self.y.saturating_add(widths.top as i32),
            inner_width,
            inner_height,
            inner_radius,
        );
        let inner = (0.5 - inner_distance).clamp(0.0, 1.0);
        RectSample {
            fill_coverage: inner,
            border_coverage: outer - inner,
        }
    }

    fn distance(&self, x: i32, y: i32) -> f32 {
        rounded_distance(x, y, self.x, self.y, self.width, self.height, self.radius)
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
            x,
            y,
            rect.x.saturating_add(self.x),
            rect.y.saturating_add(self.y),
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
        rounded_distance(x, y, self.x, self.y, self.width, self.height, self.radius) <= 0.0
    }
}

fn rounded_distance(
    x: i32,
    y: i32,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    radius: u32,
) -> f32 {
    let radius = radius.min(width.min(height) / 2);
    if radius == 0 {
        let dx = (i64::from(x) - i64::from(left)) * 2 + 1;
        let dy = (i64::from(y) - i64::from(top)) * 2 + 1;
        let qx = (dx - i64::from(width)).abs() - i64::from(width);
        let qy = (dy - i64::from(height)).abs() - i64::from(height);
        return if qx > 0 && qy > 0 {
            ((qx as f32) * (qx as f32) + (qy as f32) * (qy as f32)).sqrt() / 2.0
        } else {
            qx.max(qy) as f32 / 2.0
        };
    }

    let x = (i64::from(x) - i64::from(left)) as f32 + 0.5;
    let y = (i64::from(y) - i64::from(top)) as f32 + 0.5;
    let radius = radius as f32;
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;

    let qx = (x - half_width).abs() - (half_width - radius);
    let qy = (y - half_height).abs() - (half_height - radius);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    let inside = qx.max(qy).min(0.0);

    outside + inside - radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_radius_distance_uses_axis_aligned_geometry() {
        let rect = Rect::new(0, 0, 10, 10);
        assert_eq!(rect.distance(5, 5), -4.5);
        assert_eq!(rect.distance(0, 5), -0.5);
        assert_eq!(rect.distance(-1, 5), 0.5);
        let corner = rect.distance(-1, -1);
        assert!((corner - 2.0_f32.sqrt() / 2.0).abs() < 1e-6);
    }

    #[test]
    fn clamped_zero_radius_uses_the_same_path() {
        let thin = Rect::new(0, 0, 1, 10).with_radius(8);
        assert_eq!(thin.distance(0, 5), -0.5);
        assert_eq!(thin.distance(-1, 5), 0.5);
    }
}
