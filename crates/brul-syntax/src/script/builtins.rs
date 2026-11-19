use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    Any,
    Function,
    Value(Type),
}

#[derive(Debug, Clone)]
pub struct Builtin {
    pub name: &'static str,
    pub arguments: &'static [Argument],
    pub result: Type,
    pub marker: Option<&'static str>,
}

pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "String",
        arguments: &[Argument::Any],
        result: Type::String,
        marker: None,
    },
    Builtin {
        name: "Number",
        arguments: &[Argument::Any],
        result: Type::Number,
        marker: None,
    },
    Builtin {
        name: "len",
        arguments: &[Argument::Any],
        result: Type::Number,
        marker: None,
    },
    Builtin {
        name: "theme",
        arguments: &[],
        result: Type::String,
        marker: Some("Theme"),
    },
    Builtin {
        name: "setTheme",
        arguments: &[Argument::Value(Type::String)],
        result: Type::Void,
        marker: Some("SetTheme"),
    },
    Builtin {
        name: "setAlwaysOnTop",
        arguments: &[Argument::Value(Type::Bool)],
        result: Type::Void,
        marker: Some("SetAlwaysOnTop"),
    },
    Builtin {
        name: "openWindow",
        arguments: &[Argument::Value(Type::String)],
        result: Type::Void,
        marker: Some("OpenWindow"),
    },
    Builtin {
        name: "exit",
        arguments: &[],
        result: Type::Void,
        marker: Some("Exit"),
    },
    Builtin {
        name: "closeWindow",
        arguments: &[],
        result: Type::Void,
        marker: Some("CloseWindow"),
    },
    Builtin {
        name: "dragWindow",
        arguments: &[],
        result: Type::Void,
        marker: Some("DragWindow"),
    },
    Builtin {
        name: "showWindow",
        arguments: &[],
        result: Type::Void,
        marker: Some("ShowWindow"),
    },
    Builtin {
        name: "hideWindow",
        arguments: &[],
        result: Type::Void,
        marker: Some("HideWindow"),
    },
    Builtin {
        name: "focusWindow",
        arguments: &[],
        result: Type::Void,
        marker: Some("FocusWindow"),
    },
    Builtin {
        name: "every",
        arguments: &[Argument::Value(Type::Number), Argument::Function],
        result: Type::Void,
        marker: None,
    },
];

pub fn builtin(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|item| item.name == name)
}
