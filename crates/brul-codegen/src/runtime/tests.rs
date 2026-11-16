use super::binding::offset_value;
use super::event::EventHost;
use super::*;
use brul_gui::{Texture, Widget, WidgetId, WidgetTree};
use brul_script::{Component, Error as ScriptError, Host, Value};
use brul_syntax::ast::{BinaryOp, Expr, ExprKind, Position};
use brul_syntax::markup::PropType;
use brul_syntax::script::{Function, Statement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
    assert_eq!(text(&mut component, expr!("name")).unwrap(), "hello");

    let mut component = declared("name", Value::Number(3.0));
    let error = text(&mut component, expr!("name")).unwrap_err();
    assert!(matches!(
        error,
        ViewError::Expected { found, .. } if found == Value::Number(3.0).type_name()
    ));
}

#[test]
fn boolean_returns_the_flag_and_names_the_parameter() {
    let name = "visible";
    let mut component = declared(name, Value::Bool(true));
    assert!(boolean(&mut component, expr!("visible"), name).unwrap());

    let mut component = declared(name, Value::String("yes".into()));
    let error = boolean(&mut component, expr!("visible"), name).unwrap_err();
    assert!(
        matches!(error, ViewError::Expected { target, .. } if target == format!("parameter '{name}'"))
    );
}

#[test]
fn an_absent_offset_is_zero() {
    let mut component = Component::new();
    assert_eq!(offset(&mut component, None, "x").unwrap(), 0);
}

#[test]
fn offset_rounds_fractions_and_rejects_the_rest() {
    let name = "x";
    let mut component = declared(name, Value::Number(-7.0));
    assert_eq!(offset(&mut component, Some(expr!("x")), name).unwrap(), -7);

    let mut component = declared(name, Value::Number(2.5));
    assert_eq!(offset(&mut component, Some(expr!("x")), name).unwrap(), 3);

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
        HandlerEvent::Close,
        HandlerEvent::Focus,
        HandlerEvent::Blur,
        HandlerEvent::Input,
        HandlerEvent::Submit,
        HandlerEvent::Dismiss,
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

#[test]
fn a_class_binding_applies_the_named_class() {
    let mut declared = component();
    declared
        .declare("title", Value::String("wide".into()), false)
        .unwrap();
    let component = Rc::new(RefCell::new(declared));
    let (ids, tree) = six_widgets();
    let binding = Binding {
        widget: ids[0],
        target: BindingTarget::Class {
            table: class_table,
            expression: expr!("title"),
        },
        scopes: Vec::new().into(),
        component: Rc::clone(&component),
    };

    let _bindings = Bindings::new(&[binding], &tree, &Textures::default());
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
    let node = tree.snapshot().expect("snapshot").node(ids[0]).cloned();
    assert_eq!(node.expect("node").width, 40);
}

#[test]
fn a_class_binding_ignores_an_unknown_name() {
    let mut declared = component();
    declared
        .declare("title", Value::String("missing".into()), false)
        .unwrap();
    let component = Rc::new(RefCell::new(declared));
    let (ids, _tree) = six_widgets();
    let binding = Binding {
        widget: ids[0],
        target: BindingTarget::Class {
            table: class_table,
            expression: expr!("title"),
        },
        scopes: Vec::new().into(),
        component,
    };

    assert!(matches!(
        evaluate(&binding, &Textures::default()),
        Write::Class(None)
    ));
}

fn class_table(name: &str) -> Option<brul_gui::Class> {
    (name == "wide").then(|| brul_gui::Class {
        layout: brul_gui::LayoutPatch {
            width: Some(brul_gui::Dimension::Px(40)),
            ..brul_gui::LayoutPatch::default()
        },
        ..brul_gui::Class::default()
    })
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

    assert!(first.ptr_eq(&second));
    assert!(first.width() > 1);
    assert_eq!(textures.borrow().loaded.len(), 1);
}

#[test]
fn an_image_binding_ignores_a_missing_file() {
    let (binding, textures) = image_binding(Value::Image("no-such-image.png".into()));

    assert!(matches!(evaluate(&binding, &textures), Write::Image(None)));
    assert!(textures.borrow().loaded.is_empty());
}

#[test]
fn an_image_binding_remembers_a_missing_file() {
    let (binding, textures) = image_binding(Value::Image("no-such-image.png".into()));

    evaluate(&binding, &textures);
    assert!(matches!(evaluate(&binding, &textures), Write::Image(None)));
    assert_eq!(textures.borrow().failed.len(), 1);
}

#[test]
fn an_image_binding_shows_an_empty_texture_for_null() {
    let (binding, textures) = image_binding(Value::Null);
    let Write::Image(Some(texture)) = evaluate(&binding, &textures) else {
        panic!("null must be an empty texture");
    };

    assert_eq!((texture.width(), texture.height()), (1, 1));
}
