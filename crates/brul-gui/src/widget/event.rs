use std::sync::Arc;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyPress {
    pub code: String,
    pub physical: Option<String>,
}

impl From<&str> for KeyPress {
    fn from(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            physical: None,
        }
    }
}

impl From<String> for KeyPress {
    fn from(code: String) -> Self {
        Self {
            code,
            physical: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaretArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerOwner(u64);

impl TimerOwner {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
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
        key: KeyPress,
        modifiers: KeyModifiers,
    },
    Close,
    Timer(TimerOwner, Arc<str>),
    Focus(WidgetId),
    Blur(WidgetId),
    Input(WidgetId, String, usize),
    Submit(WidgetId),
    Dismiss(WidgetId),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WidgetError {
    #[error("widget tree is not laid out yet")]
    NoLayout,
    #[error("invalid DPI scale")]
    InvalidScale,
    #[error("layout: {0}")]
    Layout(#[from] LayoutError),
    #[error("text: {0}")]
    Text(#[from] TextError),
}
