//! Исполнение скрипт-блока BRUL: значения и окружение компонента.

pub mod component;
pub mod environment;
pub mod error;
pub mod eval;
pub mod host;
pub mod names;
pub mod scope;
pub mod value;

pub use component::Component;
pub use environment::Environment;
pub use error::Error;
pub use eval::eval;
pub use host::{Host, HostCommand};
pub use names::{Names, VariableId};
pub use scope::Scope;
pub use value::{Fields, Value};

pub type ScriptInit = fn(&mut Component) -> Result<(), Error>;
pub type ScriptDispatch = fn(&mut Component, &str, Vec<Value>) -> Result<Value, Error>;
pub type ScriptCall = fn(&mut Component, &dyn std::any::Any) -> Result<Value, Error>;

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
