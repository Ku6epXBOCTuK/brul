#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub name: String,
    pub path: String,
}
