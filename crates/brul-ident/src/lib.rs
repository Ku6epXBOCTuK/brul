use std::{
    borrow::Borrow,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FunctionName(Arc<str>);

impl FunctionName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for FunctionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for FunctionName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for FunctionName {
    fn from(name: &str) -> Self {
        Self(Arc::from(name))
    }
}

impl From<String> for FunctionName {
    fn from(name: String) -> Self {
        Self(Arc::from(name))
    }
}

impl From<FunctionName> for String {
    fn from(name: FunctionName) -> Self {
        name.0.to_string()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerOwner(u64);

impl TimerOwner {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::FunctionName;
    use std::collections::HashMap;

    #[test]
    fn a_function_name_is_found_by_its_text() {
        let mut calls = HashMap::new();
        calls.insert(FunctionName::from("tick"), 42);
        assert_eq!(calls.get("tick"), Some(&42));
    }
}
