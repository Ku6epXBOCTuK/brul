//! Исполнение скрипт-блока BRUL: значения, окружение компонента и интерпретатор AST.

pub mod environment;
pub mod error;
pub mod eval;
pub mod value;

pub use environment::Environment;
pub use error::Error;
pub use eval::eval;
pub use value::Value;
