#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgSpec {
    Any,
    Function,
    Value(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitSpec {
    Host(&'static str),
    StringFn,
    NumberFn,
    Len,
    Every,
    After,
    Cancel,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arguments: &'static [ArgSpec],
    pub result: &'static str,
    pub emit: EmitSpec,
}

pub const BUILTINS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "String",
        arguments: &[ArgSpec::Any],
        result: "String",
        emit: EmitSpec::StringFn,
    },
    BuiltinSpec {
        name: "Number",
        arguments: &[ArgSpec::Any],
        result: "Number",
        emit: EmitSpec::NumberFn,
    },
    BuiltinSpec {
        name: "len",
        arguments: &[ArgSpec::Any],
        result: "Number",
        emit: EmitSpec::Len,
    },
    BuiltinSpec {
        name: "theme",
        arguments: &[],
        result: "String",
        emit: EmitSpec::Host("Theme"),
    },
    BuiltinSpec {
        name: "setTheme",
        arguments: &[ArgSpec::Value("String")],
        result: "Void",
        emit: EmitSpec::Host("SetTheme"),
    },
    BuiltinSpec {
        name: "setAlwaysOnTop",
        arguments: &[ArgSpec::Value("Bool")],
        result: "Void",
        emit: EmitSpec::Host("SetAlwaysOnTop"),
    },
    BuiltinSpec {
        name: "openWindow",
        arguments: &[ArgSpec::Value("String")],
        result: "Void",
        emit: EmitSpec::Host("OpenWindow"),
    },
    BuiltinSpec {
        name: "exit",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("Exit"),
    },
    BuiltinSpec {
        name: "closeWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("CloseWindow"),
    },
    BuiltinSpec {
        name: "dragWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("DragWindow"),
    },
    BuiltinSpec {
        name: "showWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("ShowWindow"),
    },
    BuiltinSpec {
        name: "hideWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("HideWindow"),
    },
    BuiltinSpec {
        name: "focusWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host("FocusWindow"),
    },
    BuiltinSpec {
        name: "every",
        arguments: &[ArgSpec::Value("Number"), ArgSpec::Function],
        result: "Timer",
        emit: EmitSpec::Every,
    },
    BuiltinSpec {
        name: "after",
        arguments: &[ArgSpec::Value("Number"), ArgSpec::Function],
        result: "Timer",
        emit: EmitSpec::After,
    },
    BuiltinSpec {
        name: "cancel",
        arguments: &[ArgSpec::Value("Timer")],
        result: "Void",
        emit: EmitSpec::Cancel,
    },
];
