mod ast;
mod parser;

pub use ast::{
    Arg, Block, BlockItem, Component, Declaration, Element, Event, EventParam, File, IfBlock,
    Member, Prop, PropType, Window,
};
pub use parser::parse_file;
