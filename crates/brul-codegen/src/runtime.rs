use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brul_gui::{Texture, Widget, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_script::{
    Component, Error as ScriptError, Host, Value, Variable, VariableId, execute_program,
};
use brul_syntax::markup::PropType;
use brul_syntax::script::Program;
use brul_syntax::style::Stylesheet;
use tracing::error;

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
    pub scopes: Rc<[Rc<[Variable]>]>,
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
    pub name: VariableId,
    pub ty: PropType,
    pub expression: Expression,
    pub scopes: Rc<[Rc<[Variable]>]>,
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

pub type Expression = fn(&mut Component) -> Result<Value, ScriptError>;

#[derive(Debug, Clone)]
pub enum BindingTarget {
    Text(Expression),
    Image(Expression),
    Checked(Expression),
    Selected(Expression),
    Disabled(Expression),
    Visible(Expression),
    Offset {
        x: Option<Expression>,
        y: Option<Expression>,
    },
}

pub type StructureValue = fn(&mut Component) -> Result<Value, ScriptError>;
pub type Rebuilt = Result<(Built, Vec<RowInfo>), String>;
pub type StructureRebuild = fn(Option<&str>, &ComponentHandle) -> Rebuilt;

#[derive(Clone)]
pub struct Structure {
    pub parent: WidgetId,
    pub each: bool,
    pub component: ComponentHandle,
    pub scopes: Rc<[Rc<[Variable]>]>,
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
    pub name: VariableId,
    pub ty: PropType,
    pub expression: Expression,
}

pub struct Instance<'a> {
    pub theme: Option<&'a str>,
    pub view: View,
    pub script: Option<&'a Program>,
    pub name: &'static str,
    pub names: &'static [&'static str],
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

pub struct PendingEvent {
    pub component: ComponentHandle,
    pub function: &'static str,
    pub arguments: Vec<Value>,
}

thread_local! {
    static PENDING: RefCell<VecDeque<PendingEvent>> = const { RefCell::new(VecDeque::new()) };
}

pub fn take_event() -> Option<PendingEvent> {
    PENDING.with(|pending| pending.borrow_mut().pop_front())
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
        PENDING.with(|pending| {
            pending.borrow_mut().push_back(PendingEvent {
                component: Rc::clone(&event.component),
                function: event.function,
                arguments: arguments.to_vec(),
            });
        });
        Ok(Value::Null)
    }
}

pub fn instance(parent: &ComponentHandle, instance: Instance<'_>) -> Result<Built, String> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_names(instance.names);
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

fn declare_props(
    component: &ComponentHandle,
    parent: &ComponentHandle,
    props: &[PropArgument],
    name: &str,
) -> Result<Vec<VariableId>, String> {
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
    given: &[VariableId],
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
            name: argument.name,
            ty: argument.ty,
            expression: argument.expression,
            scopes: parent.borrow().build_scopes(),
            component: Rc::clone(parent),
        });
    }
}

fn evaluate_argument(
    component: &ComponentHandle,
    argument: &PropArgument,
) -> Result<Value, String> {
    (argument.expression)(&mut component.borrow_mut()).map_err(|error| error.reason())
}

fn assign(
    component: &ComponentHandle,
    argument: &PropArgument,
    value: Value,
    name: &str,
) -> Result<(), String> {
    if !accepts_prop(&value, argument.ty) {
        let property = component.borrow().name(argument.name);
        return Err(format!(
            "property '{property}' of component '{name}' expects {}, got {}",
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
        PropType::Image => matches!(value, Value::Null | Value::Image(_)),
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

pub type Textures = Rc<RefCell<HashMap<PathBuf, Rc<Texture>>>>;

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
    Image(Option<Rc<Texture>>),
    Checked(Option<bool>),
    Selected(Option<bool>),
    Disabled(Option<bool>),
    Visible(Option<bool>),
    Offset(Option<i32>, Option<i32>),
}

pub fn evaluate(binding: &Binding, textures: &Textures) -> Write {
    binding.component.borrow().track();
    let count = binding.scopes.len();
    binding
        .component
        .borrow_mut()
        .push_binding_scopes(&binding.scopes);
    let component = &binding.component;
    let write = match &binding.target {
        BindingTarget::Text(expression) => Write::Text(string(component, *expression)),
        BindingTarget::Image(expression) => Write::Image(texture(component, *expression, textures)),
        BindingTarget::Checked(expression) => Write::Checked(flag(component, *expression)),
        BindingTarget::Selected(expression) => Write::Selected(flag(component, *expression)),
        BindingTarget::Disabled(expression) => Write::Disabled(flag(component, *expression)),
        BindingTarget::Visible(expression) => Write::Visible(flag(component, *expression)),
        BindingTarget::Offset { x, y } => {
            Write::Offset(offset_value(component, *x), offset_value(component, *y))
        }
    };
    binding.component.borrow_mut().pop_binding_scopes(count);
    write
}

pub fn write(write: Write, widget: WidgetId, tree: &mut WidgetTree) {
    match write {
        Write::Text(value) => set_text(tree, widget, value),
        Write::Image(value) => set_texture(tree, widget, value),
        Write::Checked(value) => set_flag(tree, widget, value, WidgetTree::set_checked),
        Write::Selected(value) => set_flag(tree, widget, value, WidgetTree::set_selected),
        Write::Disabled(value) => set_flag(tree, widget, value, WidgetTree::set_disabled),
        Write::Visible(value) => set_flag(tree, widget, value, WidgetTree::set_visible),
        Write::Offset(x, y) => set_offset(tree, widget, x, y),
    }
}

fn value(component: &RefCell<Component>, expression: Expression) -> Option<Value> {
    match expression(&mut component.borrow_mut()) {
        Ok(value) => Some(value),
        Err(error) => {
            error!(%error, "binding expression failed");
            None
        }
    }
}

fn string(component: &RefCell<Component>, expression: Expression) -> Option<String> {
    value(component, expression)?.as_text().map(str::to_owned)
}

fn flag(component: &RefCell<Component>, expression: Expression) -> Option<bool> {
    value(component, expression)?.as_flag()
}

fn texture(
    component: &RefCell<Component>,
    expression: Expression,
    textures: &Textures,
) -> Option<Rc<Texture>> {
    match &value(component, expression)? {
        Value::Image(path) => load_texture(Path::new(path.as_ref()), textures),
        Value::Null => Some(Rc::new(Texture::empty())),
        _ => None,
    }
}

fn load_texture(source: &Path, textures: &Textures) -> Option<Rc<Texture>> {
    let key = match fs::canonicalize(source) {
        Ok(key) => key,
        Err(error) => {
            error!(path = %source.display(), %error, "cannot read image");
            return None;
        }
    };
    if let Some(texture) = textures.borrow().get(&key) {
        return Some(Rc::clone(texture));
    }
    let bytes = match fs::read(&key) {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(path = %source.display(), %error, "cannot read image");
            return None;
        }
    };
    match Texture::from_png(&bytes) {
        Ok(texture) => {
            let texture = Rc::new(texture);
            textures.borrow_mut().insert(key, Rc::clone(&texture));
            Some(texture)
        }
        Err(error) => {
            error!(path = %source.display(), %error, "cannot decode image");
            None
        }
    }
}

pub fn set_texture(tree: &mut WidgetTree, widget: WidgetId, value: Option<Rc<Texture>>) {
    if let Some(value) = value {
        tree.set_image(widget, value);
    }
}

pub fn set_text(tree: &mut WidgetTree, widget: WidgetId, value: Option<String>) {
    if let Some(value) = value {
        tree.set_text(widget, value);
    }
}

pub fn set_flag(
    tree: &mut WidgetTree,
    widget: WidgetId,
    value: Option<bool>,
    set: fn(&mut WidgetTree, WidgetId, bool) -> bool,
) {
    if let Some(value) = value {
        set(tree, widget, value);
    }
}

pub fn set_offset(tree: &mut WidgetTree, widget: WidgetId, x: Option<i32>, y: Option<i32>) {
    if let (Some(x), Some(y)) = (x, y) {
        tree.set_offset(widget, x, y);
    }
}

pub fn text(component: &mut Component, expression: Expression) -> Result<String, String> {
    let value = expression(component).map_err(|error| error.reason())?;
    value
        .as_text()
        .map(str::to_owned)
        .ok_or_else(|| format!("Text content expects String, got {}", value.type_name()))
}

pub fn boolean(
    component: &mut Component,
    expression: Expression,
    name: &str,
) -> Result<bool, String> {
    let value = expression(component).map_err(|error| error.reason())?;
    value
        .as_flag()
        .ok_or_else(|| format!("parameter '{name}' expects Bool, got {}", value.type_name()))
}

pub fn offset(
    component: &mut Component,
    expression: Option<Expression>,
    name: &str,
) -> Result<i32, String> {
    let Some(expression) = expression else {
        return Ok(0);
    };
    let value = expression(component).map_err(|error| error.reason())?;
    value
        .as_pixels()
        .ok_or_else(|| format!("parameter '{name}' expects a Number"))
}

fn offset_value(component: &RefCell<Component>, expression: Option<Expression>) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    value(component, expression)?.as_pixels()
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_syntax::ast::{BinaryOp, Expr, ExprKind, Position};
    use brul_syntax::script::{Function, Statement};
    use std::cell::Cell;

    macro_rules! expr {
        ($name:literal) => {{
            fn read(component: &mut Component) -> Result<Value, ScriptError> {
                component.read($name)
            }
            read
        }};
    }

    fn number(value: f64) -> Expr {
        Expr::new(ExprKind::Number(value), Position::default())
    }

    const NAMES: &[&str] = &["name", "visible", "x", "y", "title", "on", "off", "icon"];

    fn component() -> Component {
        let mut component = Component::new();
        component.set_names(NAMES);
        component
    }

    fn declared(name: &str, value: Value) -> Component {
        let mut component = component();
        component.declare(name, value, false).unwrap();
        component
    }

    #[test]
    fn text_binding_follows_the_component() {
        let mut component = component();
        component
            .declare("name", Value::String("hello".into()), false)
            .unwrap();
        let widget = Widget::text("");
        let id = widget.id();
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        let component = Rc::new(RefCell::new(component));
        let bindings = [Binding {
            widget: id,
            target: BindingTarget::Text(expr!("name")),
            scopes: Vec::new().into(),
            component: Rc::clone(&component),
        }];

        let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
        assert!(!tree.borrow_mut().set_text(id, "hello".to_owned()));

        component
            .borrow_mut()
            .assign("name", Value::String("bye".into()))
            .unwrap();
        brul_reactive::flush();
        assert!(!tree.borrow_mut().set_text(id, "bye".to_owned()));
    }

    fn reentrant(component: &mut Component) -> Result<Value, ScriptError> {
        component.call("go", &[])
    }

    struct TreeProbe {
        tree: Rc<RefCell<WidgetTree>>,
        free: Rc<Cell<bool>>,
    }

    impl Host for TreeProbe {
        fn call(&mut self, _name: &str, _arguments: &[Value]) -> Result<Value, ScriptError> {
            self.free.set(self.tree.try_borrow_mut().is_ok());
            Ok(Value::Bool(self.free.get()))
        }
    }

    fn probe(component: &mut Component) -> Result<Value, ScriptError> {
        component.call("probe", &[])
    }

    #[test]
    fn a_binding_expression_reaches_the_tree_while_it_runs() {
        let widget = Widget::text("");
        let id = widget.id();
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        let free = Rc::new(Cell::new(false));
        let handle = Rc::new(RefCell::new(Component::new()));
        handle.borrow_mut().set_host(TreeProbe {
            tree: Rc::clone(&tree),
            free: Rc::clone(&free),
        });
        let bindings = [Binding {
            widget: id,
            target: BindingTarget::Visible(probe),
            scopes: Vec::new().into(),
            component: Rc::clone(&handle),
        }];

        let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
        assert!(free.get());
        assert!(!tree.borrow_mut().set_visible(id, true));
    }

    #[test]
    fn an_event_raised_from_an_expression_is_delivered_after_the_expression_returns() {
        let handle = Rc::new(RefCell::new(Component::new()));
        handle
            .borrow_mut()
            .define_function(Function {
                name: "read".to_owned(),
                params: Vec::new(),
                body: Vec::new(),
            })
            .unwrap();
        handle.borrow_mut().set_host(EventHost {
            events: vec![ComponentEvent {
                name: "go",
                function: "read",
                params: &[],
                component: Rc::clone(&handle),
            }],
        });

        assert_eq!(reentrant(&mut handle.borrow_mut()), Ok(Value::Null));

        let PendingEvent {
            component,
            function,
            arguments,
        } = take_event().expect("the event is queued");
        assert_eq!(function, "read");
        assert!(
            component
                .borrow_mut()
                .call_function(function, arguments)
                .is_ok()
        );
    }

    #[test]
    fn text_returns_the_string_and_names_the_actual_type() {
        let mut component = declared("name", Value::String("hello".into()));
        assert_eq!(text(&mut component, expr!("name")), Ok("hello".to_owned()));

        let mut component = declared("name", Value::Number(3.0));
        let error = text(&mut component, expr!("name")).unwrap_err();
        assert!(error.contains(Value::Number(3.0).type_name()));
    }

    #[test]
    fn boolean_returns_the_flag_and_names_the_parameter() {
        let name = "visible";
        let mut component = declared(name, Value::Bool(true));
        assert_eq!(boolean(&mut component, expr!("visible"), name), Ok(true));

        let mut component = declared(name, Value::String("yes".into()));
        let error = boolean(&mut component, expr!("visible"), name).unwrap_err();
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
        assert_eq!(offset(&mut component, Some(expr!("x")), name), Ok(-7));

        let mut component = declared(name, Value::Number(2.5));
        assert_eq!(offset(&mut component, Some(expr!("x")), name), Ok(3));

        for value in [f64::NAN, f64::from(i32::MAX) + 1.0] {
            let mut component = declared(name, Value::Number(value));
            assert!(offset(&mut component, Some(expr!("x")), name).is_err());
        }
    }

    #[test]
    fn an_offset_binding_rounds_and_ignores_what_it_has_no_number_for() {
        let none = RefCell::new(Component::new());
        assert_eq!(offset_value(&none, None), Some(0));

        let fractional = RefCell::new(declared("x", Value::Number(1.5)));
        assert_eq!(offset_value(&fractional, Some(expr!("x"))), Some(2));

        let missing = RefCell::new(Component::new());
        assert_eq!(offset_value(&missing, Some(expr!("x"))), None);

        let text = RefCell::new(declared("x", Value::String("4".into())));
        assert_eq!(offset_value(&text, Some(expr!("x"))), None);

        let whole = RefCell::new(declared("x", Value::Number(4.0)));
        assert_eq!(offset_value(&whole, Some(expr!("x"))), Some(4));
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
    fn event_host_queues_the_body_and_leaves_errors_to_the_delivery() {
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
        assert_eq!(host.call("click", &[]), Ok(Value::Null));
        let PendingEvent {
            component,
            function,
            arguments,
        } = take_event().expect("queued");
        assert_eq!(
            component
                .borrow_mut()
                .call_function(function, arguments)
                .unwrap_err()
                .reason(),
            ScriptError::DivisionByZero.reason()
        );
    }

    fn declared_all() -> Rc<RefCell<Component>> {
        let mut component = component();
        for (name, value) in [
            ("title", Value::String("hi".into())),
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
            BindingTarget::Text(expr!("title")),
            BindingTarget::Checked(expr!("on")),
            BindingTarget::Selected(expr!("on")),
            BindingTarget::Disabled(expr!("on")),
            BindingTarget::Visible(expr!("off")),
            BindingTarget::Offset {
                x: Some(expr!("x")),
                y: Some(expr!("y")),
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
                scopes: Vec::new().into(),
                component: Rc::clone(&component),
            })
            .collect();

        let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
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
        let mut component = component();
        component
            .declare("title", Value::Number(1.0), false)
            .unwrap();
        component
            .declare("on", Value::String("yes".into()), false)
            .unwrap();
        let component = Rc::new(RefCell::new(component));
        let (ids, tree) = six_widgets();
        let bindings = [
            Binding {
                widget: ids[0],
                target: BindingTarget::Text(expr!("title")),
                scopes: Vec::new().into(),
                component: Rc::clone(&component),
            },
            Binding {
                widget: ids[1],
                target: BindingTarget::Checked(expr!("on")),
                scopes: Vec::new().into(),
                component: Rc::clone(&component),
            },
        ];

        let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
        assert!(!tree.borrow_mut().set_text(ids[0], "0".to_owned()));
        assert!(!tree.borrow().state(ids[1]).expect("state").checked);
    }

    #[test]
    fn an_image_prop_accepts_a_path_and_null() {
        assert!(accepts_prop(&Value::Image("a.png".into()), PropType::Image));
        assert!(accepts_prop(&Value::Null, PropType::Image));
        assert!(!accepts_prop(
            &Value::String("a.png".into()),
            PropType::Image
        ));
    }

    fn image_binding(path: Value) -> (Binding, Textures) {
        let widget = Widget::image(Texture::empty());
        let component = Rc::new(RefCell::new(declared("icon", path)));
        let binding = Binding {
            widget: widget.id(),
            target: BindingTarget::Image(expr!("icon")),
            scopes: Vec::new().into(),
            component,
        };
        (binding, Textures::default())
    }

    #[test]
    fn an_image_binding_loads_the_file_once() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../brul-gui/tests/fixtures/tint.png"
        );
        let (binding, textures) = image_binding(Value::Image(path.into()));
        let first = evaluate(&binding, &textures);
        let second = evaluate(&binding, &textures);
        let (Write::Image(Some(first)), Write::Image(Some(second))) = (first, second) else {
            panic!("the fixture must load");
        };

        assert!(Rc::ptr_eq(&first, &second));
        assert!(first.width() > 1);
        assert_eq!(textures.borrow().len(), 1);
    }

    #[test]
    fn an_image_binding_ignores_a_missing_file() {
        let (binding, textures) = image_binding(Value::Image("no-such-image.png".into()));

        assert!(matches!(evaluate(&binding, &textures), Write::Image(None)));
        assert!(textures.borrow().is_empty());
    }

    #[test]
    fn an_image_binding_shows_an_empty_texture_for_null() {
        let (binding, textures) = image_binding(Value::Null);
        let Write::Image(Some(texture)) = evaluate(&binding, &textures) else {
            panic!("null must be an empty texture");
        };

        assert_eq!((texture.width(), texture.height()), (1, 1));
    }
}
