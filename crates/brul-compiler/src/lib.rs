//! Сборка дерева BRUL: разметка, стили и скрипт в виджеты.

pub mod tree;
pub mod window;

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ComponentEvent, ComponentHandle, Context, Handler, HandlerEvent,
    Instance, PropArgument, PropBinding, Rebuilt, RowInfo, Structure, View, accepts_prop, instance,
};
pub use brul_codegen::styles;
pub use brul_codegen::{Definition, Import, Registry};
pub use tree::{build_bound_tree, build_tree};
pub use window::{Loaded, load, load_file, load_file_with, load_with};
