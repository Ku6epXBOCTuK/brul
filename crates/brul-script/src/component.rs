use std::any::Any;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use brul_reactive::Signal;
use brul_syntax::ast::Position;

use crate::dynamic::Dyn;
use crate::environment::Environment;
use crate::error::Error;
use crate::host::{AnyHost, Host};
use crate::names::VariableId;
use crate::scope::Scope;
use crate::value::Value;

struct Property {
    value: Signal<Value>,
}

#[derive(Default)]
struct Fnv(u64);

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        self.0 = bytes.iter().fold(self.0, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type Index = HashMap<&'static str, VariableId, BuildHasherDefault<Fnv>>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FunctionName(String);

impl FunctionName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FunctionName {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for FunctionName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<FunctionName> for String {
    fn from(name: FunctionName) -> Self {
        name.0
    }
}

pub struct TimerRequest {
    pub interval_ms: f64,
    pub function: FunctionName,
}

pub struct Component {
    names: &'static [&'static str],
    index: Index,
    environment: Environment,
    properties: Vec<Option<Property>>,
    dispatch: Option<crate::ScriptDispatch>,
    typed_call: Option<crate::ScriptCall>,
    state: Option<Box<dyn Any>>,
    host: Option<Box<dyn AnyHost>>,
    pending_timers: Vec<TimerRequest>,
    event_calls: HashMap<String, crate::EventCall>,
    scope: Scope,
    // NOTE: environment frames and host functions are not signals, so their
    // changes wake every reader of them through this one coarse signal
    external: Signal<u64>,
}

impl Default for Component {
    fn default() -> Self {
        Self {
            names: &[],
            index: Index::default(),
            environment: Environment::default(),
            properties: Vec::new(),
            dispatch: None,
            typed_call: None,
            state: None,
            host: None,
            pending_timers: Vec::new(),
            event_calls: HashMap::new(),
            scope: Scope::root(),
            external: Signal::new(0),
        }
    }
}

impl Component {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_names(&mut self, names: &'static [&'static str]) {
        self.names = names;
        self.index = names
            .iter()
            .enumerate()
            .map(|(position, name)| (*name, VariableId::from_index(position as u32)))
            .collect();
        self.environment.set_names(names);
        if self.properties.len() < names.len() {
            self.properties.resize_with(names.len(), || None);
        }
    }

    pub fn names(&self) -> &'static [&'static str] {
        self.names
    }

    pub fn set_host(&mut self, host: impl Host + 'static) {
        self.host = Some(Box::new(host));
    }

    pub fn host_mut<T: Host + 'static>(&mut self) -> Option<&mut T> {
        self.host
            .as_mut()
            .and_then(|host| (&mut **host as &mut dyn std::any::Any).downcast_mut())
    }

    pub fn register_event_call(&mut self, name: &str, call: crate::EventCall) {
        self.event_calls.insert(name.to_owned(), call);
    }

    pub fn event_call(&self, name: &str) -> Option<crate::EventCall> {
        self.event_calls.get(name).copied()
    }

    pub fn set_dispatch(&mut self, dispatch: crate::ScriptDispatch) {
        self.dispatch = Some(dispatch);
    }

    pub fn set_typed_call(&mut self, call: crate::ScriptCall) {
        self.typed_call = Some(call);
    }

    pub fn call_typed(&mut self, message: &dyn std::any::Any) -> Result<Dyn, Error> {
        match self.typed_call {
            Some(call) => call(self, message),
            None => Err(Error::Unsupported("typed script call")),
        }
    }

    pub fn has_dispatch(&self) -> bool {
        self.dispatch.is_some()
    }

    pub fn set_state<T: 'static>(&mut self, state: T) {
        self.state = Some(Box::new(state));
    }

    pub fn state<T: 'static>(&mut self) -> &mut T {
        self.state
            .as_mut()
            .and_then(|state| state.downcast_mut())
            .expect("script state type mismatch")
    }

    pub fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        let _ = self.external.get();
        match &mut self.host {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }

    pub fn touch(&self) {
        self.external_changed();
    }

    pub fn every(
        &mut self,
        interval_ms: f64,
        function: impl Into<FunctionName>,
    ) -> Result<(), Error> {
        self.pending_timers.push(TimerRequest {
            interval_ms,
            function: function.into(),
        });
        Ok(())
    }

    pub fn take_timers(&mut self) -> Vec<TimerRequest> {
        std::mem::take(&mut self.pending_timers)
    }

    pub fn set_scope(&mut self, scope: Scope) {
        self.scope = scope;
    }

    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    pub fn commands(&self) -> Vec<&'static str> {
        match &self.host {
            Some(host) => host.commands(),
            None => Vec::new(),
        }
    }

    pub fn call_function(&mut self, name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
        match self.dispatch {
            Some(dispatch) => Ok(Value::from(dispatch(
                self,
                name,
                arguments.into_iter().map(Dyn::from).collect(),
            )?)),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        let id = self
            .id(name)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        self.environment.declare(id, value, constant)?;
        self.external_changed();
        Ok(())
    }

    pub fn declare_if_absent(
        &mut self,
        name: &str,
        value: Value,
        constant: bool,
    ) -> Result<(), Error> {
        let id = self
            .id(name)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        if self.environment.contains(id) {
            return Ok(());
        }
        self.declare(name, value, constant)
    }

    pub fn declare_property(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        if self.property(id).is_some() {
            return Err(Error::Redeclared(self.name(id).to_owned()));
        }
        let at = id.index() as usize;
        if self.properties.len() <= at {
            self.properties.resize_with(at + 1, || None);
        }
        self.properties[at] = Some(Property {
            value: Signal::new(value),
        });
        self.external_changed();
        Ok(())
    }

    pub fn seed<T>(&mut self, name: &str, position: Position) -> Result<T, Error>
    where
        T: for<'a> TryFrom<&'a Value, Error = Error>,
    {
        let value = self.read(name)?;
        T::try_from(&value).map_err(|error| error.at(position))
    }

    pub fn read(&self, name: &str) -> Result<Value, Error> {
        match self.id(name) {
            Some(id) => self.read_id(id),
            None => Err(Error::Undefined(name.to_owned())),
        }
    }

    pub fn read_id(&self, id: VariableId) -> Result<Value, Error> {
        if let Some(value) = self.environment.get(id) {
            let _ = self.external.get();
            return Ok(value.clone());
        }
        match self.property(id) {
            Some(property) => Ok(property.value.get()),
            None => {
                let _ = self.external.get();
                Err(Error::Undefined(self.name(id).to_owned()))
            }
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), Error> {
        match self.id(name) {
            Some(id) => self.assign_id(id, value),
            None => Err(Error::Undefined(name.to_owned())),
        }
    }

    pub fn assign_id(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        if self.environment.contains(id) {
            self.environment.assign(id, value)?;
            self.external_changed();
            return Ok(());
        }
        if let Some(property) = self.property(id) {
            property.value.set(value);
            return Ok(());
        }
        Err(Error::Undefined(self.name(id).to_owned()))
    }

    pub fn assign_property(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        if let Some(property) = self.property(id) {
            property.value.set(value);
            return Ok(());
        }
        Err(Error::Undefined(self.name(id).to_owned()))
    }

    fn external_changed(&self) {
        self.external.update(|count| *count += 1);
    }

    fn property(&self, id: VariableId) -> Option<&Property> {
        self.properties
            .get(id.index() as usize)
            .and_then(Option::as_ref)
    }

    pub fn name(&self, id: VariableId) -> &'static str {
        self.names
            .get(id.index() as usize)
            .copied()
            .unwrap_or("<unknown>")
    }

    pub fn id(&self, name: &str) -> Option<VariableId> {
        self.index.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use brul_reactive::{Effect, flush};

    use super::*;

    fn component(names: &'static [&'static str]) -> Rc<RefCell<Component>> {
        let mut component = Component::new();
        component.set_names(names);
        Rc::new(RefCell::new(component))
    }

    fn count_reads(handle: &Rc<RefCell<Component>>, name: &'static str) -> Rc<Cell<usize>> {
        let runs = Rc::new(Cell::new(0));
        let seen = Rc::clone(&runs);
        let component = Rc::clone(handle);
        Effect::new(move || {
            component.borrow().read(name).expect("read");
            seen.set(seen.get() + 1);
        });
        runs
    }

    fn declare_property(handle: &Rc<RefCell<Component>>, name: &str) {
        let id = handle.borrow().id(name).expect("name is in the table");
        handle
            .borrow_mut()
            .declare_property(id, Value::Null)
            .expect("declared");
    }

    #[test]
    fn a_property_read_follows_only_that_property() {
        let handle = component(&["first", "second"]);
        declare_property(&handle, "first");
        declare_property(&handle, "second");
        let first = handle.borrow().id("first").expect("name is in the table");
        let second = handle.borrow().id("second").expect("name is in the table");
        let runs = count_reads(&handle, "first");
        assert_eq!(runs.get(), 1);

        handle
            .borrow_mut()
            .assign_property(second, Value::Bool(true))
            .expect("assigned");
        flush();
        assert_eq!(runs.get(), 1, "another property does not wake the reader");

        handle
            .borrow_mut()
            .assign_property(first, Value::Bool(true))
            .expect("assigned");
        flush();
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn a_variable_read_follows_the_coarse_signal() {
        let handle = component(&["shown", "other"]);
        handle
            .borrow_mut()
            .declare("shown", Value::Bool(false), false)
            .expect("declared");
        let runs = count_reads(&handle, "shown");
        assert_eq!(runs.get(), 1);

        handle
            .borrow_mut()
            .assign("shown", Value::Bool(true))
            .expect("assigned");
        flush();
        assert_eq!(runs.get(), 2);

        handle.borrow().touch();
        flush();
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn properties_are_addressed_by_variable_id() {
        let mut component = Component::new();
        component.set_names(&["a", "b", "c", "d", "e", "f"]);
        let last = VariableId::from_index(5);
        let first = VariableId::from_index(1);
        component
            .declare_property(last, Value::Bool(true))
            .expect("declared");
        component
            .declare_property(first, Value::Number(7.0))
            .expect("declared");
        assert_eq!(component.read_id(last).unwrap(), Value::Bool(true));
        assert_eq!(component.read_id(first).unwrap(), Value::Number(7.0));
        assert!(component.declare_property(first, Value::Null).is_err());
    }
}
