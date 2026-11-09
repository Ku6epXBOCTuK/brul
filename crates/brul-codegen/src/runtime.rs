use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Widget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Value, eval};
use brul_syntax::ast::Expr;

#[derive(Clone)]
pub enum Target {
    Text(Expr),
    Checked(Expr),
    Selected(Expr),
    Disabled(Expr),
    Visible(Expr),
    Offset { x: Option<Expr>, y: Option<Expr> },
}

#[derive(Clone)]
pub struct Binding {
    pub widget: WidgetId,
    pub target: Target,
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
}

pub struct Bindings {
    _scope: EffectScope,
}

impl Bindings {
    pub fn new(
        bindings: &[Binding],
        component: &Rc<RefCell<Component>>,
        tree: &Rc<RefCell<WidgetTree>>,
    ) -> Self {
        let scope = EffectScope::new();
        for binding in bindings {
            let binding = binding.clone();
            let component = Rc::clone(component);
            let tree = Rc::clone(tree);
            scope.run(|| {
                Effect::new(move || apply(&binding, &component, &tree));
            });
        }
        Self { _scope: scope }
    }
}

fn apply(binding: &Binding, component: &RefCell<Component>, tree: &RefCell<WidgetTree>) {
    component.borrow().track();
    match &binding.target {
        Target::Text(expression) => {
            if let Ok(Value::String(content)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_text(binding.widget, content);
            }
        }
        Target::Checked(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_checked(binding.widget, value);
            }
        }
        Target::Selected(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_selected(binding.widget, value);
            }
        }
        Target::Disabled(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_disabled(binding.widget, value);
            }
        }
        Target::Visible(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_visible(binding.widget, value);
            }
        }
        Target::Offset { x, y } => {
            if let (Some(x), Some(y)) = (
                offset_value(component, x.as_ref()),
                offset_value(component, y.as_ref()),
            ) {
                tree.borrow_mut().set_offset(binding.widget, x, y);
            }
        }
    }
}

pub fn text(component: &mut Component, expression: &Expr) -> Result<String, String> {
    match evaluate(component, expression)? {
        Value::String(content) => Ok(content),
        other => Err(format!(
            "Text content expects String, got {}",
            other.type_name()
        )),
    }
}

pub fn boolean(component: &mut Component, expression: &Expr, name: &str) -> Result<bool, String> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Ok(value),
        other => Err(format!(
            "parameter '{name}' expects Bool, got {}",
            other.type_name()
        )),
    }
}

pub fn offset(
    component: &mut Component,
    expression: Option<&Expr>,
    name: &str,
) -> Result<i32, String> {
    let Some(expression) = expression else {
        return Ok(0);
    };
    match evaluate(component, expression)? {
        Value::Number(value) if is_offset(value) => Ok(value as i32),
        _ => Err(format!("parameter '{name}' expects an integer Number")),
    }
}

fn evaluate(component: &mut Component, expression: &Expr) -> Result<Value, String> {
    eval(component, expression).map_err(|error| error.reason())
}

fn offset_value(component: &RefCell<Component>, expression: Option<&Expr>) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    match eval(&mut component.borrow_mut(), expression).ok()? {
        Value::Number(value) if is_offset(value) => Some(value as i32),
        _ => None,
    }
}

fn is_offset(value: f64) -> bool {
    value.is_finite()
        && value.fract() == 0.0
        && value >= f64::from(i32::MIN)
        && value <= f64::from(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_syntax::ast::{ExprKind, Position};

    fn ident(name: &str) -> Expr {
        Expr::new(ExprKind::Ident(name.to_owned()), Position::default())
    }

    #[test]
    fn text_binding_follows_the_component() {
        let mut component = Component::new();
        component
            .declare("name", Value::String("hello".to_owned()), false)
            .unwrap();
        let widget = Widget::text("");
        let id = widget.id();
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        let component = Rc::new(RefCell::new(component));
        let bindings = [Binding {
            widget: id,
            target: Target::Text(ident("name")),
        }];

        let _bindings = Bindings::new(&bindings, &component, &tree);
        assert!(!tree.borrow_mut().set_text(id, "hello".to_owned()));

        component
            .borrow_mut()
            .assign("name", Value::String("bye".to_owned()))
            .unwrap();
        brul_reactive::flush();
        assert!(!tree.borrow_mut().set_text(id, "bye".to_owned()));
    }
}
