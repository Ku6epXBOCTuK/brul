use crate::ast::{Expr, Position};

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Component(Component),
    Window(Window),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub args: Vec<Arg>,
    pub members: Vec<Member>,
    pub position: Position,
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.args == other.args && self.members == other.members
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Prop(Prop),
    Event(Event),
    Element(Element),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub args: Vec<Arg>,
    pub block: Block,
    pub position: Position,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.args == other.args && self.block == other.block
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Element(Element),
    If(IfBlock),
    Each(EachBlock),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfBlock {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EachBlock {
    pub collection: Expr,
    pub item: String,
    pub index: Option<String>,
    pub key: Option<Expr>,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub name: String,
    pub ty: PropType,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub name: String,
    pub params: Vec<EventParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventParam {
    pub name: String,
    pub ty: PropType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Bool,
    Number,
    String,
    Image,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Class(String),
    Named { name: String, value: Expr },
    Handler { event: String, handler: String },
}
