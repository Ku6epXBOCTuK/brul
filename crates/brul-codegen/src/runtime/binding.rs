use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use brul_gui::{Class, Image, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Dyn, Error as ScriptError, Scope, Value, VariableId};
use brul_syntax::markup::PropType;
use tracing::error;

use super::ComponentHandle;
use super::error::ViewError;
use super::paint::{Textures, set_flag, set_input, set_offset, set_text, set_texture, texture};

pub type Expression = fn(&mut Component) -> Result<Dyn, ScriptError>;
pub type TextExpression = fn(&mut Component) -> Result<String, ScriptError>;
pub type FlagExpression = fn(&mut Component) -> Result<bool, ScriptError>;
pub type NumberExpression = fn(&mut Component) -> Result<f64, ScriptError>;
pub type ClassTable = fn(&str) -> Option<Class>;

#[derive(Clone, Copy)]
pub enum PropExpression {
    Number(NumberExpression),
    Text(TextExpression),
    Flag(FlagExpression),
    Image(Expression),
}

impl PropExpression {
    pub fn evaluate(self, component: &mut Component) -> Result<Value, ScriptError> {
        Ok(match self {
            Self::Number(expression) => Value::Number(expression(component)?),
            Self::Text(expression) => Value::String(expression(component)?.into()),
            Self::Flag(expression) => Value::Bool(expression(component)?),
            Self::Image(expression) => Value::from(expression(component)?),
        })
    }
}

#[derive(Clone)]
pub struct Binding {
    pub widget: WidgetId,
    pub target: BindingTarget,
    pub scope: Scope,
    pub component: ComponentHandle,
}

impl Binding {
    pub fn new(component: &ComponentHandle, widget: WidgetId, target: BindingTarget) -> Self {
        Self {
            widget,
            target,
            scope: component.borrow().scope().clone(),
            component: Rc::clone(component),
        }
    }
}

#[derive(Clone)]
pub struct PropBinding {
    pub widget: WidgetId,
    pub target: ComponentHandle,
    pub name: VariableId,
    pub ty: PropType,
    pub expression: PropExpression,
    pub scope: Scope,
    pub component: ComponentHandle,
}

#[derive(Debug, Clone)]
pub enum BindingTarget {
    Text(TextExpression),
    Input(TextExpression),
    Image(Expression),
    Checked(FlagExpression),
    Selected(FlagExpression),
    Disabled(FlagExpression),
    Visible(FlagExpression),
    Offset {
        x: Option<NumberExpression>,
        y: Option<NumberExpression>,
    },
    Class {
        table: ClassTable,
        expression: Expression,
    },
}

pub struct Bindings {
    _scope: EffectScope,
}

impl Bindings {
    pub fn new(bindings: &[Binding], tree: &Rc<RefCell<WidgetTree>>, textures: &Textures) -> Self {
        let scope = EffectScope::new();
        for binding in bindings {
            let binding = binding.clone();
            let tree = Rc::clone(tree);
            let textures = Rc::clone(textures);
            scope.run(|| {
                Effect::new(move || {
                    let pending = evaluate(&binding, &textures);
                    write(pending, binding.widget, &mut tree.borrow_mut());
                });
            });
        }
        Self { _scope: scope }
    }
}

pub enum Write {
    Text(Option<String>),
    Input(Option<String>),
    Image(Option<Image>),
    Checked(Option<bool>),
    Selected(Option<bool>),
    Disabled(Option<bool>),
    Visible(Option<bool>),
    Offset(Option<i32>, Option<i32>),
    Class(Option<Box<Class>>),
}

pub fn evaluate(binding: &Binding, textures: &Textures) -> Write {
    binding
        .component
        .borrow_mut()
        .set_scope(binding.scope.clone());
    let component = &binding.component;
    match &binding.target {
        BindingTarget::Text(expression) => Write::Text(string(component, *expression)),
        BindingTarget::Input(expression) => Write::Input(string(component, *expression)),
        BindingTarget::Image(expression) => Write::Image(texture(component, *expression, textures)),
        BindingTarget::Checked(expression) => Write::Checked(flag(component, *expression)),
        BindingTarget::Selected(expression) => Write::Selected(flag(component, *expression)),
        BindingTarget::Disabled(expression) => Write::Disabled(flag(component, *expression)),
        BindingTarget::Visible(expression) => Write::Visible(flag(component, *expression)),
        BindingTarget::Offset { x, y } => {
            Write::Offset(offset_value(component, *x), offset_value(component, *y))
        }
        BindingTarget::Class { table, expression } => {
            Write::Class(class_value(component, *table, *expression))
        }
    }
}

pub fn write(write: Write, widget: WidgetId, tree: &mut WidgetTree) {
    match write {
        Write::Text(value) => set_text(tree, widget, value),
        Write::Input(value) => set_input(tree, widget, value),
        Write::Image(value) => set_texture(tree, widget, value),
        Write::Checked(value) => set_flag(tree, widget, value, WidgetTree::set_checked),
        Write::Selected(value) => set_flag(tree, widget, value, WidgetTree::set_selected),
        Write::Disabled(value) => set_flag(tree, widget, value, WidgetTree::set_disabled),
        Write::Visible(value) => set_flag(tree, widget, value, WidgetTree::set_visible),
        Write::Offset(x, y) => set_offset(tree, widget, x, y),
        Write::Class(value) => {
            tree.apply_class(widget, value.as_deref());
        }
    }
}

fn evaluated<T>(
    component: &RefCell<Component>,
    expression: fn(&mut Component) -> Result<T, ScriptError>,
) -> Option<T> {
    match expression(&mut component.borrow_mut()) {
        Ok(value) => Some(value),
        Err(error) => {
            error!(%error, "binding expression failed");
            None
        }
    }
}

fn value(component: &RefCell<Component>, expression: Expression) -> Option<Value> {
    evaluated(component, expression).map(Value::from)
}

fn string(component: &RefCell<Component>, expression: TextExpression) -> Option<String> {
    evaluated(component, expression)
}

fn flag(component: &RefCell<Component>, expression: FlagExpression) -> Option<bool> {
    evaluated(component, expression)
}

thread_local! {
    static UNKNOWN_CLASSES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn class_value(
    component: &RefCell<Component>,
    table: ClassTable,
    expression: Expression,
) -> Option<Box<Class>> {
    let name = match value(component, expression)? {
        Value::Null => return None,
        value => value.as_text()?.to_owned(),
    };
    if name.is_empty() {
        return None;
    }
    let found = table(&name);
    if found.is_none() {
        UNKNOWN_CLASSES.with(|seen| {
            if seen.borrow_mut().insert(name.clone()) {
                error!(class = %name, "unknown class");
            }
        });
    }
    found.map(Box::new)
}

pub fn text(
    component: &mut Component,
    expression: TextExpression,
    name: &str,
) -> Result<String, ViewError> {
    expression(component).map_err(|_| {
        ViewError::expected(
            "Text content",
            "String",
            component
                .read(name)
                .map_or_else(|_| "unknown", |value| value.type_name()),
        )
    })
}

pub fn boolean(
    component: &mut Component,
    expression: FlagExpression,
    name: &str,
) -> Result<bool, ViewError> {
    expression(component).map_err(|_| {
        ViewError::expected(
            format!("parameter '{name}'"),
            "Bool",
            component
                .read(name)
                .map_or_else(|_| "unknown", |value| value.type_name()),
        )
    })
}

fn pixels(value: f64) -> Option<i32> {
    if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return None;
    }
    Some(value.round() as i32)
}

pub fn offset(
    component: &mut Component,
    expression: Option<NumberExpression>,
    name: &str,
) -> Result<i32, ViewError> {
    let Some(expression) = expression else {
        return Ok(0);
    };
    pixels(expression(component)?)
        .ok_or_else(|| ViewError::expected(format!("parameter '{name}'"), "Number", "out of range"))
}

pub(crate) fn offset_value(
    component: &RefCell<Component>,
    expression: Option<NumberExpression>,
) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    evaluated(component, expression).and_then(pixels)
}
