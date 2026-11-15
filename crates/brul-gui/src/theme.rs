pub trait Theme: Copy + PartialEq + 'static {
    fn name(self) -> &'static str;
    fn parse(name: &str) -> Option<Self>;
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown theme '{name}'")]
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
