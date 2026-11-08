mod ast;
mod parser;
mod resolve;

pub use ast::{Function, Import, Program, Statement};
pub use parser::parse;
pub use resolve::validate;
