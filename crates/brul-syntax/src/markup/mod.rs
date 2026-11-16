mod ast;
mod parser;
mod rule;
mod widget;

pub use ast::{
    Arg, Block, BlockItem, Branch, Component, Declaration, EachBlock, Element, Event, EventParam,
    File, IfBlock, Member, Prop, PropType, Slot, Window,
};
pub use parser::parse_file;
pub use rule::Rule;
pub use widget::Widget;
