//! Исполнение скрипт-блока BRUL: значения, окружение компонента и интерпретатор AST.

pub mod component;
pub mod environment;
pub mod error;
pub mod eval;
pub mod execute;
pub mod host;
pub mod names;
pub mod value;

pub use component::{Component, Variable};
pub use environment::Environment;
pub use error::Error;
pub use eval::eval;
pub use execute::{execute, execute_program};
pub use host::Host;
pub use names::{Names, VariableId};
pub use value::{Fields, Value};

pub type ScriptInit = fn(&mut Component) -> Result<(), Error>;
pub type ScriptDispatch = fn(&mut Component, &str, Vec<Value>) -> Result<Value, Error>;

#[derive(Clone, Copy)]
pub enum Script<'a> {
    Program(&'a brul_syntax::script::Program),
    Generated {
        init: ScriptInit,
        dispatch: ScriptDispatch,
    },
}
