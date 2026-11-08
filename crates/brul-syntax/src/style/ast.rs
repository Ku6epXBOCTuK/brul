#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<ClassRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassRule {
    pub class: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Px(f64),
    Percent(f64),
    String(String),
    Keyword(String),
}
