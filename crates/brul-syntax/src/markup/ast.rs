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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub name: String,
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Class(String),
}
