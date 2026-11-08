#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<ClassRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassRule {
    pub class: String,
}
