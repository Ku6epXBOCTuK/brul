use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Direction, Widget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Value, Variable, eval};
use brul_syntax::ast::Expr;
use brul_syntax::markup::{EachBlock, IfBlock, PropType};
use brul_syntax::style::Stylesheet;

use crate::{Import, Registry};

pub type ComponentHandle = Rc<RefCell<Component>>;

#[derive(Clone)]
pub struct Context {
    pub registry: Rc<Registry>,
    pub imports: Vec<Import>,
    pub stylesheet: Rc<Stylesheet>,
}

#[derive(Clone)]
pub struct Binding {
    pub widget: WidgetId,
    pub target: BindingTarget,
    pub scopes: Vec<Vec<Variable>>,
    pub component: ComponentHandle,
}

#[derive(Clone)]
pub struct PropBinding {
    pub widget: WidgetId,
    pub target: ComponentHandle,
    pub name: String,
    pub ty: PropType,
    pub expression: Expr,
    pub scopes: Vec<Vec<Variable>>,
    pub component: ComponentHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerEvent {
    Click,
    Press,
    Release,
    Enter,
    Leave,
    Drag,
    Key,
}

impl HandlerEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Press => "press",
            Self::Release => "release",
            Self::Enter => "enter",
            Self::Leave => "leave",
            Self::Drag => "drag",
            Self::Key => "key",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "click" => Some(Self::Click),
            "press" => Some(Self::Press),
            "release" => Some(Self::Release),
            "enter" => Some(Self::Enter),
            "leave" => Some(Self::Leave),
            "drag" => Some(Self::Drag),
            "key" => Some(Self::Key),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Handler {
    pub widget: Option<WidgetId>,
    pub event: HandlerEvent,
    pub function: String,
    pub component: ComponentHandle,
}

#[derive(Debug, Clone)]
pub enum BindingTarget {
    Text(Expr),
    Checked(Expr),
    Selected(Expr),
    Disabled(Expr),
    Visible(Expr),
    Offset { x: Option<Expr>, y: Option<Expr> },
}

#[derive(Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub direction: Direction,
    pub spacing: u32,
    pub kind: StructureKind,
    pub component: ComponentHandle,
    pub context: Context,
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub key: Option<Value>,
    pub item: Value,
    pub start: usize,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub enum StructureKind {
    If {
        block: IfBlock,
        last: bool,
    },
    Each {
        block: EachBlock,
        last: Vec<Value>,
        rows: Vec<RowInfo>,
    },
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
    pub structures: Vec<Structure>,
    pub props: Vec<PropBinding>,
}

pub struct Bindings {
    _scope: EffectScope,
}

impl Bindings {
    pub fn new(bindings: &[Binding], tree: &Rc<RefCell<WidgetTree>>) -> Self {
        let scope = EffectScope::new();
        for binding in bindings {
            let binding = binding.clone();
            let tree = Rc::clone(tree);
            scope.run(|| {
                Effect::new(move || apply(&binding, &tree));
            });
        }
        Self { _scope: scope }
    }
}

fn apply(binding: &Binding, tree: &RefCell<WidgetTree>) {
    let component = Rc::clone(&binding.component);
    component.borrow().track();
    let count = binding.scopes.len();
    component.borrow_mut().push_binding_scopes(&binding.scopes);
    match &binding.target {
        BindingTarget::Text(expression) => {
            if let Ok(Value::String(content)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_text(binding.widget, content);
            }
        }
        BindingTarget::Checked(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_checked(binding.widget, value);
            }
        }
        BindingTarget::Selected(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_selected(binding.widget, value);
            }
        }
        BindingTarget::Disabled(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_disabled(binding.widget, value);
            }
        }
        BindingTarget::Visible(expression) => {
            if let Ok(Value::Bool(value)) = eval(&mut component.borrow_mut(), expression) {
                tree.borrow_mut().set_visible(binding.widget, value);
            }
        }
        BindingTarget::Offset { x, y } => {
            if let (Some(x), Some(y)) = (
                offset_value(&component, x.as_ref()),
                offset_value(&component, y.as_ref()),
            ) {
                tree.borrow_mut().set_offset(binding.widget, x, y);
            }
        }
    }
    component.borrow_mut().pop_binding_scopes(count);
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
            target: BindingTarget::Text(ident("name")),
            scopes: Vec::new(),
            component: Rc::clone(&component),
        }];

        let _bindings = Bindings::new(&bindings, &tree);
        assert!(!tree.borrow_mut().set_text(id, "hello".to_owned()));

        component
            .borrow_mut()
            .assign("name", Value::String("bye".to_owned()))
            .unwrap();
        brul_reactive::flush();
        assert!(!tree.borrow_mut().set_text(id, "bye".to_owned()));
    }
}
