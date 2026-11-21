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

pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "String",
        arguments: &[Argument::Any],
        result: Type::String,
        kind: BuiltinKind::StringFn,
    },
    Builtin {
        name: "Number",
        arguments: &[Argument::Any],
        result: Type::Number,
        kind: BuiltinKind::NumberFn,
    },
    Builtin {
        name: "len",
        arguments: &[Argument::Any],
        result: Type::Number,
        kind: BuiltinKind::Len,
    },
    Builtin {
        name: "theme",
        arguments: &[],
        result: Type::String,
        kind: BuiltinKind::Host { marker: "Theme" },
    },
    Builtin {
        name: "setTheme",
        arguments: &[Argument::Value(Type::String)],
        result: Type::Void,
        kind: BuiltinKind::Host { marker: "SetTheme" },
    },
    Builtin {
        name: "setAlwaysOnTop",
        arguments: &[Argument::Value(Type::Bool)],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "SetAlwaysOnTop",
        },
    },
    Builtin {
        name: "openWindow",
        arguments: &[Argument::Value(Type::String)],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "OpenWindow",
        },
    },
    Builtin {
        name: "exit",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host { marker: "Exit" },
    },
    Builtin {
        name: "closeWindow",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "CloseWindow",
        },
    },
    Builtin {
        name: "dragWindow",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "DragWindow",
        },
    },
    Builtin {
        name: "showWindow",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "ShowWindow",
        },
    },
    Builtin {
        name: "hideWindow",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "HideWindow",
        },
    },
    Builtin {
        name: "focusWindow",
        arguments: &[],
        result: Type::Void,
        kind: BuiltinKind::Host {
            marker: "FocusWindow",
        },
    },
    Builtin {
        name: "every",
        arguments: &[Argument::Value(Type::Number), Argument::Function],
        result: Type::Timer,
        kind: BuiltinKind::Every,
    },
    Builtin {
        name: "after",
        arguments: &[Argument::Value(Type::Number), Argument::Function],
        result: Type::Timer,
        kind: BuiltinKind::After,
    },
    Builtin {
        name: "cancel",
        arguments: &[Argument::Value(Type::Timer)],
        result: Type::Void,
        kind: BuiltinKind::Cancel,
    },
];

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|item| item.name == name)
}
