use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    Any,
    Function,
    Value(Type),
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltinEmit {
    Function { path: &'static str },
    Host { marker: &'static str },
    Method { name: &'static str },
}

#[derive(Debug, Clone)]
pub struct Builtin {
    pub name: &'static str,
    pub arguments: &'static [Argument],
    pub result: Type,
    pub emit: BuiltinEmit,
}

impl Builtin {
    pub fn function_argument(&self) -> Option<usize> {
        self.arguments
            .iter()
            .position(|argument| matches!(argument, Argument::Function))
    }
}

include!(concat!(env!("OUT_DIR"), "/lang_builtins.rs"));

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|item| item.name == name)
}
