use std::cell::OnceCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

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
    functions: Vec<Function>,
    host: Option<Box<dyn Host>>,
    pending_every: Vec<(f64, String)>,
    build_scopes: Vec<Vec<Variable>>,
    scopes_snapshot: OnceCell<Rc<[Vec<Variable>]>>,
    revision: Signal<u64>,
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
            revision: Signal::new(0),
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
        match &mut self.host {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
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

    pub fn push_build_scope(&mut self, variables: &[Variable]) {
        self.environment.push_frame();
        for variable in variables {
            let _ = self
                .environment
                .declare(variable.name, variable.value.clone(), false);
        }
        self.build_scopes.push(variables.to_vec());
        self.scopes_snapshot.take();
    }

    pub fn pop_build_scope(&mut self) {
        self.environment.pop_frame();
        self.build_scopes.pop();
        self.scopes_snapshot.take();
    }

    pub fn build_scopes(&self) -> Rc<[Vec<Variable>]> {
        Rc::clone(
            self.scopes_snapshot
                .get_or_init(|| Rc::from(self.build_scopes.clone())),
        )
    }

    pub fn push_binding_scopes(&mut self, scopes: &Rc<[Vec<Variable>]>) {
        for index in 0..scopes.len() {
            self.environment.push_scope(scopes, index);
        }
    }

    pub fn pop_binding_scopes(&mut self, count: usize) {
        for _ in 0..count {
            self.environment.pop_frame();
        }
    }

    pub fn define_function(&mut self, function: Function) -> Result<(), Error> {
        if self.has_function(&function.name) {
            return Err(Error::FunctionRedeclared(function.name));
        }
        self.functions.push(function);
        Ok(())
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|function| function.name == name)
    }

    pub fn call_function(&mut self, name: &str, arguments: Vec<Value>) -> Result<Value, Error> {
        let Some(function) = self
            .functions
            .iter()
            .find(|function| function.name == name)
            .cloned()
        else {
            return Err(Error::UnknownFunction(name.to_owned()));
        };
        if function.params.len() != arguments.len() {
            return Err(Error::FunctionArity {
                name: function.name,
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
        self.changed();
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
        self.changed();
        Ok(())
    }

    pub fn track(&self) {
        let _ = self.revision.get();
    }

    pub fn read(&self, name: &str) -> Result<Value, Error> {
        match self.id(name) {
            Some(id) => self.read_id(id),
            None => Err(Error::Undefined(name.to_owned())),
        }
    }

    pub fn read_id(&self, id: VariableId) -> Result<Value, Error> {
        if let Some(value) = self.environment.get(id) {
            return Ok(value.clone());
        }
        if let Some(property) = self.property(id) {
            return Ok(property.value.get());
        }
        Err(Error::Undefined(self.name(id).to_owned()))
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
            self.changed();
            return Ok(());
        }
        if let Some(property) = self.property(id) {
            property.value.set(value);
            self.changed();
            return Ok(());
        }
        Err(Error::Undefined(self.name(id).to_owned()))
    }

    pub fn assign_property(&mut self, id: VariableId, value: Value) -> Result<(), Error> {
        if let Some(property) = self.property(id) {
            property.value.set(value);
            self.changed();
            return Ok(());
        }
        Err(Error::Undefined(self.name(id).to_owned()))
    }

    fn changed(&self) {
        self.revision.update(|revision| *revision += 1);
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
