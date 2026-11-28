mod ast;
mod parser;
mod rule;

include!(concat!(env!("OUT_DIR"), "/widget.rs"));

pub use ast::{
    AnchorName, Arg, Block, BlockItem, Branch, Declaration, DeclarationKind, EachBlock, Element,
    Event, EventParam, File, IfBlock, Member, Prop, PropType, Slot, StructureRef,
};
pub use parser::parse_file;
pub use rule::Rule;
