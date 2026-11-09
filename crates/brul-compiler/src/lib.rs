//! Компилятор BRUL: проверка DSL и кодеген.

pub mod loader;
pub mod styles;
pub mod tree;
pub mod window;

pub use loader::{Definition, Import, Registry};
pub use tree::{
    Binding, BindingTarget, Built, ComponentHandle, Context, Handler, HandlerEvent, RowInfo,
    Structure, StructureKind, build_bound_tree, build_tree, rebuild_structure,
};
pub use window::{Loaded, load, load_file};
