//! Сборка дерева BRUL: разметка, стили и скрипт в виджеты.

pub mod tree;
pub mod window;

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ComponentHandle, Context, Handler, HandlerEvent, PropBinding,
    Rebuilt, RowInfo, Structure,
};
pub use brul_codegen::styles;
pub use brul_codegen::{Definition, Import, Registry};
pub use tree::{accepts_prop, build_bound_tree, build_tree};
pub use window::{Loaded, load, load_file, load_file_with, load_with};
