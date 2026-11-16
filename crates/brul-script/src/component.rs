use std::cell::OnceCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use brul_reactive::Signal;
use brul_syntax::script::Function;

use crate::environment::Environment;
use crate::error::Error;
use crate::execute::execute;
use crate::host::Host;
use crate::names::VariableId;
use crate::value::Value;

struct Property {
    id: VariableId,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: VariableId,
    pub value: Value,
}

pub struct Component {
    names: &'static [&'static str],
    index: Index,
    environment: Environment,
    properties: Vec<Property>,
    functions: Vec<Arc<Function>>,
    host: Option<Box<dyn Host>>,
    pending_every: Vec<(f64, String)>,
    build_scopes: Vec<Rc<[Variable]>>,
    scopes_snapshot: OnceCell<Rc<[Rc<[Variable]>]>>,
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
            functions: Vec::new(),
            host: None,
            pending_every: Vec::new(),
            build_scopes: Vec::new(),
            scopes_snapshot: OnceCell::new(),
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
    }

    pub fn names(&self) -> &'static [&'static str] {
        self.names
    }

    pub fn set_host(&mut self, host: impl Host + 'static) {
        self.host = Some(Box::new(host));
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

    pub fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        match &mut self.host {
            Some(host) => host.every(interval_ms, function),
            None => {
                self.pending_every.push((interval_ms, function.to_owned()));
                Ok(())
            }
        }
    }

    pub fn take_every(&mut self) -> Vec<(f64, String)> {
        std::mem::take(&mut self.pending_every)
    }

    pub fn push_build_scope(&mut self, variables: Vec<Variable>) {
        let scope: Rc<[Variable]> = variables.into();
        self.environment.push_scope(Rc::clone(&scope));
        self.build_scopes.push(scope);
        self.scopes_snapshot.take();
    }

    pub fn pop_build_scope(&mut self) {
        self.environment.pop_frame();
        self.build_scopes.pop();
        self.scopes_snapshot.take();
    }

    pub fn build_scopes(&self) -> Rc<[Rc<[Variable]>]> {
        Rc::clone(
            self.scopes_snapshot
                .get_or_init(|| Rc::from(self.build_scopes.clone())),
        )
    }

    pub fn push_binding_scopes(&mut self, scopes: &Rc<[Rc<[Variable]>]>) {
        for scope in scopes.iter() {
            self.environment.push_scope(Rc::clone(scope));
        }
    }

    pub fn pop_binding_scopes(&mut self, count: usize) {
        for _ in 0..count {
            self.environment.pop_frame();
        }
    }

    pub fn define_function(&mut self, function: impl Into<Arc<Function>>) -> Result<(), Error> {
        let function = function.into();
        if self.has_function(&function.name) {
            return Err(Error::FunctionRedeclared(function.name.clone()));
        }
        self.functions.push(function);
        Ok(())
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|function| function.name == name)
    }

    pub fn call_function(&mut self, name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
        let Some(found) = self.functions.iter().find(|function| function.name == name) else {
            return Err(Error::UnknownFunction(name.to_owned()));
        };
        let function = Arc::clone(found);
        if function.params.len() != arguments.len() {
            return Err(Error::FunctionArity {
                name: function.name.clone(),
                expected: function.params.len(),
                found: arguments.len(),
            });
        }
        self.environment.push_frame();
        for (param, argument) in function.params.iter().zip(arguments) {
            let id = self
                .id(param)
                .ok_or_else(|| Error::Undefined(param.to_owned()))?;
            self.environment.declare(id, argument, false)?;
        }
        let result = execute(self, &function.body);
        self.environment.pop_frame();
        result.map(|()| Value::Null)
    }

    pub fn declare(&mut self, name: &str, value: Value, constant: bool) -> Result<(), Error> {
        let id = self
            .id(name)
            .ok_or_else(|| Error::Undefined(name.to_owned()))?;
        self.environment.declare(id, value, constant)?;
        self.external_changed();
        Ok(())
    }

    pub fn declare_property(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        if self.property(id).is_some() {
            return Err(Error::Redeclared(self.name(id).to_owned()));
        }
        self.properties.push(Property {
            id,
            value: Signal::new(value),
        });
        self.external_changed();
        Ok(())
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
        self.properties.iter().find(|property| property.id == id)
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
}
