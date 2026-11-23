//! BRUL facade: app startup, window roots and the shared `Handle` context.

pub mod builtins;
mod each;
mod event;
mod handle;

pub use handle::Message;
pub mod logging;
mod run;
mod ui;

pub mod build {
    pub use brul_codegen::build::{Error, app};
}

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Build, Built, Capability, ClassTable, ComponentEvent, ComponentHandle,
    DEFAULT_SLOT, Element, EventSink, FlagExpression, Handler, Hooks, Instance, NumberExpression,
    PropArgument, PropBinding, PropExpression, Rebuilt, RowInfo, RowKey, SharedStates,
    SlotArgument, Slots, StateId, States, Structure, TextExpression, View, ViewError,
    apply_capability, emit_widget_event, instance, register_structure, slot,
};
pub use brul_gpu::effect;
pub use brul_gui as gui;
pub use brul_gui::{Event, EventKind, EventSet, EventTarget, Theme, UiEvent, UnknownTheme};
pub use brul_macros::{attach, command, effect, host, widget};
pub use brul_reactive::Signal;
pub use brul_script::{
    Component, Dyn, Error, EventCall, Fields, FunctionName, Host, HostCommand, Items, Scope,
    Script, TimerId, TimerKind, TimerOwner, TimerRequest, Value, VariableId, items, noop_event,
};
pub use brul_style as style;
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_video as video;
pub use brul_webview as webview;
pub use brul_window::perf;
pub use brul_window::{Backend, UnknownBackend, WindowConfig, WindowKey, WindowPosition};
pub use each::RowMatcher;
pub use handle::Handle;
pub use run::{
    PlatformRunOptions, WindowDef, run_generated, run_generated_with, run_generated_with_platform,
};
pub use ui::Ui;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TestTheme {
    Light,
    Dark,
}

#[cfg(test)]
impl Theme for TestTheme {
    fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}
