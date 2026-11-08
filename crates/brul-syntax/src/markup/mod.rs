mod ast;
mod parser;

pub use ast::{Arg, Component, Declaration, File, Window};
pub use parser::parse_file;
