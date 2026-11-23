use brul_event::Event;

use super::WidgetId;
use crate::layout::LayoutError;
use crate::text::TextError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaretArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventTarget {
    Window,
    Widget(WidgetId),
}

impl EventTarget {
    pub fn chain(self) -> impl Iterator<Item = Self> {
        match self {
            Self::Window => vec![Self::Window].into_iter(),
            Self::Widget(id) => vec![Self::Widget(id), Self::Window].into_iter(),
        }
    }

    pub fn widget(self) -> Option<WidgetId> {
        match self {
            Self::Window => None,
            Self::Widget(id) => Some(id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiEvent {
    pub target: EventTarget,
    pub event: Event,
}

impl UiEvent {
    pub fn window(event: Event) -> Self {
        Self {
            target: EventTarget::Window,
            event,
        }
    }

    pub fn widget(id: WidgetId, event: Event) -> Self {
        Self {
            target: EventTarget::Widget(id),
            event,
        }
    }

    pub fn at(target: EventTarget, event: Event) -> Self {
        Self { target, event }
    }
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
