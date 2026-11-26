use super::*;
#[test]
pub(super) fn the_effect_of_a_removed_branch_dies_with_its_widget() {
    let (component, built, structures) = generated_built(
        bound_view,
        NAMES,
        None,
        &[],
        Some(script_with(shown_and_label)),
    )
    .expect("launch succeeds");
    let mut ui = Ui::assemble(component, built, bound_view, None, structures, None);
    brul_reactive::flush();
    let assembled = brul_reactive::live_nodes();

    ui.component
        .borrow_mut()
        .assign("shown", Value::Bool(true))
        .unwrap();
    brul_reactive::flush();
    ui.settle();
    assert_eq!(brul_reactive::live_nodes(), assembled + 1);

    ui.component
        .borrow_mut()
        .assign("shown", Value::Bool(false))
        .unwrap();
    brul_reactive::flush();
    ui.settle();
    assert_eq!(brul_reactive::live_nodes(), assembled);
}

pub(super) fn declared(name: &'static str, value: Value) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    component.declare(name, value, false).expect("declared");
    Rc::new(RefCell::new(component))
}

pub(super) fn prop(
    component: &ComponentHandle,
    target: &ComponentHandle,
    name: &'static str,
    ty: PropType,
    expression: brul_codegen::runtime::PropExpression,
) -> PropBinding {
    PropBinding {
        widget: Widget::text("").id(),
        target: Rc::clone(target),
        name,
        ty,
        expression,
        scope: Scope::default(),
        component: Rc::clone(component),
    }
}

pub(super) fn child_with_property(name: &str, value: Value) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    let id = component.id(name).expect("prop name is in the table");
    component
        .declare_property(id, value)
        .expect("declared property");
    Rc::new(RefCell::new(component))
}

#[test]
pub(super) fn a_prop_binding_writes_the_property_of_the_child() {
    let source = declared("colour", Value::String("red".into()));
    let target = child_with_property("accent", Value::String("blue".into()));
    apply_prop(&prop(
        &source,
        &target,
        "accent",
        PropType::String,
        brul_codegen::runtime::PropExpression::Text(text!("colour")),
    ));
    assert_eq!(
        target.borrow().read("accent").expect("read back"),
        Value::String("red".into())
    );
}

#[test]
pub(super) fn a_prop_binding_skips_a_value_of_the_wrong_type() {
    let source = declared("colour", Value::String("red".into()));
    let target = child_with_property("accent", Value::String("blue".into()));
    apply_prop(&prop(
        &source,
        &target,
        "accent",
        PropType::Bool,
        brul_codegen::runtime::PropExpression::Text(text!("colour")),
    ));
    assert_eq!(
        target.borrow().read("accent").expect("read back"),
        Value::String("blue".into())
    );
}

#[test]
pub(super) fn a_prop_binding_survives_an_unknown_property() {
    let source = declared("colour", Value::String("red".into()));
    let target = child_with_property("accent", Value::String("blue".into()));
    apply_prop(&prop(
        &source,
        &target,
        "missing",
        PropType::String,
        brul_codegen::runtime::PropExpression::Text(text!("colour")),
    ));
    assert!(target.borrow().read("missing").is_err());
}
pub(super) fn themed_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    _component: &ComponentHandle,
) -> Result<Built, ViewError> {
    Ok(Built {
        widget: Widget::text(theme.map(Theme::name).unwrap_or("none")),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

pub(super) fn rejecting_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    themed_view(None, &mut Build::new(), component)
}

#[test]
pub(super) fn a_theme_change_rebuilds_the_view() {
    let (component, built, structures) =
        generated_built(themed_view, NAMES, None, &[], None).expect("launch");
    let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
    ui.set_theme(TestTheme::Dark).expect("theme changes");
    assert_eq!(ui.theme, Some(TestTheme::Dark));
}

#[test]
pub(super) fn a_failed_theme_change_keeps_the_previous_theme() {
    let (component, built, structures) =
        generated_built(rejecting_view, NAMES, None, &[], None).expect("launch");
    let mut ui = Ui::assemble(
        component,
        built,
        rejecting_view,
        Some(TestTheme::Light),
        structures,
        None,
    );
    ui.set_theme(TestTheme::Light)
        .expect("same theme is a no-op");
    assert!(ui.set_theme(TestTheme::Dark).is_err());
    assert_eq!(ui.theme, Some(TestTheme::Light));
}

pub(super) fn binding(
    component: &ComponentHandle,
    widget: WidgetId,
    target: BindingTarget,
) -> Binding {
    Binding::new(component, widget, target)
}

pub(super) fn icon(component: &mut Component) -> Result<Dyn, Error> {
    Ok(Dyn::from(component.read("icon")?))
}

pub(super) fn image_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let widget = Widget::image(Texture::empty());
    let id = widget.id();
    Ok(Built {
        widget,
        bindings: vec![binding(component, id, BindingTarget::Image(icon))],
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

#[test]
pub(super) fn an_image_binding_reaches_the_tree_through_ui() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/brul-gui/tests/fixtures/tint.png"
    );
    let (component, built, structures) = generated_built(
        image_view,
        NAMES,
        None,
        &[("icon", Value::Image(path.into()))],
        None,
    )
    .expect("launch");
    let ui = Ui::assemble(component, built, image_view, None, structures, None);
    let mut engine = TextEngine::new();
    let mut tree = ui.tree.borrow_mut();

    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot.nodes.first().expect("node");

    assert!(node.width > 1);
}

pub(super) fn state_after(targets: [BindingTarget; 6]) -> (Vec<WidgetId>, WidgetTree) {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
        .declare("title", Value::String("hi".into()), false)
        .unwrap();
    component.declare("on", Value::Bool(true), false).unwrap();
    component.declare("off", Value::Bool(false), false).unwrap();
    component.declare("x", Value::Number(4.0), false).unwrap();
    component.declare("y", Value::Number(6.0), false).unwrap();
    let component = Rc::new(RefCell::new(component));
    let widgets: Vec<Widget> = (0..targets.len())
        .map(|index| Widget::text(index.to_string()))
        .collect();
    let ids: Vec<WidgetId> = widgets.iter().map(Widget::id).collect();
    let mut tree = WidgetTree::new(Widget::view(Direction::Column, widgets));
    for (index, target) in targets.into_iter().enumerate() {
        let binding = binding(&component, ids[index], target);
        let pending = runtime::evaluate(&binding, &runtime::Textures::default());
        runtime::write(pending, binding.widget, &mut tree);
    }
    (ids, tree)
}

#[test]
pub(super) fn every_binding_target_writes_its_own_state() {
    let targets = [
        BindingTarget::Text(text!("title")),
        BindingTarget::Checked(flag!("on")),
        BindingTarget::Selected(flag!("on")),
        BindingTarget::Disabled(flag!("on")),
        BindingTarget::Visible(flag!("off")),
        BindingTarget::Offset {
            x: Some(number!("x")),
            y: Some(number!("y")),
        },
    ];
    let (ids, mut tree) = state_after(targets);
    assert!(!tree.set_text(ids[0], "hi".to_owned()));
    let state = tree.state(ids[1]).expect("state");
    assert!(state.checked);
    assert!(tree.state(ids[2]).expect("state").selected);
    assert!(tree.state(ids[3]).expect("state").disabled);
    assert!(!tree.state(ids[4]).expect("state").visible);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 200,
            height: 200,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot.node(ids[5]).cloned().expect("node");
    assert_eq!((node.x, node.y), (4, 6));
}
