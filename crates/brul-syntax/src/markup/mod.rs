mod ast;
mod parser;

pub use ast::{
    Arg, Component, Declaration, Event, EventParam, File, Member, Prop, PropType, Window,
};
pub use parser::parse_file;
