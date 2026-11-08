use crate::ast::Expr;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub name: String,
    pub args: Vec<Arg>,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Prop(Prop),
    Event(Event),
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
}
