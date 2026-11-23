use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use brul_gui::{ImeKind, KeyModifiers, KeyPress, WidgetEvent, WidgetId, WidgetTree};
use brul_script::{Component, Dyn, Error as ScriptError, Value};

use super::event_sink;
use super::{ComponentHandle, EventSink, SharedStates, StateId, ViewError};

pub type Param = fn(&mut Component) -> Result<Dyn, ScriptError>;
pub type Apply = fn(&mut AttachCtx<'_>, StateId, &[Dyn]) -> Result<(), ViewError>;
pub type EventHook = fn(&mut AttachCtx<'_>, StateId, &AttachEvent<'_>) -> bool;
pub type CleanupHook = fn(&mut AttachCtx<'_>, StateId);

#[derive(Clone, Copy, Debug, Default)]
pub struct AttachEvents {
    pub key: bool,
    pub ime: bool,
    pub input: bool,
    pub submit: bool,
    pub focus: bool,
    pub blur: bool,
    pub click: bool,
}

impl AttachEvents {
    pub fn accepts(self, event: &AttachEvent<'_>) -> bool {
        match event {
            AttachEvent::Key { .. } => self.key,
            AttachEvent::Ime(_) => self.ime,
            AttachEvent::Input { .. } => self.input,
            AttachEvent::Submit => self.submit,
            AttachEvent::Focus => self.focus,
            AttachEvent::Blur => self.blur,
            AttachEvent::Click => self.click,
        }
    }
}

#[derive(Clone)]
pub struct Attachment {
    pub widget: WidgetId,
    pub state: StateId,
    pub component: ComponentHandle,
    pub params: Vec<Param>,
    pub apply: Apply,
    pub event: EventHook,
    pub cleanup: CleanupHook,
    pub events: AttachEvents,
}

pub fn apply(ctx: &mut AttachCtx<'_>, params: &[Param], apply: Apply) -> Result<(), ViewError> {
    let mut values = Vec::with_capacity(params.len());
    for param in params {
        values.push(param(&mut ctx.component.borrow_mut())?);
    }
    apply(ctx, ctx.state, &values)
}

pub struct AttachCtx<'a> {
    pub widget: WidgetId,
    pub state: StateId,
    pub component: &'a ComponentHandle,
    pub tree: &'a Rc<RefCell<WidgetTree>>,
    pub states: &'a SharedStates,
    pub sink: Option<&'a Arc<dyn EventSink>>,
}

impl AttachCtx<'_> {
    pub fn element(&self) -> Element<'_> {
        Element {
            id: self.widget,
            tree: self.tree,
        }
    }

    pub fn state<T: 'static>(&self) -> Option<Rc<RefCell<T>>> {
        self.states.borrow().get_cloned::<RefCell<T>>(self.state)
    }

    pub fn set_state<T: 'static>(&mut self, value: T) {
        self.states
            .borrow_mut()
            .get::<RefCell<T>>(self.state, self.component, || RefCell::new(value));
    }

    pub fn remove_state(&self) {
        self.states.borrow_mut().remove(self.state);
    }

    pub fn emit(&self, event: &str, arguments: Vec<Value>) {
        event_sink::emit(&self.sink.cloned(), self.state, event, arguments);
    }
}

pub struct Element<'a> {
    id: WidgetId,
    tree: &'a Rc<RefCell<WidgetTree>>,
}

impl Element<'_> {
    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn alive(&self) -> bool {
        self.tree.borrow().contains(self.id)
    }

    pub fn set_text(&self, text: impl Into<String>) -> bool {
        self.tree.borrow_mut().set_text(self.id, text.into())
    }

    pub fn set_input(&self, text: impl Into<String>) -> bool {
        self.tree.borrow_mut().set_input_text(self.id, text.into())
    }

    pub fn set_checked(&self, checked: bool) -> bool {
        self.tree.borrow_mut().set_checked(self.id, checked)
    }

    pub fn set_selected(&self, selected: bool) -> bool {
        self.tree.borrow_mut().set_selected(self.id, selected)
    }

    pub fn set_disabled(&self, disabled: bool) -> bool {
        self.tree.borrow_mut().set_disabled(self.id, disabled)
    }

    pub fn set_visible(&self, visible: bool) -> bool {
        self.tree.borrow_mut().set_visible(self.id, visible)
    }

    pub fn set_offset(&self, x: i32, y: i32) -> bool {
        self.tree.borrow_mut().set_offset(self.id, x, y)
    }

    pub fn caret_area(&self) -> Option<brul_gui::CaretArea> {
        self.tree.borrow().caret_area()
    }
}

pub enum AttachEvent<'a> {
    Key {
        key: &'a KeyPress,
        modifiers: &'a KeyModifiers,
    },
    Ime(&'a ImeKind),
    Input {
        value: &'a str,
        caret: usize,
    },
    Submit,
    Focus,
    Blur,
    Click,
}

pub fn from_widget(event: &WidgetEvent) -> Option<AttachEvent<'_>> {
    match event {
        WidgetEvent::Key { key, modifiers } => Some(AttachEvent::Key { key, modifiers }),
        WidgetEvent::Ime { kind, .. } => Some(AttachEvent::Ime(kind)),
        WidgetEvent::Input(_, value, caret) => Some(AttachEvent::Input {
            value,
            caret: *caret,
        }),
        WidgetEvent::Submit(_) => Some(AttachEvent::Submit),
        WidgetEvent::Focus(_) => Some(AttachEvent::Focus),
        WidgetEvent::Blur(_) => Some(AttachEvent::Blur),
        WidgetEvent::Click(_) => Some(AttachEvent::Click),
        _ => None,
    }
}

pub fn event_widget(event: &WidgetEvent) -> Option<WidgetId> {
    match event {
        WidgetEvent::PointerEnter(id)
        | WidgetEvent::PointerLeave(id)
        | WidgetEvent::Press(id)
        | WidgetEvent::Release(id)
        | WidgetEvent::Click(id)
        | WidgetEvent::Drag(id)
        | WidgetEvent::Input(id, _, _)
        | WidgetEvent::Ime { id, .. }
        | WidgetEvent::Focus(id)
        | WidgetEvent::Blur(id)
        | WidgetEvent::Submit(id)
        | WidgetEvent::Dismiss(id) => Some(*id),
        _ => None,
    }
}
