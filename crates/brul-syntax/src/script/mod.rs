mod ast;
mod parser;

pub use ast::{Function, Import, Program, Statement};
pub use parser::parse;
