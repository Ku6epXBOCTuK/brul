mod ast;
mod parser;

pub use ast::{ClassRule, Color, Gradient, Property, Stylesheet, Theme, Value};
pub use parser::parse_file;
