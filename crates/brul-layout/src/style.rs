use crate::{AreaTemplate, GridLine, TrackSize};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Px(u32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Padding {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Padding {
    pub const fn all(value: u32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Flex,
    Grid,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
}

macro_rules! layout_patch {
    ($($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutStyle {
            $(pub $field: $ty,)+
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutPatch {
            $(pub $field: Option<$ty>,)+
        }

        impl LayoutPatch {
            pub const DEFAULT: Self = Self {
                $($field: None,)+
            };

            pub fn is_empty(&self) -> bool {
                *self == Self::DEFAULT
            }
        }

        impl From<&LayoutStyle> for LayoutPatch {
            fn from(style: &LayoutStyle) -> Self {
                let default = LayoutStyle::default();
                Self {
                    $($field: (style.$field != default.$field).then(|| style.$field.clone()),)+
                }
            }
        }

        impl LayoutStyle {
            pub fn apply(&mut self, patch: &LayoutPatch) {
                $(
                    if let Some(value) = &patch.$field {
                        self.$field = value.clone();
                    }
                )+
            }

            pub fn row_gap(&self) -> u32 {
                if self.row_gap == 0 { self.gap } else { self.row_gap }
            }

            pub fn column_gap(&self) -> u32 {
                if self.column_gap == 0 { self.gap } else { self.column_gap }
            }
        }
    };
}

layout_patch! {
    width: Dimension,
    height: Dimension,
    padding: Padding,
    border_width: u32,
    gap: u32,
    row_gap: u32,
    column_gap: u32,
    flex_grow: f32,
    flex_shrink: f32,
    flex_basis: Dimension,
    display: Display,
    flex_direction: Direction,
    justify: Justify,
    align: Align,
    grid_template_columns: Vec<TrackSize>,
    grid_template_rows: Vec<TrackSize>,
    grid_column: GridLine,
    grid_row: GridLine,
    grid_auto_flow: GridAutoFlow,
    grid_template_areas: AreaTemplate,
    grid_area: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollOffset(i32);

impl ScrollOffset {
    pub const ZERO: Self = Self(0);

    pub const fn new(pixels: i32) -> Self {
        Self(pixels)
    }

    pub const fn pixels(self) -> i32 {
        self.0
    }

    pub fn saturating_add(self, pixels: i32) -> Self {
        Self(self.0.saturating_add(pixels))
    }

    pub fn clamp(self, min: i32, max: i32) -> Self {
        Self(self.0.clamp(min, max))
    }
}

pub enum LayoutNode {
    Leaf {
        style: LayoutStyle,
        size: Size,
        offset: Option<Offset>,
    },
    Container {
        style: LayoutStyle,
        direction: Direction,
        children: Vec<LayoutNode>,
        offset: Option<Offset>,
        scroll_offset: ScrollOffset,
    },
}

impl LayoutNode {
    pub fn leaf(width: u32, height: u32) -> Self {
        Self::Leaf {
            style: LayoutStyle::default(),
            size: Size { width, height },
            offset: None,
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Row,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Column,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn with_style(mut self, style: LayoutStyle) -> Self {
        match &mut self {
            Self::Leaf { style: current, .. } | Self::Container { style: current, .. } => {
                *current = style
            }
        }
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        let offset = Some(Offset { x, y });
        match &mut self {
            Self::Leaf {
                offset: current, ..
            }
            | Self::Container {
                offset: current, ..
            } => *current = offset,
        }
        self
    }

    pub fn with_scroll(mut self, scroll: ScrollOffset) -> Self {
        if let Self::Container {
            scroll_offset: current,
            ..
        } = &mut self
        {
            *current = scroll;
        }
        self
    }

    pub(crate) fn style(&self) -> &LayoutStyle {
        match self {
            Self::Leaf { style, .. } | Self::Container { style, .. } => style,
        }
    }

    pub(crate) fn offset(&self) -> Option<Offset> {
        match self {
            Self::Leaf { offset, .. } | Self::Container { offset, .. } => *offset,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub children: Vec<LayoutBox>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LayoutError {
    #[error("invalid size: percent must be finite and non-negative")]
    InvalidDimension,
    #[error("invalid flex: factors and basis percent must be finite and non-negative")]
    InvalidFlex,
    #[error("invalid grid line: lines must be non-zero i16 and spans positive")]
    InvalidGridLine,
}

pub fn inset_x(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right)
}

pub fn inset_y(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.top)
        .saturating_add(style.padding.bottom)
}
