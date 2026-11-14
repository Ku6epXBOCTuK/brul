use std::fmt;

use super::WidgetId;
use crate::layout::LayoutError;
use crate::text::TextError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaretArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidgetEvent {
    PointerEnter(WidgetId),
    PointerLeave(WidgetId),
    Press(WidgetId),
    Release(WidgetId),
    Click(WidgetId),
    Drag(WidgetId),
    Key {
        code: String,
        modifiers: KeyModifiers,
    },
    Close,
    Timer(String),
    Focus(WidgetId),
    Blur(WidgetId),
    Input(WidgetId, String, usize),
    Submit(WidgetId),
    Copy(String),
    Paste,
}

#[derive(Clone, Debug, Default)]
pub struct Dirty {
    pub full: bool,
    pub ids: Vec<WidgetId>,
}

impl Dirty {
    pub fn all() -> Self {
        Self {
            full: true,
            ids: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.ids.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetError {
    NoLayout,
    InvalidScale,
    Layout(LayoutError),
    Text(TextError),
}

impl fmt::Display for WidgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLayout => formatter.write_str("widget tree is not laid out yet"),
            Self::InvalidScale => formatter.write_str("invalid DPI scale"),
            Self::Layout(error) => write!(formatter, "layout: {error}"),
            Self::Text(error) => write!(formatter, "text: {error}"),
        }
    }
}

impl std::error::Error for WidgetError {}

impl From<LayoutError> for WidgetError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<TextError> for WidgetError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}
