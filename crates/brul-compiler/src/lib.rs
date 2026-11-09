//! Сборка дерева BRUL: разметка, стили и скрипт в виджеты.

pub mod tree;
pub mod window;

pub use brul_codegen::styles;
pub use brul_codegen::{Definition, Import, Registry};
pub use tree::{
    Binding, BindingTarget, Built, ComponentHandle, Context, Handler, HandlerEvent, PropBinding,
    RowInfo, Structure, StructureKind, accepts_prop, build_bound_tree, build_tree,
    rebuild_structure,
};
pub use window::{Loaded, load, load_file, load_file_with, load_with};
