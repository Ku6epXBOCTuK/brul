mod ast;
mod parser;
mod rule;

include!(concat!(env!("OUT_DIR"), "/widget.rs"));

pub use ast::{
    Arg, Block, BlockItem, Branch, Declaration, DeclarationKind, EachBlock, Element, Event,
    EventParam, File, IfBlock, Member, Prop, PropType, Slot,
};
pub use parser::parse_file;
pub use rule::Rule;
