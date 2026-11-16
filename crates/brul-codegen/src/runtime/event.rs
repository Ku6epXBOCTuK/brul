use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use brul_gui::WidgetId;
use brul_script::{Error as ScriptError, Host, Value};
use brul_syntax::markup::PropType;

use super::ComponentHandle;
use super::component::{accepts_prop, prop_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerEvent {
    Click,
    Press,
    Release,
    Enter,
    Leave,
    Drag,
    Key,
    Close,
    Focus,
    Blur,
    Input,
    Submit,
    Dismiss,
}

impl HandlerEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Press => "press",
            Self::Release => "release",
            Self::Enter => "enter",
            Self::Leave => "leave",
            Self::Drag => "drag",
            Self::Key => "key",
            Self::Close => "close",
            Self::Focus => "focus",
            Self::Blur => "blur",
            Self::Input => "input",
            Self::Submit => "submit",
            Self::Dismiss => "dismiss",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "click" => Some(Self::Click),
            "press" => Some(Self::Press),
            "release" => Some(Self::Release),
            "enter" => Some(Self::Enter),
            "leave" => Some(Self::Leave),
            "drag" => Some(Self::Drag),
            "key" => Some(Self::Key),
            "close" => Some(Self::Close),
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            "input" => Some(Self::Input),
            "submit" => Some(Self::Submit),
            "dismiss" => Some(Self::Dismiss),
            _ => None,
        }
    }

    pub const fn arguments_count(self) -> usize {
        match self {
            Self::Key | Self::Input => 1,
            _ => 0,
        }
    }
}

#[derive(Clone)]
pub struct Handler {
    pub widget: Option<WidgetId>,
    pub event: HandlerEvent,
    pub function: String,
    pub component: ComponentHandle,
}

impl Handler {
    pub fn new(
        component: &ComponentHandle,
        widget: Option<WidgetId>,
        event: HandlerEvent,
        function: &str,
    ) -> Self {
        Self {
            widget,
            event,
            function: function.to_owned(),
            component: Rc::clone(component),
        }
    }
}

pub struct ComponentEvent {
    pub name: &'static str,
    pub function: &'static str,
    pub params: &'static [PropType],
    pub component: ComponentHandle,
}

pub struct PendingEvent {
    pub component: ComponentHandle,
    pub function: &'static str,
    pub arguments: Vec<Value>,
}

thread_local! {
    static PENDING: RefCell<VecDeque<PendingEvent>> = const { RefCell::new(VecDeque::new()) };
}

pub fn take_event() -> Option<PendingEvent> {
    PENDING.with(|pending| pending.borrow_mut().pop_front())
}

pub(crate) struct EventHost {
    pub(crate) events: Vec<ComponentEvent>,
}

impl Host for EventHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, ScriptError> {
        let Some(event) = self.events.iter().find(|event| event.name == name) else {
            return Err(ScriptError::UnknownFunction(name.to_owned()));
        };
        if event.params.len() != arguments.len() {
            return Err(ScriptError::FunctionArity {
                name: event.name.to_owned(),
                expected: event.params.len(),
                found: arguments.len(),
            });
        }
        for (ty, value) in event.params.iter().zip(arguments) {
            if !accepts_prop(value, *ty) {
                return Err(ScriptError::EventArgument {
                    name: event.name.to_owned(),
                    expected: prop_name(*ty),
                    found: value.type_name(),
                });
            }
        }
        PENDING.with(|pending| {
            pending.borrow_mut().push_back(PendingEvent {
                component: Rc::clone(&event.component),
                function: event.function,
                arguments: arguments.to_vec(),
            });
        });
        Ok(Value::Null)
    }
}
