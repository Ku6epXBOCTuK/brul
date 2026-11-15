use std::fmt;

pub trait Theme: Copy + PartialEq + 'static {
    fn name(self) -> &'static str;
    fn parse(name: &str) -> Option<Self>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownTheme {
    name: String,
}

impl UnknownTheme {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for UnknownTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown theme '{}'", self.name)
    }
}

impl std::error::Error for UnknownTheme {}
