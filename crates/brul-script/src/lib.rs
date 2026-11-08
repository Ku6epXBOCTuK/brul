//! Исполнение скрипт-блока BRUL: значения, окружение компонента и интерпретатор AST.

pub mod environment;
pub mod value;

pub use environment::Environment;
pub use value::Value;
