//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::RefCell;
use std::rc::Rc;

use brul_compiler::{Binding, BindingTarget, Handler, HandlerEvent, Loaded};
use brul_gui::{Widget, WidgetEvent, WidgetId, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Value, eval};
use brul_syntax::ast::Expr;

pub struct Ui {
    pub component: Rc<RefCell<Component>>,
    pub tree: Rc<RefCell<WidgetTree>>,
    handlers: Vec<Handler>,
    _scope: EffectScope,
}

impl Ui {
    pub fn new(
        component: Component,
        widget: Widget,
        bindings: Vec<Binding>,
        handlers: Vec<Handler>,
    ) -> Self {
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
            handlers,
            _scope: scope,
        }
    }

    pub fn dispatch(&mut self, events: &[WidgetEvent]) {
        for event in events {
            let Some((widget, target)) = handler_target(*event) else {
                continue;
            };
            let Some(handler) = self
                .handlers
                .iter()
                .find(|handler| handler.widget == widget && handler.event == target)
            else {
                continue;
            };
            let function = handler.function.clone();
            if let Err(error) = self
                .component
                .borrow_mut()
                .call_function(&function, Vec::new())
            {
                eprintln!("{error}");
            }
        }
    }
}

fn handler_target(event: WidgetEvent) -> Option<(WidgetId, HandlerEvent)> {
    match event {
        WidgetEvent::Click(id) => Some((id, HandlerEvent::Click)),
        WidgetEvent::Press(id) => Some((id, HandlerEvent::Press)),
        WidgetEvent::Release(id) => Some((id, HandlerEvent::Release)),
        WidgetEvent::PointerEnter(id) => Some((id, HandlerEvent::Enter)),
        WidgetEvent::PointerLeave(id) => Some((id, HandlerEvent::Leave)),
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
        handlers,
        component,
        ..
    } = brul_compiler::load(source, theme)?;
    let mut ui = Ui::new(component, tree, bindings, handlers);
    let tree = Rc::clone(&ui.tree);
    window::run_tree(config, tree, move |events| ui.dispatch(events))?;
    Ok(())
}
