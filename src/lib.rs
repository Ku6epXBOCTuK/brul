//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::RefCell;
use std::rc::Rc;

use brul_compiler::{Binding, BindingTarget, Loaded};
use brul_gui::{Widget, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Value, eval};
use brul_syntax::ast::Expr;

pub struct Ui {
    pub component: Rc<RefCell<Component>>,
    pub tree: Rc<RefCell<WidgetTree>>,
    _scope: EffectScope,
}

impl Ui {
    pub fn new(component: Component, widget: Widget, bindings: Vec<Binding>) -> Self {
        let component = Rc::new(RefCell::new(component));
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        let scope = EffectScope::new();
        for binding in bindings {
            let component = Rc::clone(&component);
            let tree = Rc::clone(&tree);
            scope.run(|| {
                Effect::new(move || apply(&binding, &component, &tree));
            });
        }
        Self {
            component,
            tree,
            _scope: scope,
        }
    }
}

fn apply(binding: &Binding, component: &RefCell<Component>, tree: &RefCell<WidgetTree>) {
    component.borrow().track();
    match &binding.target {
        BindingTarget::Text(expression) => {
            if let Some(content) = text(component, expression) {
                tree.borrow_mut().set_text(binding.widget, content);
            }
        }
        BindingTarget::Checked(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_checked(binding.widget, value);
            }
        }
        BindingTarget::Selected(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_selected(binding.widget, value);
            }
        }
        BindingTarget::Disabled(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_disabled(binding.widget, value);
            }
        }
        BindingTarget::Visible(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_visible(binding.widget, value);
            }
        }
        BindingTarget::Offset { x, y } => {
            if let (Some(x), Some(y)) =
                (offset(component, x.as_ref()), offset(component, y.as_ref()))
            {
                tree.borrow_mut().set_offset(binding.widget, x, y);
            }
        }
    }
}

fn text(component: &RefCell<Component>, expression: &Expr) -> Option<String> {
    match evaluate(component, expression)? {
        Value::String(content) => Some(content),
        _ => None,
    }
}

fn boolean(component: &RefCell<Component>, expression: &Expr) -> Option<bool> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}

fn offset(component: &RefCell<Component>, expression: Option<&Expr>) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    match evaluate(component, expression)? {
        Value::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= f64::from(i32::MIN)
                && value <= f64::from(i32::MAX) =>
        {
            Some(value as i32)
        }
        _ => None,
    }
}

fn evaluate(component: &RefCell<Component>, expression: &Expr) -> Option<Value> {
    eval(&mut component.borrow_mut(), expression).ok()
}

/// Загружает `.brul` и открывает его окно.
pub fn run(source: &str, theme: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let Loaded {
        config,
        tree,
        bindings,
        component,
        ..
    } = brul_compiler::load(source, theme)?;
    let ui = Ui::new(component, tree, bindings);
    window::run_tree(config, Rc::clone(&ui.tree), |_| {})?;
    Ok(())
}
