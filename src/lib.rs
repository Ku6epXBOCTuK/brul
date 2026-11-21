//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

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
    Binding, BindingTarget, Build, Built, ClassTable, ComponentEvent, ComponentHandle,
    DEFAULT_SLOT, FlagExpression, Handler, HandlerEvent, Instance, NumberExpression, PropArgument,
    PropBinding, PropExpression, Rebuilt, RowInfo, RowKey, SlotArgument, Slots, Structure,
    TextExpression, View, ViewError, instance, register_structure, slot,
};
pub use brul_gui as gui;
pub use brul_gui::{Theme, UnknownTheme};
pub use brul_macros::{command, host};
pub use brul_reactive::Signal;
pub use brul_script::eval;
pub use brul_script::{
    Component, Dyn, Error, EventCall, Fields, FunctionName, Host, HostCommand, Items, Scope,
    Script, TimerOwner, TimerRequest, Value, VariableId, items, noop_event,
};
pub use brul_style as style;
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_window::perf;
pub use brul_window::{Backend, UnknownBackend, WindowConfig, WindowKey, WindowPosition};
pub use each::RowMatcher;
pub use handle::Handle;
pub use run::{WindowDef, run_generated, run_generated_with};
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
