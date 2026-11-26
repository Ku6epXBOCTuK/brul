use brul_style::Side;

use crate::{BorderColors, BorderWidths, Color};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Border {
    pub widths: BorderWidths,
    pub colors: BorderColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderArrays {
    pub widths: [f32; 4],
    pub left: [f32; 4],
    pub top: [f32; 4],
    pub right: [f32; 4],
    pub bottom: [f32; 4],
}

impl Border {
    pub const fn new(widths: BorderWidths, colors: BorderColors) -> Self {
        Self { widths, colors }
    }

    pub const fn empty() -> Self {
        Self {
            widths: BorderWidths::all(0),
            colors: BorderColors::all(Color::TRANSPARENT),
        }
    }

    pub fn visible(self) -> bool {
        Side::ALL
            .into_iter()
            .any(|side| self.widths[side] > 0 && self.colors[side].a > 0)
    }

    pub fn flatten(self) -> BorderArrays {
        BorderArrays {
            widths: [
                self.widths.left as f32,
                self.widths.top as f32,
                self.widths.right as f32,
                self.widths.bottom as f32,
            ],
            left: rgba(self.colors.left),
            top: rgba(self.colors.top),
            right: rgba(self.colors.right),
            bottom: rgba(self.colors.bottom),
        }
    }
}

fn rgba(color: Color) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::Border;
    use crate::{BorderColors, BorderWidths, Color};

    #[test]
    fn visibility_requires_both_width_and_alpha() {
        let color = Color::rgb(0, 0, 255);
        assert!(Border::new(BorderWidths::all(1), BorderColors::all(color)).visible());
        assert!(!Border::new(BorderWidths::all(0), BorderColors::all(color)).visible());
        assert!(
            !Border::new(BorderWidths::all(1), BorderColors::all(Color::TRANSPARENT)).visible()
        );
    }

    #[test]
    fn flatten_keeps_side_order() {
        let border = Border::new(
            BorderWidths::new(1, 2, 3, 4),
            BorderColors::new(
                Color::rgb(10, 0, 0),
                Color::rgb(20, 0, 0),
                Color::rgb(30, 0, 0),
                Color::rgb(40, 0, 0),
            ),
        );
        let arrays = border.flatten();

        assert_eq!(arrays.widths, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(arrays.left[0], 10.0 / 255.0);
        assert_eq!(arrays.bottom[0], 40.0 / 255.0);
    }
}
