//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

mod each;
mod event;
pub mod logging;
mod run;
mod ui;

pub mod build {
    pub use brul_codegen::build::{Error, app};
}

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ClassTable, ComponentEvent, ComponentHandle, Handler,
    HandlerEvent, Instance, PropArgument, PropBinding, Rebuilt, RowInfo, Structure, View, instance,
    register_structure, take_structures,
};
pub use brul_gui as gui;
pub use brul_macros::{command, host};
pub use brul_script::eval;
pub use brul_script::{Component, Error, Host, Value, Variable, VariableId, execute_program};
pub use brul_style as style;
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_syntax::script;
pub use brul_window::WindowConfig;
pub use run::{WindowDef, run_generated};
pub use ui::Ui;
