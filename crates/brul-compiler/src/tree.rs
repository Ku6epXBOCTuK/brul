use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Direction, LayoutStyle, Widget, WidgetId};
use brul_script::{Component, Error as ScriptError, Host, Value, Variable, eval, execute_program};
use brul_syntax::ast::{Expr, Position};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, BlockItem, EachBlock, Element, IfBlock, Member, PropType};
use brul_syntax::style::Stylesheet;

use crate::loader::{Definition, Import, Registry};
use crate::styles::{self, Resolved};

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

    pub(crate) fn parse(name: &str) -> Option<Self> {
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

type Parts = (
    Widget,
    Vec<Binding>,
    Vec<Handler>,
    Vec<Structure>,
    Vec<PropBinding>,
);
type Children = (
    Vec<Widget>,
    Vec<Binding>,
    Vec<Handler>,
    Vec<Structure>,
    Vec<PropBinding>,
);

pub fn build_tree(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
    component: &mut Component,
) -> Result<Widget, Error> {
    let cell: ComponentHandle = Rc::new(RefCell::new(std::mem::take(component)));
    let context = Context {
        registry: Rc::new(Registry {
            definitions: Vec::new(),
            imports: Vec::new(),
        }),
        imports: Vec::new(),
        stylesheet: Rc::new(stylesheet.clone()),
    };
    let built = build_bound_tree(element, &cell, &context, theme);
    let widget = built.map(|built| built.widget);
    *component = Rc::try_unwrap(cell)
        .map(RefCell::into_inner)
        .unwrap_or_default();
    widget
}

pub fn build_bound_tree(
    element: &Element,
    component: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
) -> Result<Built, Error> {
    let resolved = resolve(element, &context.stylesheet, theme)?;

    let (widget, bindings, handlers, structures, props) = match element.name.as_str() {
        "View" | "Row" | "Column" => {
            let direction = if element.name == "Row" {
                Direction::Row
            } else {
                Direction::Column
            };
            container(direction, element, component, context, theme, resolved)?
        }
        "Text" => {
            let (widget, expression) = text(element, component, resolved)?;
            let binding = Binding {
                widget: widget.id(),
                target: BindingTarget::Text(expression),
                scopes: component.borrow().build_scopes(),
                component: Rc::clone(component),
            };
            (widget, vec![binding], Vec::new(), Vec::new(), Vec::new())
        }
        "Image" => {
            return Err(error_at(
                element.position,
                "widget 'Image' is not supported yet",
            ));
        }
        name => {
            let Some(import) = context.imports.iter().find(|import| import.name == name) else {
                return Err(error_at(
                    element.position,
                    format!("unknown widget '{name}'"),
                ));
            };
            return instance(element, component, context, theme, import.definition);
        }
    };

    let mut built = parameters(element, component, widget)?;
    built.bindings.extend(bindings);
    built.handlers.extend(handlers);
    built.structures.extend(structures);
    built.props.extend(props);
    Ok(built)
}

fn instance(
    element: &Element,
    parent: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
    index: usize,
) -> Result<Built, Error> {
    let definition = &context.registry.definitions[index];
    if !element.block.items.is_empty() {
        return Err(error_at(
            element.position,
            format!("component '{}' does not accept children", definition.name),
        ));
    }

    let handlers = instance_handlers(element, parent, definition)?;
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    if !handlers.is_empty() {
        component
            .borrow_mut()
            .set_host(EventHost { events: handlers });
    }
    if let Some(program) = &definition.program {
        execute_program(&mut component.borrow_mut(), program).map_err(script_error)?;
    }

    let mut given: Vec<&str> = Vec::new();
    let mut reactive: Vec<(String, PropType, Expr, Vec<Vec<Variable>>)> = Vec::new();
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {
                return Err(error_at(
                    element.position,
                    format!(
                        "component '{}' does not accept classes yet",
                        definition.name
                    ),
                ));
            }
            Arg::Handler { .. } => {}
            Arg::Named { name, value } => {
                if given.contains(&name.as_str()) {
                    return Err(error_at(
                        value.position,
                        format!("property '{name}' is given twice"),
                    ));
                }
                let Some(prop) =
                    definition
                        .component
                        .members
                        .iter()
                        .find_map(|member| match member {
                            Member::Prop(prop) if prop.name == *name => Some(prop),
                            _ => None,
                        })
                else {
                    return Err(error_at(
                        value.position,
                        format!("component '{}' has no property '{name}'", definition.name),
                    ));
                };
                let position = value.position;
                let expression = value.clone();
                let value = evaluate(parent, value)?;
                let value = typed(value, prop.ty, name, &definition.name, position)?;
                component
                    .borrow_mut()
                    .declare_property(name, value)
                    .map_err(|error| error_at(position, error.reason()))?;
                reactive.push((
                    name.clone(),
                    prop.ty,
                    expression,
                    parent.borrow().build_scopes(),
                ));
                given.push(name);
            }
        }
    }

    for member in &definition.component.members {
        let Member::Prop(prop) = member else {
            continue;
        };
        if given.contains(&prop.name.as_str()) {
            continue;
        }
        let Some(default) = &prop.default else {
            return Err(error_at(
                element.position,
                format!(
                    "property '{}' of component '{}' requires a default value",
                    prop.name, definition.name
                ),
            ));
        };
        let value = evaluate(&component, default)?;
        let value = typed(
            value,
            prop.ty,
            &prop.name,
            &definition.name,
            default.position,
        )?;
        component
            .borrow_mut()
            .declare_property(&prop.name, value)
            .map_err(|error| error_at(default.position, error.reason()))?;
    }

    let mut roots = definition
        .component
        .members
        .iter()
        .filter_map(|member| match member {
            Member::Element(element) => Some(element),
            Member::Prop(_) | Member::Event(_) => None,
        });
    let Some(root) = roots.next() else {
        return Err(error_at(
            element.position,
            format!("component '{}' has no root element", definition.name),
        ));
    };
    if roots.next().is_some() {
        return Err(error_at(
            element.position,
            format!(
                "component '{}' has more than one root element",
                definition.name
            ),
        ));
    }

    let context = Context {
        registry: Rc::clone(&context.registry),
        imports: definition.imports.clone(),
        stylesheet: Rc::clone(&definition.stylesheet),
    };
    let mut built = build_bound_tree(root, &component, &context, theme)?;
    let widget = built.widget.id();
    for (name, ty, expression, scopes) in reactive {
        built.props.push(PropBinding {
            widget,
            target: Rc::clone(&component),
            name,
            ty,
            expression,
            scopes,
            component: Rc::clone(parent),
        });
    }
    Ok(built)
}

struct Emitted {
    event: String,
    handler: String,
    params: Vec<PropType>,
    component: ComponentHandle,
}

fn instance_handlers(
    element: &Element,
    parent: &ComponentHandle,
    definition: &Definition,
) -> Result<Vec<Emitted>, Error> {
    let mut emitted: Vec<Emitted> = Vec::new();
    for arg in &element.args {
        let Arg::Handler { event, handler } = arg else {
            continue;
        };
        if emitted.iter().any(|item| item.event == *event) {
            return Err(error_at(
                element.position,
                format!("event '{event}' is handled twice"),
            ));
        }
        let Some(declaration) =
            definition
                .component
                .members
                .iter()
                .find_map(|member| match member {
                    Member::Event(item) if item.name == *event => Some(item),
                    _ => None,
                })
        else {
            return Err(error_at(
                element.position,
                format!("component '{}' has no event '{event}'", definition.name),
            ));
        };
        if !parent.borrow().has_function(handler) {
            return Err(error_at(
                element.position,
                format!("function '{handler}' is not defined"),
            ));
        }
        emitted.push(Emitted {
            event: event.clone(),
            handler: handler.clone(),
            params: declaration.params.iter().map(|param| param.ty).collect(),
            component: Rc::clone(parent),
        });
    }
    Ok(emitted)
}

struct EventHost {
    events: Vec<Emitted>,
}

impl Host for EventHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, ScriptError> {
        let Some(event) = self.events.iter().find(|event| event.event == name) else {
            return Err(ScriptError::UnknownFunction(name.to_owned()));
        };
        if event.params.len() != arguments.len() {
            return Err(ScriptError::FunctionArity {
                name: event.event.clone(),
                expected: event.params.len(),
                found: arguments.len(),
            });
        }
        for (ty, value) in event.params.iter().zip(arguments) {
            if !accepts_prop(value, *ty) {
                return Err(ScriptError::EventArgument {
                    name: event.event.clone(),
                    expected: prop_name(*ty),
                    found: value.type_name(),
                });
            }
        }
        let component = Rc::clone(&event.component);
        component
            .borrow_mut()
            .call_function(&event.handler, arguments.to_vec())
    }
}

fn prop_name(ty: PropType) -> &'static str {
    match ty {
        PropType::Bool => "Bool",
        PropType::Number => "Number",
        PropType::String => "String",
        PropType::Image => "Image",
    }
}

pub fn accepts_prop(value: &Value, ty: PropType) -> bool {
    match ty {
        PropType::Bool => matches!(value, Value::Bool(_)),
        PropType::Number => matches!(value, Value::Number(_)),
        PropType::String => matches!(value, Value::String(_)),
        PropType::Image => matches!(value, Value::Null),
    }
}

fn typed(
    value: Value,
    ty: PropType,
    name: &str,
    component: &str,
    position: Position,
) -> Result<Value, Error> {
    if accepts_prop(&value, ty) {
        return Ok(value);
    }
    let expected = match ty {
        PropType::Bool => "Bool",
        PropType::Number => "Number",
        PropType::String => "String",
        PropType::Image => "Image",
    };
    Err(error_at(
        position,
        format!(
            "property '{name}' of component '{component}' expects {expected}, got {}",
            value.type_name()
        ),
    ))
}

fn resolve(
    element: &Element,
    stylesheet: &Stylesheet,
    theme: Option<&str>,
) -> Result<Resolved, Error> {
    let classes: Vec<&str> = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(class) => Some(class.as_str()),
            _ => None,
        })
        .collect();
    styles::resolve(stylesheet, &classes, theme)
        .map_err(|message| error_at(element.position, message))
}

fn parameters(
    element: &Element,
    component: &ComponentHandle,
    widget: Widget,
) -> Result<Built, Error> {
    let mut widget = widget;
    let id = widget.id();
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut x_value = None;
    let mut y_value = None;
    let mut x = None;
    let mut y = None;
    for arg in &element.args {
        match arg {
            Arg::Class(_) => {}
            Arg::Named { name, value } => match name.as_str() {
                "checked" => {
                    widget = widget.checked(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Checked(value.clone()),
                        scopes: component.borrow().build_scopes(),
                        component: Rc::clone(component),
                    });
                }
                "selected" => {
                    widget = widget.selected(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Selected(value.clone()),
                        scopes: component.borrow().build_scopes(),
                        component: Rc::clone(component),
                    });
                }
                "disabled" => {
                    widget = widget.disabled(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Disabled(value.clone()),
                        scopes: component.borrow().build_scopes(),
                        component: Rc::clone(component),
                    });
                }
                "visible" => {
                    widget = widget.visible(boolean(component, value, name)?);
                    bindings.push(Binding {
                        widget: id,
                        target: BindingTarget::Visible(value.clone()),
                        scopes: component.borrow().build_scopes(),
                        component: Rc::clone(component),
                    });
                }
                "x" => {
                    x_value = Some(integer(component, value, name)?);
                    x = Some(value.clone());
                }
                "y" => {
                    y_value = Some(integer(component, value, name)?);
                    y = Some(value.clone());
                }
                _ => {
                    return Err(error_at(
                        value.position,
                        format!("parameter '{name}' is not supported yet"),
                    ));
                }
            },
            Arg::Handler { event, handler } => {
                let Some(event) = HandlerEvent::parse(event) else {
                    return Err(error_at(
                        element.position,
                        format!("event '{event}' is not supported yet"),
                    ));
                };
                if event == HandlerEvent::Key {
                    return Err(error_at(
                        element.position,
                        "key events are only supported on the window",
                    ));
                }
                if event == HandlerEvent::Drag {
                    if handler != "dragWindow" {
                        return Err(error_at(
                            element.position,
                            "drag handler must be dragWindow",
                        ));
                    }
                    widget = widget.draggable(true);
                    continue;
                }
                if handlers.iter().any(|item: &Handler| item.event == event) {
                    return Err(error_at(
                        element.position,
                        format!("event '{}' is handled twice", event.name()),
                    ));
                }
                if !component.borrow().has_function(handler) {
                    return Err(error_at(
                        element.position,
                        format!("function '{handler}' is not defined"),
                    ));
                }
                handlers.push(Handler {
                    widget: Some(id),
                    event,
                    function: handler.clone(),
                    component: Rc::clone(component),
                });
            }
        }
    }
    if x.is_some() || y.is_some() {
        widget = widget.with_offset(x_value.unwrap_or(0), y_value.unwrap_or(0));
        bindings.push(Binding {
            widget: id,
            target: BindingTarget::Offset { x, y },
            scopes: component.borrow().build_scopes(),
            component: Rc::clone(component),
        });
    }
    Ok(Built {
        widget,
        bindings,
        handlers,
        structures: Vec::new(),
        props: Vec::new(),
    })
}

fn container(
    direction: Direction,
    element: &Element,
    component: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
    resolved: Resolved,
) -> Result<Parts, Error> {
    let built = build_items(
        &element.block.items,
        component,
        context,
        theme,
        element.position,
        direction,
        resolved.layout.spacing,
    )?;
    let mut children = Vec::with_capacity(built.len());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    let mut props = Vec::new();
    for child in built {
        children.push(child.widget);
        bindings.extend(child.bindings);
        handlers.extend(child.handlers);
        structures.extend(child.structures);
        props.extend(child.props);
    }
    let widget = Widget::view(direction, children)
        .with_layout(resolved.layout)
        .with_styles(resolved.styles);
    Ok((widget, bindings, handlers, structures, props))
}

fn build_items(
    items: &[BlockItem],
    component: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<Vec<Built>, Error> {
    let mut built = Vec::new();
    for item in items {
        match item {
            BlockItem::Element(child) => {
                built.push(build_bound_tree(child, component, context, theme)?);
            }
            BlockItem::If(block) => {
                let (mut node, last) = build_if(
                    block, component, context, theme, position, direction, spacing,
                )?;
                node.structures.push(Structure {
                    parent: node.widget.id(),
                    direction,
                    spacing,
                    kind: StructureKind::If {
                        block: block.clone(),
                        last,
                    },
                    component: Rc::clone(component),
                    context: context.clone(),
                });
                built.push(node);
            }
            BlockItem::Each(block) => {
                let (mut node, last, rows) = build_each(
                    block, component, context, theme, position, direction, spacing,
                )?;
                node.structures.push(Structure {
                    parent: node.widget.id(),
                    direction,
                    spacing,
                    kind: StructureKind::Each {
                        block: block.clone(),
                        last,
                        rows,
                    },
                    component: Rc::clone(component),
                    context: context.clone(),
                });
                built.push(node);
            }
            BlockItem::Expr(expression) => {
                return Err(error_at(
                    expression.position,
                    "unexpected expression in a container",
                ));
            }
        }
    }
    Ok(built)
}

fn build_if(
    block: &IfBlock,
    component: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<(Built, bool), Error> {
    let condition = match evaluate(component, &block.condition)? {
        Value::Bool(condition) => condition,
        other => {
            return Err(error_at(
                block.condition.position,
                format!("if condition expects Bool, got {}", other.type_name()),
            ));
        }
    };
    let branch = if condition {
        Some(&block.then_block)
    } else {
        block.else_block.as_ref()
    };
    let (children, bindings, handlers, structures, props) = match branch {
        Some(branch) => flatten(build_items(
            &branch.items,
            component,
            context,
            theme,
            position,
            direction,
            spacing,
        )?),
        None => (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    };
    let mut wrapper = Widget::view(direction, Vec::new()).with_layout(LayoutStyle {
        spacing,
        ..Default::default()
    });
    wrapper.set_children(children);
    Ok((
        Built {
            widget: wrapper,
            bindings,
            handlers,
            structures,
            props,
        },
        condition,
    ))
}

fn flatten(built: Vec<Built>) -> Children {
    let mut children = Vec::with_capacity(built.len());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    let mut props = Vec::new();
    for child in built {
        children.push(child.widget);
        bindings.extend(child.bindings);
        handlers.extend(child.handlers);
        structures.extend(child.structures);
        props.extend(child.props);
    }
    (children, bindings, handlers, structures, props)
}

fn build_each(
    block: &EachBlock,
    component: &ComponentHandle,
    context: &Context,
    theme: Option<&str>,
    position: Position,
    direction: Direction,
    spacing: u32,
) -> Result<(Built, Vec<Value>, Vec<RowInfo>), Error> {
    let items = match evaluate(component, &block.collection)? {
        Value::Array(items) => items,
        other => {
            return Err(error_at(
                block.collection.position,
                format!("each expects Array, got {}", other.type_name()),
            ));
        }
    };
    let last = items.clone();
    let mut all = Vec::new();
    let mut rows = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        let mut variables = vec![Variable {
            name: block.item.clone(),
            value: item.clone(),
        }];
        if let Some(name) = &block.index {
            variables.push(Variable {
                name: name.clone(),
                value: Value::Number(index as f64),
            });
        }
        component.borrow_mut().push_build_scope(&variables);
        let outcome = (|| -> Result<(Option<Value>, Vec<Built>), Error> {
            let key = match &block.key {
                Some(expression) => Some(evaluate(component, expression)?),
                None => None,
            };
            let built = build_items(
                &block.block.items,
                component,
                context,
                theme,
                position,
                direction,
                spacing,
            )?;
            Ok((key, built))
        })();
        component.borrow_mut().pop_build_scope();
        let (key, row) = outcome?;
        let start = all.len();
        all.extend(row);
        rows.push(RowInfo {
            key,
            item,
            start,
            count: all.len() - start,
        });
    }
    let (children, bindings, handlers, structures, props) = flatten(all);
    let mut wrapper = Widget::view(direction, Vec::new()).with_layout(LayoutStyle {
        spacing,
        ..Default::default()
    });
    wrapper.set_children(children);
    Ok((
        Built {
            widget: wrapper,
            bindings,
            handlers,
            structures,
            props,
        },
        last,
        rows,
    ))
}

pub fn rebuild_structure(
    structure: &Structure,
    theme: Option<&str>,
) -> Result<(Built, Value, Vec<RowInfo>), Error> {
    match &structure.kind {
        StructureKind::If { block, .. } => {
            let (built, condition) = build_if(
                block,
                &structure.component,
                &structure.context,
                theme,
                block.condition.position,
                structure.direction,
                structure.spacing,
            )?;
            Ok((built, Value::Bool(condition), Vec::new()))
        }
        StructureKind::Each { block, .. } => {
            let (built, items, rows) = build_each(
                block,
                &structure.component,
                &structure.context,
                theme,
                block.collection.position,
                structure.direction,
                structure.spacing,
            )?;
            Ok((built, Value::Array(items), rows))
        }
    }
}

fn text(
    element: &Element,
    component: &ComponentHandle,
    resolved: Resolved,
) -> Result<(Widget, Expr), Error> {
    match element.block.items.as_slice() {
        [BlockItem::Expr(expression)] => match evaluate(component, expression)? {
            Value::String(content) => Ok((
                Widget::styled_text(content, resolved.text)
                    .with_layout(resolved.layout)
                    .with_styles(resolved.styles),
                expression.clone(),
            )),
            other => Err(error_at(
                expression.position,
                format!("Text content must be a string, got {}", other.type_name()),
            )),
        },
        _ => Err(error_at(
            element.position,
            "Text requires a single content expression",
        )),
    }
}

fn boolean(component: &ComponentHandle, expression: &Expr, name: &str) -> Result<bool, Error> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Ok(value),
        other => Err(error_at(
            expression.position,
            format!("parameter '{name}' expects Bool, got {}", other.type_name()),
        )),
    }
}

fn integer(component: &ComponentHandle, expression: &Expr, name: &str) -> Result<i32, Error> {
    match evaluate(component, expression)? {
        Value::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= f64::from(i32::MIN)
                && value <= f64::from(i32::MAX) =>
        {
            Ok(value as i32)
        }
        _ => Err(error_at(
            expression.position,
            format!("parameter '{name}' expects an integer Number"),
        )),
    }
}

fn evaluate(component: &ComponentHandle, expression: &Expr) -> Result<Value, Error> {
    eval(&mut component.borrow_mut(), expression).map_err(script_error)
}

pub(crate) fn script_error(error: brul_script::Error) -> Error {
    let position = error.position().unwrap_or_default();
    Error {
        message: error.reason(),
        line: position.line,
        column: position.column,
    }
}

fn error_at(position: Position, message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        line: position.line,
        column: position.column,
    }
}
