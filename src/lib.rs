//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{WidgetTree, window};

/// Загружает `.brul` и открывает его окно.
pub fn run(source: &str, theme: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let loaded = brul_compiler::load(source, theme)?;
    let tree = Rc::new(RefCell::new(WidgetTree::new(loaded.tree)));
    window::run_tree(loaded.config, tree, |_| {})?;
    Ok(())
}
