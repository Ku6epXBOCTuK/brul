//! Исполнение скрипт-блока BRUL: значения и окружение компонента.

pub mod component;
pub mod dynamic;
pub mod environment;
pub mod error;
pub mod eval;
pub mod host;
pub mod names;
pub mod scope;
pub mod value;

pub use brul_ident::{FunctionName, TimerOwner};
pub use component::{Component, TimerRequest};
pub use dynamic::{Dyn, items};
pub use environment::Environment;
pub use error::Error;
pub use eval::eval;
pub use host::{Host, HostCommand};
pub use names::{Names, VariableId};
pub use scope::Scope;
pub use value::{Fields, Value};

pub use brul_syntax::TypeName;

pub type Items = Vec<Dyn>;

pub type ScriptInit = fn(&mut Component) -> Result<(), Error>;
pub type ScriptDispatch = fn(&mut Component, &str, Vec<Dyn>) -> Result<Dyn, Error>;
pub type ScriptCall = fn(&mut Component, &dyn std::any::Any) -> Result<Dyn, Error>;
pub type EventCall = fn(&mut Component, &[Dyn]) -> Result<Dyn, Error>;

pub fn noop_event(_component: &mut Component, _arguments: &[Dyn]) -> Result<Dyn, Error> {
    Ok(Dyn::null())
}

#[derive(Clone, Copy)]
pub enum Script {
    Generated {
        init: ScriptInit,
        dispatch: ScriptDispatch,
    },
    Typed {
        init: ScriptInit,
        call: ScriptCall,
        dispatch: ScriptDispatch,
    },
}
