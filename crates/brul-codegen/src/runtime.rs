use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Widget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Value, Variable, eval, execute_program};
use brul_syntax::ast::Expr;
use brul_syntax::markup::PropType;
use brul_syntax::script::Program;
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

impl Binding {
    pub fn new(component: &ComponentHandle, widget: WidgetId, target: BindingTarget) -> Self {
        Self {
            widget,
            target,
            scopes: component.borrow().build_scopes(),
            component: Rc::clone(component),
        }
    }
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

impl Handler {
    pub fn new(
        component: &ComponentHandle,
        widget: Option<WidgetId>,
        event: HandlerEvent,
        function: &str,
    ) -> Self {
        Self {
            widget,
            event,
            function: function.to_owned(),
            component: Rc::clone(component),
        }
    }
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

pub type StructureValue = fn(&ComponentHandle) -> Option<Value>;
pub type Rebuilt = Result<(Built, Vec<RowInfo>), String>;
pub type StructureRebuild = fn(Option<&str>, &ComponentHandle) -> Rebuilt;

#[derive(Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub each: bool,
    pub value: StructureValue,
    pub rebuild: StructureRebuild,
}

#[derive(Debug, Clone)]
pub struct RowInfo {
    pub key: Option<Value>,
    pub item: Value,
    pub start: usize,
    pub count: usize,
}

pub struct Built {
    pub widget: Widget,
    pub bindings: Vec<Binding>,
    pub handlers: Vec<Handler>,
    pub structures: Vec<Structure>,
    pub props: Vec<PropBinding>,
}

pub type View = fn(Option<&str>, &ComponentHandle) -> Result<Built, String>;

pub struct PropArgument {
    pub name: &'static str,
    pub ty: PropType,
    pub expression: Expr,
}

pub struct Instance<'a> {
    pub theme: Option<&'a str>,
    pub view: View,
    pub script: Option<&'a Program>,
    pub name: &'static str,
    pub props: &'a [PropArgument],
    pub defaults: &'a [PropArgument],
}

pub fn instance(parent: &ComponentHandle, instance: Instance<'_>) -> Result<Built, String> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    if let Some(program) = instance.script {
        execute_program(&mut component.borrow_mut(), program).map_err(|error| error.reason())?;
    }
    let mut given: Vec<&str> = Vec::new();
    for argument in instance.props {
        let value = evaluate_argument(parent, argument)?;
        assign(&component, argument, value, instance.name)?;
        given.push(argument.name);
    }
    for argument in instance.defaults {
        if given.contains(&argument.name) {
            continue;
        }
        let value = evaluate_argument(&component, argument)?;
        assign(&component, argument, value, instance.name)?;
    }
    let mut built = (instance.view)(instance.theme, &component)?;
    let widget = built.widget.id();
    for argument in instance.props {
        built.props.push(PropBinding {
            widget,
            target: Rc::clone(&component),
            name: argument.name.to_owned(),
            ty: argument.ty,
            expression: argument.expression.clone(),
            scopes: parent.borrow().build_scopes(),
            component: Rc::clone(parent),
        });
    }
    Ok(built)
}

fn evaluate_argument(
    component: &ComponentHandle,
    argument: &PropArgument,
) -> Result<Value, String> {
    eval(&mut component.borrow_mut(), &argument.expression).map_err(|error| error.reason())
}

fn assign(
    component: &ComponentHandle,
    argument: &PropArgument,
    value: Value,
    name: &str,
) -> Result<(), String> {
    if !accepts_prop(&value, argument.ty) {
        return Err(format!(
            "property '{}' of component '{name}' expects {}, got {}",
            argument.name,
            prop_name(argument.ty),
            value.type_name()
        ));
    }
    component
        .borrow_mut()
        .declare_property(argument.name, value)
        .map_err(|error| error.reason())
}

pub fn accepts_prop(value: &Value, ty: PropType) -> bool {
    match ty {
        PropType::Bool => matches!(value, Value::Bool(_)),
        PropType::Number => matches!(value, Value::Number(_)),
        PropType::String => matches!(value, Value::String(_)),
        PropType::Image => matches!(value, Value::Null),
    }
}

pub fn prop_name(ty: PropType) -> &'static str {
    match ty {
        PropType::Bool => "Bool",
        PropType::Number => "Number",
        PropType::String => "String",
        PropType::Image => "Image",
    }
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
