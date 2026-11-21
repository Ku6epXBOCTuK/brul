use crate::ast::{Expr, Position};

pub use crate::types::Type as PropType;

use super::widget::Widget;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub declaration: Option<Declaration>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub members: Vec<Member>,
    pub position: Position,
    pub kind: DeclarationKind,
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.members == other.members && self.kind == other.kind
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclarationKind {
    Component,
    Window { args: Vec<Arg> },
}

impl Declaration {
    pub fn is_window(&self) -> bool {
        matches!(self.kind, DeclarationKind::Window { .. })
    }

    pub fn args(&self) -> &[Arg] {
        match &self.kind {
            DeclarationKind::Window { args } => args,
            DeclarationKind::Component => &[],
        }
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
    pub widget: Option<Widget>,
    pub args: Vec<Arg>,
    pub block: Block,
    pub position: Position,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.widget == other.widget
            && self.args == other.args
            && self.block == other.block
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
    Slot(Slot),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub name: Option<String>,
    pub block: Option<Block>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfBlock {
    pub branches: Vec<Branch>,
    pub else_block: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub condition: Expr,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EachBlock {
    pub collection: Expr,
    pub item: String,
    pub index: Option<String>,
    pub key: Option<Expr>,
    pub block: Block,
    pub else_block: Option<Block>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Class(String),
    Named { name: String, value: Expr },
    Handler { event: String, handler: String },
}
