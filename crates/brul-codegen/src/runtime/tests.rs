use super::binding::offset_value;
use super::event::EventHost;
use super::*;
use brul_gui::{Image, Texture, Widget, WidgetId, WidgetTree};
use brul_script::{
    Component, Dyn, Error as ScriptError, EventCall, Host, Scope, Value, noop_event,
};
use brul_syntax::TypeName;
use brul_syntax::markup::PropType;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn take_component() -> PendingEvent {
    match take_pending().expect("queued") {
        Pending::Component(event) => event,
        Pending::Ui(_) => panic!("a component event was expected"),
    }
}

macro_rules! typed_expr {
    ($macro:ident, $name:literal, $ty:ty) => {{
        fn read(component: &mut Component) -> Result<$ty, ScriptError> {
            (&component.read($name)?).try_into()
        }
        read
    }};
}

macro_rules! expr {
    ($name:literal) => {{
        fn read(component: &mut Component) -> Result<Dyn, ScriptError> {
            Ok(Dyn::from(component.read($name)?))
        }
        read
    }};
}

macro_rules! text {
    ($name:literal) => {
        typed_expr!(text, $name, String)
    };
}

macro_rules! flag {
    ($name:literal) => {
        typed_expr!(flag, $name, bool)
    };
}

macro_rules! number {
    ($name:literal) => {
        typed_expr!(number, $name, f64)
    };
}

const NAMES: &[&str] = &["name", "visible", "x", "y", "title", "on", "off", "icon"];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn noop(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, ScriptError> {
    Ok(Dyn::null())
}

fn failing_event(_: &mut Component, _: &[Dyn]) -> Result<Dyn, ScriptError> {
    Err(ScriptError::DivisionByZero)
}

fn failing(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, ScriptError> {
    Err(ScriptError::DivisionByZero)
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
        target: BindingTarget::Text(text!("name")),
        scope: Scope::default(),
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

fn probe_flag(component: &mut Component) -> Result<bool, ScriptError> {
    (&probe(component)?).try_into()
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
        target: BindingTarget::Visible(probe_flag),
        scope: Scope::default(),
        component: Rc::clone(&handle),
    }];

    let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
    assert!(free.get());
    assert!(!tree.borrow_mut().set_visible(id, true));
}

#[test]
fn an_event_raised_from_an_expression_is_delivered_after_the_expression_returns() {
    let handle = Rc::new(RefCell::new(Component::new()));
    handle.borrow_mut().set_dispatch(noop);
    handle.borrow_mut().set_host(EventHost {
        events: vec![ComponentEvent {
            name: "go",
            call: noop_event,
            params: &[],
            component: Rc::clone(&handle),
        }],
    });

    assert_eq!(reentrant(&mut handle.borrow_mut()), Ok(Value::Null));

    let PendingEvent {
        component,
        call,
        arguments,
    } = take_component();
    assert!(call(&mut component.borrow_mut(), &arguments).is_ok());
}

#[test]
fn text_returns_the_string_and_names_the_actual_type() {
    let mut component = declared("name", Value::String("hello".into()));
    assert_eq!(
        text(&mut component, text!("name"), "name").unwrap(),
        "hello"
    );

    let mut component = declared("name", Value::Number(3.0));
    let error = text(&mut component, text!("name"), "name").unwrap_err();
    assert!(matches!(
        error,
        ViewError::Expected { found, .. } if found == Value::Number(3.0).type_name()
    ));
}

#[test]
fn boolean_returns_the_flag_and_names_the_parameter() {
    let name = "visible";
    let mut component = declared(name, Value::Bool(true));
    assert!(boolean(&mut component, flag!("visible"), name).unwrap());

    let mut component = declared(name, Value::String("yes".into()));
    let error = boolean(&mut component, flag!("visible"), name).unwrap_err();
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
    assert_eq!(
        offset(&mut component, Some(number!("x")), name).unwrap(),
        -7
    );

    let mut component = declared(name, Value::Number(2.5));
    assert_eq!(offset(&mut component, Some(number!("x")), name).unwrap(), 3);

    for value in [f64::NAN, f64::from(i32::MAX) + 1.0] {
        let mut component = declared(name, Value::Number(value));
        assert!(offset(&mut component, Some(number!("x")), name).is_err());
    }
}

#[test]
fn an_offset_binding_rounds_and_ignores_what_it_has_no_number_for() {
    let none = RefCell::new(Component::new());
    assert_eq!(offset_value(&none, None), Some(0));

    let fractional = RefCell::new(declared("x", Value::Number(1.5)));
    assert_eq!(offset_value(&fractional, Some(number!("x"))), Some(2));

    let missing = RefCell::new(Component::new());
    assert_eq!(offset_value(&missing, Some(number!("x"))), None);

    let text = RefCell::new(declared("x", Value::String("4".into())));
    assert_eq!(offset_value(&text, Some(number!("x"))), None);

    let whole = RefCell::new(declared("x", Value::Number(4.0)));
    assert_eq!(offset_value(&whole, Some(number!("x"))), Some(4));
}

#[test]
fn event_names_round_trip_and_stay_distinct() {
    let events = EventKind::ALL;
    for event in events {
        assert_eq!(event.name().parse::<EventKind>().unwrap(), *event);
    }
    let mut names: Vec<_> = events.iter().map(|event| event.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), events.len());
    assert!("hover".parse::<EventKind>().is_err());
}

fn event_host(events: Vec<ComponentEvent>) -> EventHost {
    EventHost { events }
}

fn event(
    call: EventCall,
    params: &'static [PropType],
    component: &ComponentHandle,
) -> ComponentEvent {
    ComponentEvent {
        name: "click",
        call,
        params,
        component: Rc::clone(component),
    }
}

#[test]
fn event_host_rejects_an_unknown_event() {
    let component = Rc::new(RefCell::new(Component::new()));
    let mut host = event_host(vec![event(noop_event, &[], &component)]);
    let error = host.call("hover", &[]).unwrap_err();
    assert!(matches!(error, ScriptError::UnknownFunction(ref name) if name == "hover"));
}

#[test]
fn event_host_checks_the_arity_before_the_call() {
    let component = Rc::new(RefCell::new(Component::new()));
    let mut host = event_host(vec![event(noop_event, &[PropType::Bool], &component)]);
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
    let mut host = event_host(vec![event(noop_event, &[PropType::Bool], &component)]);
    let error = host.call("click", &[Value::Number(1.0)]).unwrap_err();
    assert!(matches!(error, ScriptError::EventArgument { .. }));
}

#[test]
fn event_host_runs_the_component_function() {
    let component = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_dispatch(noop);
    let mut host = event_host(vec![event(noop_event, &[], &component)]);
    assert_eq!(host.call("click", &[]), Ok(Value::Null));
}

#[test]
fn event_host_queues_the_body_and_leaves_errors_to_the_delivery() {
    let component = Rc::new(RefCell::new(Component::new()));
    component.borrow_mut().set_dispatch(failing);
    let mut host = event_host(vec![event(failing_event, &[], &component)]);
    assert_eq!(host.call("click", &[]), Ok(Value::Null));
    let PendingEvent {
        component,
        call,
        arguments,
    } = take_component();
    assert_eq!(
        call(&mut component.borrow_mut(), &arguments)
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
        scope: Scope::default(),
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
        scope: Scope::default(),
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
        BindingTarget::Text(text!("title")),
        BindingTarget::Checked(flag!("on")),
        BindingTarget::Selected(flag!("on")),
        BindingTarget::Disabled(flag!("on")),
        BindingTarget::Visible(flag!("off")),
        BindingTarget::Offset {
            x: Some(number!("x")),
            y: Some(number!("y")),
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
            scope: Scope::default(),
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
            target: BindingTarget::Text(text!("title")),
            scope: Scope::default(),
            component: Rc::clone(&component),
        },
        Binding {
            widget: ids[1],
            target: BindingTarget::Checked(flag!("on")),
            scope: Scope::default(),
            component: Rc::clone(&component),
        },
    ];

    let _bindings = Bindings::new(&bindings, &tree, &Textures::default());
    assert!(!tree.borrow_mut().set_text(ids[0], "0".to_owned()));
    assert!(!tree.borrow().state(ids[1]).expect("state").checked);
}

#[test]
fn an_image_prop_accepts_a_path_and_null() {
    assert!(accepts_prop(
        &Value::Image("a.png".into()),
        &PropType::Image
    ));
    assert!(accepts_prop(&Value::Null, &PropType::Image));
    assert!(!accepts_prop(
        &Value::String("a.png".into()),
        &PropType::Image
    ));
    assert!(accepts_prop(
        &Value::image_bytes(&[1, 2, 3]),
        &PropType::Image
    ));
}

fn image_binding(path: Value) -> (Binding, Textures) {
    let widget = Widget::image(Texture::empty());
    let component = Rc::new(RefCell::new(declared("icon", path)));
    let binding = Binding {
        widget: widget.id(),
        target: BindingTarget::Image(expr!("icon")),
        scope: Scope::default(),
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
fn an_image_binding_decodes_an_embedded_resource() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../brul-gui/tests/fixtures/tint.png"
    ))
    .unwrap();
    let (binding, textures) = image_binding(Value::image_bytes(&bytes));

    let Write::Image(Some(image)) = evaluate(&binding, &textures) else {
        panic!("the fixture must decode");
    };
    assert!(image.width() > 1);
    assert!(textures.borrow().loaded.is_empty());
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

#[test]
fn attach_primitives_edit_the_field_and_query_focus_and_flags() {
    while take_pending().is_some() {}

    let input = Widget::input("abc");
    let id = input.id();
    let root = Widget::row(vec![input, Widget::text("label")]);
    let root_id = root.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    let mut build = Build::new();
    build.set_tree(Rc::clone(&tree));
    build.set_target(id, StateId::new(9), Rc::new(RefCell::new(Component::new())));

    let element = build.element();
    assert_eq!(element.edit_buffer().expect("input").text(), "abc");
    let edited = element.with_edit(|buffer| {
        buffer.insert_text(Position::new(0, 0), "x");
        buffer.text().to_owned()
    });
    assert_eq!(edited.as_deref(), Some("xabc"));
    assert_eq!(tree.borrow().edit_buffer(id).expect("input").text(), "xabc");

    let rejected = element.with_edit(|buffer| {
        buffer.insert_fragment(
            Position::new(0, 1),
            Fragment::Image(Image::from(Texture::empty())),
        );
    });
    assert!(rejected.is_none());
    assert_eq!(tree.borrow().edit_buffer(id).expect("input").text(), "xabc");
    assert_eq!(element.caret(), Some(Position::new(0, 4)));
    assert_eq!(
        element.selection(),
        Position::new(0, 4)..Position::new(0, 4)
    );

    assert_eq!(build.focused(), None);
    assert!(build.focus(Some(id)));
    assert_eq!(build.focused(), Some(id));
    assert!(element.focused());
    assert!(build.focusables().contains(&id));
    assert_eq!(build.path_to(id), vec![root_id, id]);
    assert_eq!(build.ancestors(id), vec![root_id]);
    assert_eq!(build.modal_scope(), None);
    assert!(build.within_scope(root_id, id));

    assert!(element.set_hovered(true));
    assert!(element.hovered());
    assert!(element.set_pressed(true));
    assert!(element.pressed());
    assert!(element.scroll_by(5));

    build.emit_event(UiEvent::widget(id, Event::click()));
    let Pending::Ui(event) = take_pending().expect("ui event") else {
        panic!("a semantic event was expected");
    };
    assert_eq!(event.target, EventTarget::Widget(id));
    assert!(take_pending().is_none());
}

#[test]
fn the_clipboard_port_is_typed_and_optional() {
    struct MemoryClipboard(RefCell<Option<ClipboardData>>);

    impl Clipboard for MemoryClipboard {
        fn read(&self) -> Option<ClipboardData> {
            self.0.borrow().clone()
        }

        fn write(&self, data: &ClipboardData) {
            *self.0.borrow_mut() = Some(data.clone());
        }
    }

    let mut build = Build::new();
    build.set_clipboard(&ClipboardData::from(String::from("copied")));
    assert!(build.clipboard().is_none());

    build.set_clipboard_bridge(Some(Rc::new(MemoryClipboard(RefCell::new(None)))));
    build.set_clipboard(&ClipboardData::from(String::from("copied")));
    assert!(matches!(build.clipboard(), Some(ClipboardData::Text(text)) if text == "copied"));
}
