use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    Any,
    Function,
    Value(Type),
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltinKind {
    Host { marker: &'static str },
    StringFn,
    NumberFn,
    Len,
    Every,
    After,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Builtin {
    pub name: &'static str,
    pub arguments: &'static [Argument],
    pub result: Type,
    pub kind: BuiltinKind,
}

include!(concat!(env!("OUT_DIR"), "/lang_builtins.rs"));

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|item| item.name == name)
}
