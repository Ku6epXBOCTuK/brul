#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgSpec {
    Any,
    Function,
    Value(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSpecial {
    Theme,
    OpenWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostEmit {
    pub marker: &'static str,
    pub command: Option<&'static str>,
    pub argument: Option<&'static str>,
    pub special: Option<HostSpecial>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitSpec {
    Function(&'static str),
    Host(HostEmit),
    Method(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arguments: &'static [ArgSpec],
    pub result: &'static str,
    pub emit: EmitSpec,
    pub note: &'static str,
}

pub const BUILTINS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "String",
        arguments: &[ArgSpec::Any],
        result: "String",
        emit: EmitSpec::Function("brul::builtins::string"),
        note: "explicit conversion to text",
    },
    BuiltinSpec {
        name: "Number",
        arguments: &[ArgSpec::Any],
        result: "Number",
        emit: EmitSpec::Function("brul::builtins::number"),
        note: "explicit conversion to a number",
    },
    BuiltinSpec {
        name: "len",
        arguments: &[ArgSpec::Any],
        result: "Number",
        emit: EmitSpec::Function("brul::builtins::len"),
        note: "array length",
    },
    BuiltinSpec {
        name: "theme",
        arguments: &[],
        result: "String",
        emit: EmitSpec::Host(HostEmit {
            marker: "Theme",
            command: None,
            argument: None,
            special: Some(HostSpecial::Theme),
        }),
        note: "current theme; null before the first window",
    },
    BuiltinSpec {
        name: "setTheme",
        arguments: &[ArgSpec::Value("String")],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "SetTheme",
            command: Some("SetTheme"),
            argument: Some("String"),
            special: None,
        }),
        note: "switch the theme of all windows",
    },
    BuiltinSpec {
        name: "setAlwaysOnTop",
        arguments: &[ArgSpec::Value("Bool")],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "SetAlwaysOnTop",
            command: Some("AlwaysOnTop"),
            argument: Some("Bool"),
            special: None,
        }),
        note: "raise the current window above the others",
    },
    BuiltinSpec {
        name: "openWindow",
        arguments: &[ArgSpec::Value("String")],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "OpenWindow",
            command: None,
            argument: Some("String"),
            special: Some(HostSpecial::OpenWindow),
        }),
        note: "open a window declared in the markup",
    },
    BuiltinSpec {
        name: "exit",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "Exit",
            command: Some("Close"),
            argument: None,
            special: None,
        }),
        note: "close the current window; the last one ends the app",
    },
    BuiltinSpec {
        name: "closeWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "CloseWindow",
            command: Some("Close"),
            argument: None,
            special: None,
        }),
        note: "same as exit()",
    },
    BuiltinSpec {
        name: "dragWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "DragWindow",
            command: Some("Drag"),
            argument: None,
            special: None,
        }),
        note: "start a window drag",
    },
    BuiltinSpec {
        name: "showWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "ShowWindow",
            command: Some("Show"),
            argument: None,
            special: None,
        }),
        note: "show the current window",
    },
    BuiltinSpec {
        name: "hideWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "HideWindow",
            command: Some("Hide"),
            argument: None,
            special: None,
        }),
        note: "hide the current window; timers keep running",
    },
    BuiltinSpec {
        name: "focusWindow",
        arguments: &[],
        result: "Void",
        emit: EmitSpec::Host(HostEmit {
            marker: "FocusWindow",
            command: Some("Focus"),
            argument: None,
            special: None,
        }),
        note: "give the current window keyboard focus",
    },
    BuiltinSpec {
        name: "every",
        arguments: &[ArgSpec::Value("Number"), ArgSpec::Function],
        result: "Timer",
        emit: EmitSpec::Method("component.every"),
        note: "repeat a function every ms",
    },
    BuiltinSpec {
        name: "after",
        arguments: &[ArgSpec::Value("Number"), ArgSpec::Function],
        result: "Timer",
        emit: EmitSpec::Method("component.after"),
        note: "call a function once after ms",
    },
    BuiltinSpec {
        name: "cancel",
        arguments: &[ArgSpec::Value("Timer")],
        result: "Void",
        emit: EmitSpec::Method("component.cancel"),
        note: "cancel a timer created by every or after",
    },
];
