use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{Widget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Error as ScriptError, Host, Value, Variable, eval, execute_program};
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
    pub events: &'a [ComponentEvent],
}

pub struct ComponentEvent {
    pub name: &'static str,
    pub function: &'static str,
    pub params: &'static [PropType],
    pub component: ComponentHandle,
}

struct EventHost {
    events: Vec<ComponentEvent>,
}

impl Host for EventHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, ScriptError> {
        let Some(event) = self.events.iter().find(|event| event.name == name) else {
            return Err(ScriptError::UnknownFunction(name.to_owned()));
        };
        if event.params.len() != arguments.len() {
            return Err(ScriptError::FunctionArity {
                name: event.name.to_owned(),
                expected: event.params.len(),
                found: arguments.len(),
            });
        }
        for (ty, value) in event.params.iter().zip(arguments) {
            if !accepts_prop(value, *ty) {
                return Err(ScriptError::EventArgument {
                    name: event.name.to_owned(),
                    expected: prop_name(*ty),
                    found: value.type_name(),
                });
            }
        }
        let component = Rc::clone(&event.component);
        component
            .borrow_mut()
            .call_function(event.function, arguments.to_vec())
    }
}

pub fn instance(parent: &ComponentHandle, instance: Instance<'_>) -> Result<Built, String> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    start(&component, &instance)?;
    let given = declare_props(&component, parent, instance.props, instance.name)?;
    declare_defaults(&component, instance.defaults, &given, instance.name)?;
    let mut built = (instance.view)(instance.theme, &component)?;
    bind(&mut built, parent, &component, instance.props);
    Ok(built)
}

fn start(component: &ComponentHandle, instance: &Instance<'_>) -> Result<(), String> {
    if !instance.events.is_empty() {
        component.borrow_mut().set_host(EventHost {
            events: instance
                .events
                .iter()
                .map(|event| ComponentEvent {
                    name: event.name,
                    function: event.function,
                    params: event.params,
                    component: Rc::clone(&event.component),
                })
                .collect(),
        });
    }
    match instance.script {
        Some(program) => {
            execute_program(&mut component.borrow_mut(), program).map_err(|error| error.reason())
        }
        None => Ok(()),
    }
}

fn declare_props<'a>(
    component: &ComponentHandle,
    parent: &ComponentHandle,
    props: &'a [PropArgument],
    name: &str,
) -> Result<Vec<&'a str>, String> {
    let mut given = Vec::new();
    for argument in props {
        let value = evaluate_argument(parent, argument)?;
        assign(component, argument, value, name)?;
        given.push(argument.name);
    }
    Ok(given)
}

fn declare_defaults(
    component: &ComponentHandle,
    defaults: &[PropArgument],
    given: &[&str],
    name: &str,
) -> Result<(), String> {
    for argument in defaults {
        if given.contains(&argument.name) {
            continue;
        }
        let value = evaluate_argument(component, argument)?;
        assign(component, argument, value, name)?;
    }
    Ok(())
}

fn bind(
    built: &mut Built,
    parent: &ComponentHandle,
    component: &ComponentHandle,
    props: &[PropArgument],
) {
    let widget = built.widget.id();
    for argument in props {
        built.props.push(PropBinding {
            widget,
            target: Rc::clone(component),
            name: argument.name.to_owned(),
            ty: argument.ty,
            expression: argument.expression.clone(),
            scopes: parent.borrow().build_scopes(),
            component: Rc::clone(parent),
        });
    }
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

pub fn apply(binding: &Binding, tree: &RefCell<WidgetTree>) {
    let component = Rc::clone(&binding.component);
    component.borrow().track();
    let count = binding.scopes.len();
    component.borrow_mut().push_binding_scopes(&binding.scopes);
    write_target(binding, &component, tree);
    component.borrow_mut().pop_binding_scopes(count);
}

fn write_target(binding: &Binding, component: &RefCell<Component>, tree: &RefCell<WidgetTree>) {
    let widget = binding.widget;
    match &binding.target {
        BindingTarget::Text(expression) => set_text(tree, widget, string(component, expression)),
        BindingTarget::Checked(expression) => set_flag(
            tree,
            widget,
            flag(component, expression),
            WidgetTree::set_checked,
        ),
        BindingTarget::Selected(expression) => set_flag(
            tree,
            widget,
            flag(component, expression),
            WidgetTree::set_selected,
        ),
        BindingTarget::Disabled(expression) => set_flag(
            tree,
            widget,
            flag(component, expression),
            WidgetTree::set_disabled,
        ),
        BindingTarget::Visible(expression) => set_flag(
            tree,
            widget,
            flag(component, expression),
            WidgetTree::set_visible,
        ),
        BindingTarget::Offset { x, y } => set_offset(
            tree,
            widget,
            offset_value(component, x.as_ref()),
            offset_value(component, y.as_ref()),
        ),
    }
}

fn string(component: &RefCell<Component>, expression: &Expr) -> Option<String> {
    eval(&mut component.borrow_mut(), expression)
        .ok()?
        .as_text()
        .map(str::to_owned)
}

fn flag(component: &RefCell<Component>, expression: &Expr) -> Option<bool> {
    eval(&mut component.borrow_mut(), expression)
        .ok()?
        .as_flag()
}

pub fn set_text(tree: &RefCell<WidgetTree>, widget: WidgetId, value: Option<String>) {
    if let Some(value) = value {
        tree.borrow_mut().set_text(widget, value);
    }
}

pub fn set_flag(
    tree: &RefCell<WidgetTree>,
    widget: WidgetId,
    value: Option<bool>,
    set: fn(&mut WidgetTree, WidgetId, bool) -> bool,
) {
    if let Some(value) = value {
        set(&mut tree.borrow_mut(), widget, value);
    }
}

pub fn set_offset(tree: &RefCell<WidgetTree>, widget: WidgetId, x: Option<i32>, y: Option<i32>) {
    if let (Some(x), Some(y)) = (x, y) {
        tree.borrow_mut().set_offset(widget, x, y);
    }
}

pub fn text(component: &mut Component, expression: &Expr) -> Result<String, String> {
    let value = evaluate(component, expression)?;
    value
        .as_text()
        .map(str::to_owned)
        .ok_or_else(|| format!("Text content expects String, got {}", value.type_name()))
}

pub fn boolean(component: &mut Component, expression: &Expr, name: &str) -> Result<bool, String> {
    let value = evaluate(component, expression)?;
    value
        .as_flag()
        .ok_or_else(|| format!("parameter '{name}' expects Bool, got {}", value.type_name()))
}

pub fn offset(
    component: &mut Component,
    expression: Option<&Expr>,
    name: &str,
) -> Result<i32, String> {
    let Some(expression) = expression else {
        return Ok(0);
    };
    let value = evaluate(component, expression)?;
    value
        .as_pixels()
        .ok_or_else(|| format!("parameter '{name}' expects a Number"))
}

pub fn evaluate(component: &mut Component, expression: &Expr) -> Result<Value, String> {
    eval(component, expression).map_err(|error| error.reason())
}

fn offset_value(component: &RefCell<Component>, expression: Option<&Expr>) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    eval(&mut component.borrow_mut(), expression)
        .ok()?
        .as_pixels()
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_syntax::ast::{BinaryOp, ExprKind, Position};
    use brul_syntax::script::{Function, Statement};

    fn ident(name: &str) -> Expr {
        Expr::new(ExprKind::Ident(name.to_owned()), Position::default())
    }

    fn number(value: f64) -> Expr {
        Expr::new(ExprKind::Number(value), Position::default())
    }

    fn declared(name: &str, value: Value) -> Component {
        let mut component = Component::new();
        component.declare(name, value, false).unwrap();
        component
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

    #[test]
    fn text_returns_the_string_and_names_the_actual_type() {
        let mut component = declared("name", Value::String("hello".to_owned()));
        assert_eq!(text(&mut component, &ident("name")), Ok("hello".to_owned()));

        let mut component = declared("name", Value::Number(3.0));
        let error = text(&mut component, &ident("name")).unwrap_err();
        assert!(error.contains(Value::Number(3.0).type_name()));
    }

    #[test]
    fn boolean_returns_the_flag_and_names_the_parameter() {
        let name = "visible";
        let mut component = declared(name, Value::Bool(true));
        assert_eq!(boolean(&mut component, &ident(name), name), Ok(true));

        let mut component = declared(name, Value::String("yes".to_owned()));
        let error = boolean(&mut component, &ident(name), name).unwrap_err();
        assert!(error.contains(name));
    }

    #[test]
    fn an_absent_offset_is_zero() {
        let mut component = Component::new();
        assert_eq!(offset(&mut component, None, "x"), Ok(0));
    }

    #[test]
    fn offset_rounds_fractions_and_rejects_the_rest() {
        let name = "x";
        let mut component = declared(name, Value::Number(-7.0));
        assert_eq!(offset(&mut component, Some(&ident(name)), name), Ok(-7));

        let mut component = declared(name, Value::Number(2.5));
        assert_eq!(offset(&mut component, Some(&ident(name)), name), Ok(3));

        for value in [f64::NAN, f64::from(i32::MAX) + 1.0] {
            let mut component = declared(name, Value::Number(value));
            assert!(offset(&mut component, Some(&ident(name)), name).is_err());
        }
    }

    #[test]
    fn an_offset_binding_rounds_and_ignores_what_it_has_no_number_for() {
        let none = RefCell::new(Component::new());
        assert_eq!(offset_value(&none, None), Some(0));

        let fractional = RefCell::new(declared("x", Value::Number(1.5)));
        assert_eq!(offset_value(&fractional, Some(&ident("x"))), Some(2));

        let missing = RefCell::new(Component::new());
        assert_eq!(offset_value(&missing, Some(&ident("x"))), None);

        let text = RefCell::new(declared("x", Value::String("4".to_owned())));
        assert_eq!(offset_value(&text, Some(&ident("x"))), None);

        let whole = RefCell::new(declared("x", Value::Number(4.0)));
        assert_eq!(offset_value(&whole, Some(&ident("x"))), Some(4));
    }

    #[test]
    fn every_prop_type_has_its_own_name() {
        let names: Vec<&str> = [
            PropType::Bool,
            PropType::Number,
            PropType::String,
            PropType::Image,
        ]
        .into_iter()
        .map(prop_name)
        .collect();
        assert!(names.iter().all(|name| !name.is_empty()));
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn event_names_round_trip_and_stay_distinct() {
        let events = [
            HandlerEvent::Click,
            HandlerEvent::Press,
            HandlerEvent::Release,
            HandlerEvent::Enter,
            HandlerEvent::Leave,
            HandlerEvent::Drag,
            HandlerEvent::Key,
        ];
        for event in events {
            assert_eq!(HandlerEvent::parse(event.name()), Some(event));
        }
        let mut names: Vec<_> = events.iter().map(|event| event.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), events.len());
        assert_eq!(HandlerEvent::parse("hover"), None);
    }

    fn event_host(events: Vec<ComponentEvent>) -> EventHost {
        EventHost { events }
    }

    fn event(
        function: &'static str,
        params: &'static [PropType],
        component: &ComponentHandle,
    ) -> ComponentEvent {
        ComponentEvent {
            name: "click",
            function,
            params,
            component: Rc::clone(component),
        }
    }

    #[test]
    fn event_host_rejects_an_unknown_event() {
        let component = Rc::new(RefCell::new(Component::new()));
        let mut host = event_host(vec![event("on_click", &[], &component)]);
        let error = host.call("hover", &[]).unwrap_err();
        assert!(matches!(error, ScriptError::UnknownFunction(ref name) if name == "hover"));
    }

    #[test]
    fn event_host_checks_the_arity_before_the_call() {
        let component = Rc::new(RefCell::new(Component::new()));
        let mut host = event_host(vec![event("on_click", &[PropType::Bool], &component)]);
        let error = host.call("click", &[]).unwrap_err();
        assert!(matches!(
            error,
            ScriptError::FunctionArity {
                expected: 1,
                found: 0,
                ..
            }
        ));
    }

    #[test]
    fn event_host_checks_the_argument_types() {
        let component = Rc::new(RefCell::new(Component::new()));
        let mut host = event_host(vec![event("on_click", &[PropType::Bool], &component)]);
        let error = host.call("click", &[Value::Number(1.0)]).unwrap_err();
        assert!(matches!(error, ScriptError::EventArgument { .. }));
    }

    #[test]
    fn event_host_runs_the_component_function() {
        let component = Rc::new(RefCell::new(Component::new()));
        component
            .borrow_mut()
            .define_function(Function {
                name: "on_click".to_owned(),
                params: Vec::new(),
                body: Vec::new(),
            })
            .unwrap();
        let mut host = event_host(vec![event("on_click", &[], &component)]);
        assert_eq!(host.call("click", &[]), Ok(Value::Null));
    }

    #[test]
    fn event_host_surfaces_an_error_from_the_body() {
        let component = Rc::new(RefCell::new(Component::new()));
        let divide_by_zero = Expr::new(
            ExprKind::Binary {
                op: BinaryOp::Div,
                left: Box::new(number(1.0)),
                right: Box::new(number(0.0)),
            },
            Position::default(),
        );
        component
            .borrow_mut()
            .define_function(Function {
                name: "on_click".to_owned(),
                params: Vec::new(),
                body: vec![Statement::Expr(divide_by_zero)],
            })
            .unwrap();
        let mut host = event_host(vec![event("on_click", &[], &component)]);
        assert_eq!(
            host.call("click", &[]).unwrap_err().reason(),
            ScriptError::DivisionByZero.reason()
        );
    }

    fn declared_all() -> Rc<RefCell<Component>> {
        let mut component = Component::new();
        for (name, value) in [
            ("title", Value::String("hi".to_owned())),
            ("on", Value::Bool(true)),
            ("off", Value::Bool(false)),
            ("x", Value::Number(4.0)),
            ("y", Value::Number(6.0)),
        ] {
            component.declare(name, value, false).unwrap();
        }
        Rc::new(RefCell::new(component))
    }

    fn targets() -> Vec<BindingTarget> {
        vec![
            BindingTarget::Text(ident("title")),
            BindingTarget::Checked(ident("on")),
            BindingTarget::Selected(ident("on")),
            BindingTarget::Disabled(ident("on")),
            BindingTarget::Visible(ident("off")),
            BindingTarget::Offset {
                x: Some(ident("x")),
                y: Some(ident("y")),
            },
        ]
    }

    fn six_widgets() -> (Vec<WidgetId>, Rc<RefCell<WidgetTree>>) {
        let widgets: Vec<Widget> = (0..6)
            .map(|index| Widget::text(index.to_string()))
            .collect();
        let ids: Vec<WidgetId> = widgets.iter().map(Widget::id).collect();
        let tree = Rc::new(RefCell::new(WidgetTree::new(Widget::view(
            brul_gui::Direction::Column,
            widgets,
        ))));
        (ids, tree)
    }

    #[test]
    fn every_binding_target_writes_its_own_state() {
        let component = declared_all();
        let (ids, tree) = six_widgets();
        let bindings: Vec<Binding> = targets()
            .into_iter()
            .enumerate()
            .map(|(index, target)| Binding {
                widget: ids[index],
                target,
                scopes: Vec::new(),
                component: Rc::clone(&component),
            })
            .collect();

        let _bindings = Bindings::new(&bindings, &tree);
        assert!(!tree.borrow_mut().set_text(ids[0], "hi".to_owned()));
        assert!(tree.borrow().state(ids[1]).expect("state").checked);
        assert!(tree.borrow().state(ids[2]).expect("state").selected);
        assert!(tree.borrow().state(ids[3]).expect("state").disabled);
        assert!(!tree.borrow().state(ids[4]).expect("state").visible);
        let mut engine = brul_gui::TextEngine::new();
        tree.borrow_mut()
            .layout(
                &mut engine,
                brul_gui::Size {
                    width: 200,
                    height: 200,
                },
                1.0,
            )
            .expect("layout");
        let mut tree = tree.borrow_mut();
        let node = tree.snapshot().expect("snapshot").node(ids[5]).cloned();
        let node = node.expect("node");
        assert_eq!((node.x, node.y), (4, 6));
    }

    #[test]
    fn a_binding_ignores_a_value_of_the_wrong_type() {
        let mut component = Component::new();
        component
            .declare("title", Value::Number(1.0), false)
            .unwrap();
        component
            .declare("on", Value::String("yes".to_owned()), false)
            .unwrap();
        let component = Rc::new(RefCell::new(component));
        let (ids, tree) = six_widgets();
        let bindings = [
            Binding {
                widget: ids[0],
                target: BindingTarget::Text(ident("title")),
                scopes: Vec::new(),
                component: Rc::clone(&component),
            },
            Binding {
                widget: ids[1],
                target: BindingTarget::Checked(ident("on")),
                scopes: Vec::new(),
                component: Rc::clone(&component),
            },
        ];

        let _bindings = Bindings::new(&bindings, &tree);
        assert!(!tree.borrow_mut().set_text(ids[0], "0".to_owned()));
        assert!(!tree.borrow().state(ids[1]).expect("state").checked);
    }
}
