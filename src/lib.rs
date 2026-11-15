//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

mod each;
mod event;
mod handle;
pub mod logging;
mod run;
mod ui;

pub mod build {
    pub use brul_codegen::build::{Error, app};
}

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ClassTable, ComponentEvent, ComponentHandle, DEFAULT_SLOT,
    Handler, HandlerEvent, Instance, PropArgument, PropBinding, Rebuilt, RowInfo, SlotArgument,
    Slots, Structure, View, ViewError, instance, register_structure, slot, take_structures,
};
pub use brul_gui as gui;
pub use brul_gui::{Theme, UnknownTheme};
pub use brul_macros::{command, host};
pub use brul_script::eval;
pub use brul_script::{Component, Error, Host, Value, Variable, VariableId, execute_program};
pub use brul_style as style;
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_syntax::script;
pub use brul_window::{WindowConfig, WindowPosition};
pub use handle::{Handle, Signal};
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
