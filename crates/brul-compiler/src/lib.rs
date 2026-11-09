//! Компилятор BRUL: проверка DSL и кодеген.

pub mod styles;
pub mod tree;
pub mod window;

pub use tree::{
    Binding, BindingTarget, Built, Handler, HandlerEvent, build_bound_tree, build_tree, watches,
};
pub use window::{Loaded, load};
