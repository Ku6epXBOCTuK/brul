use std::sync::atomic::{AtomicU32, Ordering};

use crate::image::Image;
use crate::layout::{Direction, LayoutPatch, LayoutStyle, Offset};
use crate::text::TextStyle;

static NEXT_WIDGET_ID: AtomicU32 = AtomicU32::new(1);

macro_rules! patch {
    ($vis:vis $name:ident { $($field:ident),+ $(,)? }) => {
        impl $name {
            pub const DEFAULT: Self = Self {
                $($field: None,)+
            };

            $vis fn merge(&mut self, other: &Self) {
                $(
                    if other.$field.is_some() {
                        self.$field = other.$field;
                    }
                )+
            }
        }
    };
}

fn set_if_some<T>(dst: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *dst = value;
    }
}

mod event;
mod snapshot;
mod style;
#[cfg(test)]
mod tests;
mod text;
mod tree;

pub use brul_ident::{FunctionName, TimerOwner};
pub use event::{CaretArea, Dirty, KeyModifiers, KeyPress, WidgetError, WidgetEvent};
pub use snapshot::{WidgetSnapshot, WidgetSnapshotNode};
pub use style::{Class, Layers, StyleSet, StyleState, Transitions, WidgetStyle, WidgetStylePatch};
pub use text::{TextStylePatch, TextStyleSet};
pub use tree::WidgetTree;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WidgetId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetKind {
    View,
    Structure,
    Text,
    Image,
    Input,
}

macro_rules! define_widget_state {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct WidgetState {
            $(pub $field: bool,)+
            pub visible: bool,
        }

        impl Default for WidgetState {
            fn default() -> Self {
                Self {
                    $($field: false,)+
                    visible: true,
                }
            }
        }
    };
}

brul_style::states!(define_widget_state);

pub struct Widget {
    id: WidgetId,
    data: Content,
    children: Vec<Widget>,
    layout: LayoutStyle,
    styles: StyleSet,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
    focusable: bool,
    layer: bool,
    modal: bool,
}

enum Content {
    View,
    Structure,
    Text {
        content: String,
        styles: Box<TextStyleSet>,
    },
    Input {
        value: String,
        styles: Box<TextStyleSet>,
        caret: usize,
        anchor: usize,
        multiline: bool,
        scroll: i32,
        preedit: String,
    },
    Image(Image),
}

impl Content {
    fn kind(&self) -> WidgetKind {
        match self {
            Self::View => WidgetKind::View,
            Self::Structure => WidgetKind::Structure,
            Self::Text { .. } => WidgetKind::Text,
            Self::Input { .. } => WidgetKind::Input,
            Self::Image(_) => WidgetKind::Image,
        }
    }

    fn is_group(&self) -> bool {
        matches!(self, Self::View | Self::Structure)
    }
}

impl Widget {
    pub fn view(direction: Direction, children: Vec<Self>) -> Self {
        let mut widget = Self::new(Content::View);
        widget.layout.flex_direction = direction;
        widget.children = children;
        widget
    }

    pub fn structure(children: Vec<Self>) -> Self {
        let mut widget = Self::new(Content::Structure);
        widget.children = children;
        widget
    }

    pub fn set_children(&mut self, children: Vec<Self>) {
        if self.data.is_group() {
            self.children = children;
        }
    }

    pub fn into_children(self) -> Vec<Self> {
        if self.data.is_group() {
            self.children
        } else {
            vec![self]
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::view(Direction::Row, children)
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::view(Direction::Column, children)
    }

    pub fn layer(children: Vec<Self>) -> Self {
        let mut widget = Self::column(children);
        widget.layer = true;
        widget
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self::styled_text(content, TextStyle::default())
    }

    pub fn styled_text(content: impl Into<String>, style: TextStyle) -> Self {
        Self::text_with_styles(content, TextStyleSet::new(style))
    }

    pub fn text_with_styles(content: impl Into<String>, styles: TextStyleSet) -> Self {
        Self::new(Content::Text {
            content: content.into(),
            styles: Box::new(styles),
        })
    }

    pub fn image(image: impl Into<Image>) -> Self {
        Self::new(Content::Image(image.into()))
    }

    pub fn input(value: impl Into<String>) -> Self {
        Self::input_with_styles(value, TextStyleSet::default())
    }

    pub fn input_with_styles(value: impl Into<String>, styles: TextStyleSet) -> Self {
        let value = value.into();
        let caret = value.len();
        Self::new(Content::Input {
            value,
            styles: Box::new(styles),
            caret,
            anchor: caret,
            multiline: false,
            scroll: 0,
            preedit: String::new(),
        })
        .focusable(true)
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        if let Content::Input {
            multiline: current, ..
        } = &mut self.data
        {
            *current = multiline;
        }
        self
    }

    fn new(data: Content) -> Self {
        Self {
            id: WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed)),
            data,
            children: Vec::new(),
            layout: LayoutStyle::default(),
            styles: StyleSet::default(),
            state: WidgetState::default(),
            offset: None,
            draggable: false,
            focusable: false,
            layer: false,
            modal: false,
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn kind(&self) -> WidgetKind {
        self.data.kind()
    }

    pub fn with_layout(mut self, layout: LayoutStyle) -> Self {
        self.layout.apply(&LayoutPatch::from(&layout));
        self
    }

    pub fn with_styles(mut self, styles: StyleSet) -> Self {
        self.styles = styles;
        self
    }

    pub fn with_text_styles(mut self, styles: TextStyleSet) -> Self {
        match &mut self.data {
            Content::Text {
                styles: current, ..
            }
            | Content::Input {
                styles: current, ..
            } => **current = styles,
            Content::View | Content::Structure | Content::Image(_) => {}
        }
        self
    }

    pub fn with_class(mut self, class: &Class) -> Self {
        self.layout.apply(&class.layout);
        self.styles.apply_layers(&class.style, class.transitions);
        match &mut self.data {
            Content::Text {
                styles: current, ..
            }
            | Content::Input {
                styles: current, ..
            } => current.apply_layers(&class.text),
            Content::View | Content::Structure | Content::Image(_) => {}
        }
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = Some(Offset { x, y });
        self
    }

    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.state.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.state.visible = visible;
        self
    }
}
