use brul_event::{Event, RawEvent};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DispatchEvent {
    Raw(RawUiEvent),
    Semantic(UiEvent),
}

impl DispatchEvent {
    pub fn target(&self) -> EventTarget {
        match self {
            Self::Raw(event) => event.target,
            Self::Semantic(event) => event.target,
        }
    }

    pub fn raw(id: WidgetId, event: RawEvent) -> Self {
        Self::Raw(RawUiEvent::widget(id, event))
    }

    pub fn semantic(id: WidgetId, event: Event) -> Self {
        Self::Semantic(UiEvent::widget(id, event))
    }
}

impl From<RawUiEvent> for DispatchEvent {
    fn from(event: RawUiEvent) -> Self {
        Self::Raw(event)
    }
}

impl From<UiEvent> for DispatchEvent {
    fn from(event: UiEvent) -> Self {
        Self::Semantic(event)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawUiEvent {
    pub target: EventTarget,
    pub event: RawEvent,
}

impl RawUiEvent {
    pub fn window(event: RawEvent) -> Self {
        Self {
            target: EventTarget::Window,
            event,
        }
    }

    pub fn widget(id: WidgetId, event: RawEvent) -> Self {
        Self {
            target: EventTarget::Widget(id),
            event,
        }
    }

    pub fn at(target: EventTarget, event: RawEvent) -> Self {
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
