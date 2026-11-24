use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use brul_event::EventKind;
use brul_gui::{RawUiEvent, UiEvent, WidgetId};
use brul_script::{Dyn, Error as ScriptError, EventCall, Host, Value};
use brul_syntax::markup::PropType;

use super::ComponentHandle;
use super::component::accepts_prop;

#[derive(Clone)]
pub struct Handler {
    pub widget: Option<WidgetId>,
    pub event: EventKind,
    pub call: EventCall,
    pub component: ComponentHandle,
}

impl Handler {
    pub fn new(
        component: &ComponentHandle,
        widget: Option<WidgetId>,
        event: EventKind,
        call: EventCall,
    ) -> Self {
        Self {
            widget,
            event,
            call,
            component: Rc::clone(component),
        }
    }
}

pub struct ComponentEvent {
    pub name: &'static str,
    pub call: EventCall,
    pub params: &'static [PropType],
    pub component: ComponentHandle,
}

pub struct PendingEvent {
    pub component: ComponentHandle,
    pub call: EventCall,
    pub arguments: Vec<Dyn>,
}

pub enum Pending {
    Component(PendingEvent),
    Ui(UiEvent),
    Raw(RawUiEvent),
}

thread_local! {
    static PENDING: RefCell<VecDeque<Pending>> = const { RefCell::new(VecDeque::new()) };
}

pub fn push_ui_event(event: UiEvent) {
    PENDING.with(|pending| pending.borrow_mut().push_back(Pending::Ui(event)));
}

pub fn push_raw_event(event: RawUiEvent) {
    PENDING.with(|pending| pending.borrow_mut().push_back(Pending::Raw(event)));
}

pub fn take_pending() -> Option<Pending> {
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
            if !accepts_prop(value, ty) {
                return Err(ScriptError::event_argument(
                    event.name.to_owned(),
                    ty,
                    value,
                ));
            }
        }
        PENDING.with(|pending| {
            pending
                .borrow_mut()
                .push_back(Pending::Component(PendingEvent {
                    component: Rc::clone(&event.component),
                    call: event.call,
                    arguments: arguments.iter().cloned().map(Dyn::from).collect(),
                }));
        });
        Ok(Value::Null)
    }
}
