//! Окно BRUL: winit, wgpu и цикл событий.
//!
//! [`run`] открывает пустое окно, [`run_tree`] и [`run_tree_themed`] — окно с
//! деревом виджетов: дерево раскладывается, снапшот рисуется в кадр на CPU и
//! уходит на wgpu-поверхность одной текстурой. Меняются только dirty-регионы;
//! размер и DPI перерисовывают весь кадр.
//!
//! Конфиг задан в логических пикселях, поверхность — в физических; масштаб
//! берётся у окна. Высота хедера задаёт зону перетаскивания, Esc и закрытие окна
//! завершают цикл. winit 0.31-beta прибит точной версией.
//!
//! ```no_run
//! use brul_window::{WindowConfig, run};
//!
//! run(WindowConfig::default()).expect("window");
//! ```

mod gpu;
mod input;
mod surface;
mod tree;
mod window;

pub use window::{
    ColorScheme, WindowCommand, WindowConfig, WindowError, run, run_tree, run_tree_themed,
};
