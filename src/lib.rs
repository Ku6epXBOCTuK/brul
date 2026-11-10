//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

mod each;
mod event;
mod run;
mod ui;

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ComponentEvent, ComponentHandle, Handler, HandlerEvent,
    Instance, PropArgument, PropBinding, Rebuilt, RowInfo, Structure, View, instance,
};
pub use brul_gui::window::WindowConfig;
pub use brul_script::{Component, Error, Host, Value, Variable, execute_program};
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_syntax::script;
pub use run::run_generated;
pub use ui::{Ui, evaluate};
