use super::*;

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

pub(super) fn six_widgets() -> (Vec<WidgetId>, Rc<RefCell<WidgetTree>>) {
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

pub(super) fn image_binding(path: Value) -> (Binding, Textures) {
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
