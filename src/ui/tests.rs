use super::*;
use crate::{Scope, Signal, TestTheme};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use brul_gui::{Direction, Size, TextEngine, Texture, Widget};
use brul_syntax::markup::PropType;
use brul_syntax::script::{BUILTINS, BuiltinEmit};
use brul_window::{TimerCommand, WindowCommand, WindowKey};

use crate::run::generated_built;
use crate::{Binding, BindingTarget, Component, Dyn, Error, EventSet, RowInfo, RowKey, Script};

const NAMES: &[&str] = &[
    "shown", "label", "colour", "title", "on", "off", "x", "y", "ran", "seen", "code", "accent",
    "missing", "icon", "value", "script", "cleaned", "attach", "window",
];

fn unknown_theme() -> ViewError {
    ViewError::from(crate::gui::style::StyleError::from(
        crate::style::SnapshotError::UnknownTheme("sepia".to_owned()),
    ))
}

macro_rules! typed_read {
    ($macro:ident, $name:literal, $ty:ty) => {{
        fn read(component: &mut Component) -> Result<$ty, Error> {
            (&component.read($name)?).try_into()
        }
        read
    }};
}

macro_rules! text {
    ($name:literal) => {
        typed_read!(text, $name, String)
    };
}

macro_rules! flag {
    ($name:literal) => {
        typed_read!(flag, $name, bool)
    };
}

macro_rules! number {
    ($name:literal) => {
        typed_read!(number, $name, f64)
    };
}

fn shown(component: &mut Component) -> Result<Value, Error> {
    let flag = component.read("shown")?.as_flag();
    Ok(Value::Bool(flag.unwrap_or(false)))
}

fn rebuild_shown(
    theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
    _old_rows: &[RowInfo],
) -> Result<(Built, Vec<RowInfo>), ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let mut widget = Widget::view(Direction::Column, Vec::new());
    let flag = shown(&mut component.borrow_mut())
        .ok()
        .and_then(|value| value.as_flag());
    if flag.unwrap_or(false) {
        widget.set_children(vec![Widget::text("shown")]);
    }
    Ok((
        Built {
            widget,
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        },
        Vec::new(),
    ))
}

fn rebuild_timer_structure(
    _theme: Option<&str>,
    build: &mut Build,
    component: &ComponentHandle,
    _old_rows: &[RowInfo],
) -> Result<(Built, Vec<RowInfo>), ViewError> {
    let mut widget = Widget::view(Direction::Column, Vec::new());
    let flag = shown(&mut component.borrow_mut())
        .ok()
        .and_then(|value| value.as_flag())
        .unwrap_or(false);
    if flag {
        let child = runtime::instance(
            build,
            component,
            runtime::Instance {
                theme: None,
                view: child_timer_view,
                script: Some(script_with(schedule_child_tick)),
                name: "TimerChild",
                names: NAMES,
                props: &[],
                defaults: &[],
                events: &[],
                slots: &[],
            },
        )?;
        widget.set_children(vec![child.widget]);
    }
    Ok((
        Built {
            widget,
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        },
        Vec::new(),
    ))
}

fn timer_structure_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let holder = Widget::view(Direction::Column, Vec::new());
    runtime::register_structure(
        _build,
        component,
        holder.id(),
        false,
        shown,
        rebuild_timer_structure,
    );
    Ok(Built {
        widget: holder,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn list(component: &mut Component) -> Result<Value, Error> {
    component.read("label")
}

fn list_script(component: &mut Component) -> Result<(), Error> {
    component.declare("label", Value::Array(Vec::new().into()), false)
}

fn rebuild_timer_rows(
    _theme: Option<&str>,
    build: &mut Build,
    component: &ComponentHandle,
    old_rows: &[RowInfo],
) -> Result<(Built, Vec<RowInfo>), ViewError> {
    let Value::Array(items) = component.borrow().read("label").map_err(ViewError::from)? else {
        return Ok((
            Built {
                widget: Widget::view(Direction::Column, Vec::new()),
                bindings: Vec::new(),
                handlers: Vec::new(),
                props: Vec::new(),
            },
            Vec::new(),
        ));
    };
    let mut matcher = crate::each::RowMatcher::new(old_rows);
    let mut children = Vec::new();
    let mut handlers = Vec::new();
    let mut rows = Vec::new();
    for (position, item) in items.iter().cloned().enumerate() {
        let key = RowKey::Explicit(item.clone());
        let start = children.len();
        if let Some(other) = matcher.take(&key, &item, position) {
            rows.push(RowInfo {
                key,
                item,
                start,
                count: 0,
                reuse: Some(other),
            });
            continue;
        }
        let child = runtime::instance(
            build,
            component,
            runtime::Instance {
                theme: None,
                view: child_timer_view,
                script: Some(script_with(schedule_child_tick)),
                name: "TimerChild",
                names: NAMES,
                props: &[],
                defaults: &[],
                events: &[],
                slots: &[],
            },
        )?;
        children.push(child.widget);
        handlers.extend(child.handlers);
        rows.push(RowInfo {
            key,
            item,
            start,
            count: 1,
            reuse: None,
        });
    }
    Ok((
        Built {
            widget: Widget::view(Direction::Column, children),
            bindings: Vec::new(),
            handlers,
            props: Vec::new(),
        },
        rows,
    ))
}

fn timer_rows_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let holder = Widget::view(Direction::Column, Vec::new());
    runtime::register_structure(
        _build,
        component,
        holder.id(),
        true,
        list,
        rebuild_timer_rows,
    );
    Ok(Built {
        widget: holder,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn structured_view(
    theme: Option<TestTheme>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let widget = Widget::view(Direction::Column, Vec::new());
    runtime::register_structure(build, component, widget.id(), false, shown, rebuild_shown);
    Ok(Built {
        widget,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn child_count<T: Theme>(ui: &Ui<T>, parent: WidgetId) -> usize {
    let children = ui
        .tree
        .borrow_mut()
        .take_children(parent)
        .unwrap_or_default();
    let count = children.len();
    ui.tree.borrow_mut().replace_all_children(parent, children);
    count
}

#[test]
fn structure_rebuilds_when_its_value_changes() {
    let (component, built, structures) = generated_built(
        structured_view,
        NAMES,
        None,
        &[],
        Some(script_with(shown_false)),
    )
    .expect("launch succeeds");
    let parent = built.widget.id();
    let mut ui = Ui::assemble(component, built, structured_view, None, structures, None);
    brul_reactive::flush();
    assert_eq!(child_count(&ui, parent), 0);

    ui.component
        .borrow_mut()
        .assign("shown", Value::Bool(true))
        .unwrap();
    brul_reactive::flush();
    ui.settle();
    assert_eq!(child_count(&ui, parent), 1);
}

fn slot_child_view(
    _theme: Option<&str>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let holder = Widget::view(Direction::Column, Vec::new());
    runtime::register_structure(build, component, holder.id(), false, shown, rebuild_slot);
    Ok(Built {
        widget: holder,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn rebuild_slot(
    _theme: Option<&str>,
    build: &mut Build,
    _component: &ComponentHandle,
    _old_rows: &[RowInfo],
) -> Result<(Built, Vec<RowInfo>), ViewError> {
    let built = runtime::slot(None, build, runtime::DEFAULT_SLOT)?;
    Ok((
        Built {
            widget: Widget::view(Direction::Column, built.widget.into_children()),
            bindings: built.bindings,
            handlers: built.handlers,
            props: built.props,
        },
        Vec::new(),
    ))
}

fn slot_fill(
    _theme: Option<&str>,
    _build: &mut Build,
    _component: &ComponentHandle,
) -> Result<Built, ViewError> {
    Ok(Built {
        widget: Widget::view(Direction::Column, vec![Widget::text("filled")]),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn slot_view(
    theme: Option<TestTheme>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let slots = [runtime::SlotArgument {
        name: runtime::DEFAULT_SLOT,
        view: slot_fill,
    }];
    runtime::instance(
        build,
        component,
        runtime::Instance {
            theme: theme.map(Theme::name),
            view: slot_child_view,
            script: None,
            name: "Slot",
            names: NAMES,
            props: &[],
            defaults: &[],
            events: &[],
            slots: &slots,
        },
    )
}

#[test]
fn a_slot_survives_a_structure_rebuild() {
    let (_component, _built, structures) =
        generated_built(slot_view, NAMES, None, &[], None).expect("launch");
    assert_eq!(structures.len(), 1);
    let mut build = Build::with_slots(structures[0].slots.clone());
    let rebuilt =
        (structures[0].rebuild)(None, &mut build, &structures[0].component, &[]).expect("rebuild");
    assert_eq!(rebuilt.0.widget.into_children().len(), 1);
}

fn bound_view(
    theme: Option<TestTheme>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let holder = Widget::view(Direction::Column, Vec::new());
    runtime::register_structure(build, component, holder.id(), false, shown, rebuild_bound);
    Ok(Built {
        widget: holder,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

fn rebuild_bound(
    theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
    _old_rows: &[RowInfo],
) -> Result<(Built, Vec<RowInfo>), ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let mut children = Vec::new();
    let mut bindings = Vec::new();
    let visible = shown(&mut component.borrow_mut())
        .ok()
        .and_then(|value| value.as_flag())
        .unwrap_or(false);
    if visible {
        let text = Widget::text("bound");
        bindings.push(Binding::new(
            component,
            text.id(),
            BindingTarget::Text(text!("label")),
        ));
        children.push(text);
    }
    Ok((
        Built {
            widget: Widget::view(Direction::Column, children),
            bindings,
            handlers: Vec::new(),
            props: Vec::new(),
        },
        Vec::new(),
    ))
}

#[test]
fn the_effect_of_a_removed_branch_dies_with_its_widget() {
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

fn declared(name: &'static str, value: Value) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    component.declare(name, value, false).expect("declared");
    Rc::new(RefCell::new(component))
}

fn prop(
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

fn child_with_property(name: &str, value: Value) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    let id = component.id(name).expect("prop name is in the table");
    component
        .declare_property(id, value)
        .expect("declared property");
    Rc::new(RefCell::new(component))
}

#[test]
fn a_prop_binding_writes_the_property_of_the_child() {
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
fn a_prop_binding_skips_a_value_of_the_wrong_type() {
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
fn a_prop_binding_survives_an_unknown_property() {
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

fn command_host(app: Option<Box<dyn Host>>) -> (CommandHost, Rc<RefCell<Vec<WindowCommand>>>) {
    let commands = Rc::new(RefCell::new(Vec::new()));
    (
        CommandHost {
            commands: Rc::clone(&commands),
            app,
            windows: Rc::new(HashMap::new()),
            theme: Rc::new(RefCell::new(None)),
        },
        commands,
    )
}

#[test]
fn theme_reports_the_shown_theme() {
    let (mut host, _) = command_host(None);
    assert_eq!(host.call("theme", &[]), Ok(Value::Null));

    *host.theme.borrow_mut() = Some("dark".to_owned());
    assert_eq!(
        host.call("theme", &[]),
        Ok(Value::String("dark".to_owned().into()))
    );
    assert_eq!(
        host.call("theme", &[Value::String("dark".into())]),
        Err(Error::Arity {
            name: "theme".to_owned(),
            expected: "no",
        })
    );
}

struct Recorder {
    calls: Rc<RefCell<Vec<String>>>,
}

impl Host for Recorder {
    fn call(&mut self, name: &str, _arguments: &[Value]) -> Result<Value, Error> {
        self.calls.borrow_mut().push(name.to_owned());
        Ok(Value::Null)
    }
}

#[test]
fn set_theme_takes_exactly_one_string() {
    let (mut host, commands) = command_host(None);
    assert_eq!(
        host.call("setTheme", &[]),
        Err(Error::Arity {
            name: "setTheme".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(
        host.call("setTheme", &[Value::Number(1.0)]),
        Err(Error::Argument {
            name: "setTheme",
            expected: "String",
            found: "Number",
        })
    );
    assert_eq!(
        host.call("setTheme", &[Value::String("dark".into())]),
        Ok(Value::Null)
    );
    assert_eq!(
        *commands.borrow(),
        vec![WindowCommand::SetTheme("dark".to_owned())]
    );
}

#[test]
fn set_always_on_top_takes_exactly_one_flag() {
    let (mut host, commands) = command_host(None);
    assert_eq!(
        host.call("setAlwaysOnTop", &[]),
        Err(Error::Arity {
            name: "setAlwaysOnTop".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(
        host.call("setAlwaysOnTop", &[Value::Number(1.0)]),
        Err(Error::Argument {
            name: "setAlwaysOnTop",
            expected: "Bool",
            found: "Number",
        })
    );
    assert_eq!(
        host.call("setAlwaysOnTop", &[Value::Bool(true)]),
        Ok(Value::Null)
    );
    assert_eq!(*commands.borrow(), vec![WindowCommand::AlwaysOnTop(true)]);
}

#[test]
fn every_host_builtin_is_dispatched() {
    let (mut host, _) = command_host(None);
    let names: Vec<_> = BUILTINS
        .iter()
        .filter(|builtin| matches!(builtin.emit, BuiltinEmit::Host { .. }))
        .map(|builtin| builtin.name)
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(
            !matches!(host.call(name, &[]), Err(Error::UnknownFunction(_))),
            "{name} is not dispatched"
        );
    }
}

#[test]
fn every_host_builtin_has_a_marker() {
    let names: Vec<_> = BUILTINS
        .iter()
        .filter_map(|builtin| match builtin.emit {
            BuiltinEmit::Host { marker } => Some((builtin.name, marker)),
            _ => None,
        })
        .collect();
    assert!(!names.is_empty());
    for (name, marker) in names {
        let entry = crate::builtins::HOST_MARKERS
            .iter()
            .find(|(marker_name, _, _)| *marker_name == name)
            .unwrap_or_else(|| panic!("{name} has no generated marker"));
        assert_eq!(entry.1, marker);
        assert_eq!((entry.2)(), name);
    }
}

#[test]
fn the_builtin_commands_take_no_arguments() {
    let (mut host, commands) = command_host(None);
    assert_eq!(host.call("exit", &[]), Ok(Value::Null));
    assert_eq!(host.call("closeWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("dragWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("showWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("hideWindow", &[]), Ok(Value::Null));
    assert_eq!(host.call("focusWindow", &[]), Ok(Value::Null));
    assert_eq!(
        host.call("exit", &[Value::Number(1.0)]),
        Err(Error::Arity {
            name: "exit".to_owned(),
            expected: "no",
        })
    );
    assert_eq!(
        *commands.borrow(),
        vec![
            WindowCommand::Close,
            WindowCommand::Close,
            WindowCommand::Drag,
            WindowCommand::Show,
            WindowCommand::Hide,
            WindowCommand::Focus
        ]
    );
}

#[test]
fn open_window_finds_a_declared_name() {
    let commands = Rc::new(RefCell::new(Vec::new()));
    let mut windows = HashMap::new();
    windows.insert("Settings".to_owned(), WindowKey(1));
    let mut host = CommandHost {
        commands: Rc::clone(&commands),
        app: None,
        windows: Rc::new(windows),
        theme: Rc::new(RefCell::new(None)),
    };

    assert_eq!(
        host.call("openWindow", &[Value::String("Settings".into())]),
        Ok(Value::Null)
    );
    assert_eq!(
        host.call("openWindow", &[Value::String("Missing".into())]),
        Err(Error::UnknownWindow("Missing".to_owned()))
    );
    assert_eq!(
        host.call("openWindow", &[]),
        Err(Error::Arity {
            name: "openWindow".to_owned(),
            expected: "one",
        })
    );
    assert_eq!(*commands.borrow(), vec![WindowCommand::Open(WindowKey(1))]);
}

#[test]
fn an_unknown_command_without_a_host_is_an_error() {
    let (mut host, _commands) = command_host(None);
    assert_eq!(
        host.call("appCommand", &[]),
        Err(Error::UnknownFunction("appCommand".to_owned()))
    );
}

#[test]
fn commands_reach_the_application_host() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorder = Recorder {
        calls: Rc::clone(&calls),
    };
    let (mut host, _commands) = command_host(Some(Box::new(recorder)));
    assert_eq!(host.call("appCommand", &[]), Ok(Value::Null));
    assert_eq!(*calls.borrow(), vec!["appCommand".to_owned()]);
}

fn themed_view(
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

fn rejecting_view(
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
fn a_theme_change_rebuilds_the_view() {
    let (component, built, structures) =
        generated_built(themed_view, NAMES, None, &[], None).expect("launch");
    let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
    ui.set_theme(TestTheme::Dark).expect("theme changes");
    assert_eq!(ui.theme, Some(TestTheme::Dark));
}

#[test]
fn a_failed_theme_change_keeps_the_previous_theme() {
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

fn binding(component: &ComponentHandle, widget: WidgetId, target: BindingTarget) -> Binding {
    Binding::new(component, widget, target)
}

fn icon(component: &mut Component) -> Result<Dyn, Error> {
    Ok(Dyn::from(component.read("icon")?))
}

fn image_view(
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
fn an_image_binding_reaches_the_tree_through_ui() {
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

fn state_after(targets: [BindingTarget; 6]) -> (Vec<WidgetId>, WidgetTree) {
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
fn every_binding_target_writes_its_own_state() {
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

fn handler_dispatch(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, Error> {
    match name {
        "clicked" | "closing" | "tick" | "first" => {
            component.assign("ran", Value::Bool(true))?;
        }
        "second" => component.assign("off", Value::Bool(true))?,
        "pressed" => component.assign("seen", Value::from(arguments[0].clone()))?,
        _ => return Err(Error::UnknownFunction(name.to_owned())),
    }
    Ok(Dyn::null())
}

fn component_with(declared: &[&str]) -> ComponentHandle {
    let mut component = Component::new();
    component.set_names(NAMES);
    for name in declared {
        component
            .declare(name, Value::Null, false)
            .expect("declared");
    }
    component.set_dispatch(handler_dispatch);
    component.register_event_call("tick", set_ran);
    component.register_event_call("applied", applied_call);
    component.register_event_call("schedule", schedule_tick);
    Rc::new(RefCell::new(component))
}

fn noop_init(_: &mut Component) -> Result<(), Error> {
    Ok(())
}

fn noop_dispatch(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, Error> {
    Ok(Dyn::null())
}

fn raise_dispatch(
    component: &mut Component,
    name: &str,
    arguments: Vec<Dyn>,
) -> Result<Dyn, Error> {
    if name == "raise" {
        let arguments = arguments.into_iter().map(Value::from).collect();
        Ok(Dyn::from(brul_script::eval::call(
            component, "clicked", arguments,
        )?))
    } else {
        Err(Error::UnknownFunction(name.to_owned()))
    }
}

fn applied_dispatch(component: &mut Component, name: &str, _: Vec<Dyn>) -> Result<Dyn, Error> {
    if name == "applied" {
        component.assign("label", Value::String("after".into()))?;
        Ok(Dyn::null())
    } else {
        Err(Error::UnknownFunction(name.to_owned()))
    }
}

fn set_ran(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("ran", Value::Bool(true))?;
    Ok(Dyn::null())
}

fn schedule_tick(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.every(1000.0, "tick")?;
    Ok(Dyn::null())
}

fn set_off(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("off", Value::Bool(true))?;
    Ok(Dyn::null())
}

fn key_code(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("seen", Value::from(arguments[0].clone()))?;
    Ok(Dyn::null())
}

fn raise_call(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    raise_dispatch(component, "raise", arguments.to_vec())
}

fn applied_call(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("label", Value::String("after".into()))?;
    Ok(Dyn::null())
}

fn script_with(init: fn(&mut Component) -> Result<(), Error>) -> Script {
    Script::Generated {
        init,
        dispatch: noop_dispatch,
    }
}

fn shown_false(component: &mut Component) -> Result<(), Error> {
    component.declare("shown", Value::Bool(false), false)
}

fn shown_and_label(component: &mut Component) -> Result<(), Error> {
    component.declare("shown", Value::Bool(false), false)?;
    component.declare("label", Value::String("bound".into()), false)
}

fn dispatching(
    component: &ComponentHandle,
    widget: Widget,
    handlers: impl FnOnce(WidgetId) -> Vec<Handler>,
) -> (WidgetId, Ui<TestTheme>) {
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: handlers(id),
        props: Vec::new(),
    };
    (
        id,
        Ui::assemble(
            Rc::clone(component),
            built,
            themed_view,
            None,
            Vec::new(),
            None,
        ),
    )
}

#[test]
fn dispatch_calls_the_handler_of_the_widget() {
    let component = component_with(&["ran"]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
        vec![Handler::new(
            &component,
            Some(id),
            EventKind::Click,
            set_ran,
        )]
    });
    ui.dispatch(&[UiEvent::widget(id, Event::click())]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

struct ProbeSource {
    value: Signal<String>,
}

#[derive(Default)]
struct ProbeState {
    applied: Vec<String>,
}

fn probe_apply(ctx: &mut Build, state: StateId, params: &[Dyn]) -> Result<(), ViewError> {
    if ctx.get_state::<ProbeState>().is_none() {
        ctx.set_state(ProbeState::default());
    }
    let value = params
        .first()
        .and_then(|param| String::try_from(param.clone()).ok())
        .unwrap_or_default();
    if let Some(probe) = ctx.get_state::<ProbeState>() {
        probe.borrow_mut().applied.push(value);
    }
    let _ = state;
    Ok(())
}

fn probe_event(ctx: &mut Build, _state: StateId, event: &UiEvent) -> bool {
    if matches!(event.event, Event::Click { .. }) {
        ctx.component()
            .borrow_mut()
            .assign("ran", Value::Bool(true))
            .expect("ran");
        true
    } else {
        false
    }
}

fn probe_cleanup(ctx: &mut Build, _state: StateId) {
    ctx.component()
        .borrow_mut()
        .assign("cleaned", Value::Bool(true))
        .expect("cleaned");
}

fn set_script(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
    component.assign("script", Value::Bool(true))?;
    Ok(Dyn::null())
}

#[test]
fn attachment_reruns_vetoes_events_and_cleans_up() {
    let component = component_with(&["ran", "script", "cleaned"]);
    component.borrow_mut().set_state(ProbeSource {
        value: Signal::new("first".to_owned()),
    });
    let widget = Widget::text("");
    let id = widget.id();
    let hooks = Hooks {
        widget: id,
        state: StateId::new(7),
        component: Rc::clone(&component),
        params: vec![|component: &mut Component| -> Result<Dyn, Error> {
            Ok(Dyn::text(component.state::<ProbeSource>().value.get()))
        }],
        apply: probe_apply,
        event: probe_event,
        cleanup: probe_cleanup,
        events: EventSet::from([EventKind::Click]),
    };
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![Handler::new(
            &component,
            Some(id),
            EventKind::Click,
            set_script,
        )],
        props: Vec::new(),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![hooks],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.settle();
    let state = StateId::new(7).for_widget(id);
    let probe = ui
        .states
        .borrow()
        .get_cloned::<RefCell<ProbeState>>(state)
        .expect("probe state");
    assert_eq!(probe.borrow().applied, ["first".to_owned()]);

    component
        .borrow_mut()
        .state::<ProbeSource>()
        .value
        .set("second".to_owned());
    brul_reactive::flush();
    ui.settle();
    assert_eq!(
        probe.borrow().applied,
        ["first".to_owned(), "second".to_owned()]
    );

    ui.dispatch(&[UiEvent::widget(id, Event::click())]);
    assert_eq!(component.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_ne!(component.borrow().read("script"), Ok(Value::Bool(true)));

    drop(ui);
    assert_eq!(component.borrow().read("cleaned"), Ok(Value::Bool(true)));
}

#[test]
fn dispatch_runs_the_first_of_two_handlers_of_a_widget() {
    let component = component_with(&["ran", "off"]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
        vec![
            Handler::new(&component, Some(id), EventKind::Click, set_ran),
            Handler::new(&component, Some(id), EventKind::Click, set_off),
        ]
    });

    ui.dispatch(&[UiEvent::widget(id, Event::click())]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_ne!(ui.component.borrow().read("off"), Ok(Value::Bool(true)));
}

#[test]
fn dispatch_passes_the_key_code() {
    let component = component_with(&["seen"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
        vec![Handler::new(&component, None, EventKind::Key, key_code)]
    });
    ui.dispatch(&[UiEvent::window(Event::key(
        brul_gui::KeyPress::from("Enter"),
        brul_gui::KeyModifiers::default(),
    ))]);
    let seen = ui.component.borrow().read("seen").expect("seen");
    assert_eq!(
        seen,
        Value::Object(
            vec![
                ("code".to_owned(), Value::String("Enter".into())),
                ("physical".to_owned(), Value::Null),
                ("ctrl".to_owned(), Value::Bool(false)),
                ("shift".to_owned(), Value::Bool(false)),
                ("alt".to_owned(), Value::Bool(false)),
                ("meta".to_owned(), Value::Bool(false)),
                ("target".to_owned(), Value::Null),
            ]
            .into()
        )
    );
}

fn record_attachment_target(ctx: &mut Build, _state: StateId, event: &UiEvent) -> bool {
    if let Some(id) = event.target.widget() {
        ctx.component()
            .borrow_mut()
            .assign("attach", Value::Number(id.get() as f64))
            .expect("attach");
    }
    false
}

fn record_script_target(
    component: &mut Component,
    name: &str,
    arguments: &[Dyn],
) -> Result<Dyn, Error> {
    let target = arguments[0].field("target")?.as_number()?;
    component.assign(name, Value::Number(target))?;
    Ok(Dyn::null())
}

fn record_widget_script_target(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    record_script_target(component, "script", arguments)
}

fn record_window_script_target(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
    record_script_target(component, "window", arguments)
}

#[test]
fn one_targeted_event_reaches_the_attachment_script_and_window_handlers() {
    let component = component_with(&["attach", "script", "window"]);
    let widget = Widget::text("");
    let id = widget.id();
    let built = Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![
            Handler::new(
                &component,
                Some(id),
                EventKind::Key,
                record_widget_script_target,
            ),
            Handler::new(
                &component,
                None,
                EventKind::Key,
                record_window_script_target,
            ),
        ],
        props: Vec::new(),
    };
    let hooks = Hooks {
        widget: id,
        state: StateId::new(7),
        component: Rc::clone(&component),
        params: Vec::new(),
        apply: probe_apply,
        event: record_attachment_target,
        cleanup: |_, _| {},
        events: EventSet::from([EventKind::Key]),
    };
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        themed_view,
        None,
        AssemblyParts {
            structures: Vec::new(),
            capabilities: vec![hooks],
            nested: Vec::new(),
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
        },
        Rc::new(RefCell::new(Vec::new())),
        false,
    );
    ui.dispatch(&[UiEvent::widget(
        id,
        Event::key(
            brul_gui::KeyPress::from("a"),
            brul_gui::KeyModifiers::default(),
        ),
    )]);
    let expected = Value::Number(id.get() as f64);
    let component = ui.component.borrow();
    assert_eq!(component.read("attach"), Ok(expected.clone()));
    assert_eq!(component.read("script"), Ok(expected.clone()));
    assert_eq!(component.read("window"), Ok(expected));
}

#[test]
fn a_close_handler_keeps_the_window_open() {
    let component = component_with(&["ran"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
        vec![Handler::new(&component, None, EventKind::Close, set_ran)]
    });

    assert!(ui.dispatch(&[UiEvent::window(Event::close())]).is_empty());
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
fn a_close_without_a_handler_closes_the_window() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());

    assert_eq!(
        ui.dispatch(&[UiEvent::window(Event::close())]),
        vec![WindowCommand::Close]
    );
}

#[test]
fn dispatch_runs_a_timer() {
    let component = component_with(&["ran"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    ui.dispatch(&[UiEvent::window(Event::timer(
        TimerOwner::new(0),
        "tick".into(),
    ))]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
fn owners_are_unique_across_rebuilds() {
    let component = component_with(&[]);
    let built = themed_view(None, &mut Build::new(), &component).expect("view");
    let mut ui = Ui::assemble(component, built, themed_view, None, Vec::new(), None);
    let first = ui.components[0].owner;
    ui.set_theme(TestTheme::Dark).expect("theme changes");
    let second = ui.components[0].owner;
    ui.set_theme(TestTheme::Light).expect("theme changes");
    let third = ui.components[0].owner;
    assert_ne!(first, second);
    assert_ne!(second, third);
}

#[test]
fn a_timer_from_a_rebuilt_component_does_not_reach_the_new_one() {
    let component = component_with(&["ran"]);
    component.borrow_mut().every(500.0, "tick").expect("every");
    let built = themed_view(None, &mut Build::new(), &component).expect("view");
    let mut ui = Ui::assemble(component, built, themed_view, None, Vec::new(), None);
    let stale = ui.components[0].owner;
    ui.set_theme(TestTheme::Dark).expect("theme changes");
    assert_ne!(ui.components[0].owner, stale);
    ui.dispatch(&[UiEvent::window(Event::timer(stale, "tick".into()))]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Null));
}

#[test]
fn a_vanished_if_branch_cancels_its_component_timer() {
    let script = script_with(shown_false);
    let (component, built, structures) =
        generated_built(timer_structure_view, NAMES, None, &[], Some(script))
            .expect("launch succeeds");
    let mut ui = Ui::assemble(
        component,
        built,
        timer_structure_view,
        None,
        structures,
        None,
    );
    brul_reactive::flush();
    ui.component
        .borrow_mut()
        .assign("shown", Value::Bool(true))
        .expect("assign");
    brul_reactive::flush();
    ui.settle();
    assert_eq!(ui.components.len(), 2);
    let owner = ui.components[1].owner;
    let starts = ui.dispatch(&[]);
    let id = match &starts[0] {
        WindowCommand::Timer(TimerCommand::Start { id, .. }) => *id,
        other => panic!("expected a timer start, got {other:?}"),
    };

    ui.component
        .borrow_mut()
        .assign("shown", Value::Bool(false))
        .expect("assign");
    brul_reactive::flush();
    ui.settle();
    assert_eq!(ui.components.len(), 1);
    assert_eq!(
        ui.dispatch(&[]),
        vec![WindowCommand::Timer(TimerCommand::Cancel { id, owner })]
    );
}

#[test]
fn a_removed_each_row_cancels_only_its_component_timer() {
    let script = script_with(list_script);
    let (component, built, structures) =
        generated_built(timer_rows_view, NAMES, None, &[], Some(script)).expect("launch succeeds");
    let mut ui = Ui::assemble(component, built, timer_rows_view, None, structures, None);
    brul_reactive::flush();
    ui.component
        .borrow_mut()
        .assign(
            "label",
            Value::Array(vec![Value::String("a".into()), Value::String("b".into())].into()),
        )
        .expect("assign");
    brul_reactive::flush();
    ui.settle();
    assert_eq!(ui.components.len(), 3);
    let starts = ui.dispatch(&[]);
    let timers: Vec<_> = starts
        .iter()
        .map(|command| match command {
            WindowCommand::Timer(TimerCommand::Start { id, owner, .. }) => (*id, *owner),
            other => panic!("expected a timer start, got {other:?}"),
        })
        .collect();
    assert_eq!(timers.len(), 2);

    ui.component
        .borrow_mut()
        .assign(
            "label",
            Value::Array(vec![Value::String("a".into())].into()),
        )
        .expect("assign");
    brul_reactive::flush();
    ui.settle();
    assert_eq!(ui.components.len(), 2);
    let (removed_id, removed_owner) = timers
        .iter()
        .find(|(_, owner)| !ui.components.iter().any(|owned| owned.owner == *owner))
        .copied()
        .expect("one owner is gone");
    assert_eq!(
        ui.dispatch(&[]),
        vec![WindowCommand::Timer(TimerCommand::Cancel {
            id: removed_id,
            owner: removed_owner
        })]
    );
}

#[test]
fn every_before_assembly_becomes_a_window_command() {
    let component = component_with(&[]);
    component.borrow_mut().every(500.0, "tick").expect("every");
    let (_, ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    assert_eq!(
        *ui.commands.borrow(),
        vec![WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(0),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            interval: Duration::from_secs_f64(0.5),
            kind: brul_ident::TimerKind::Every,
        })]
    );
}

#[test]
fn every_from_a_handler_is_scheduled_without_an_app_host() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    assert_eq!(
        ui.dispatch(&[UiEvent::window(Event::timer(
            TimerOwner::new(0),
            "schedule".into()
        ))]),
        vec![WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(0),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            interval: Duration::from_secs(1),
            kind: brul_ident::TimerKind::Every,
        })]
    );
}

#[test]
fn dispatch_ignores_events_without_a_handler() {
    let component = component_with(&[]);
    let (id, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    let other = Widget::text("other").id();
    assert!(
        ui.dispatch(&[
            UiEvent::widget(id, Event::drag()),
            UiEvent::widget(other, Event::click()),
        ])
        .is_empty()
    );
    assert!(ui.component.borrow().read("ran").is_err());
}

#[test]
fn dispatch_returns_commands_it_cannot_handle() {
    let component = component_with(&[]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    ui.commands.borrow_mut().push(WindowCommand::Exit);
    assert_eq!(ui.dispatch(&[]), vec![WindowCommand::Exit]);
}

#[test]
fn dispatch_returns_the_theme_command() {
    let (component, built, structures) =
        generated_built(themed_view, NAMES, None, &[], None).expect("launch");
    let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
    ui.commands
        .borrow_mut()
        .push(WindowCommand::SetTheme("dark".to_owned()));
    assert_eq!(
        ui.dispatch(&[]),
        vec![WindowCommand::SetTheme("dark".to_owned())]
    );
    assert!(ui.theme.is_none());
}

fn child_view(
    theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let text = Widget::text("child");
    let id = text.id();
    Ok(Built {
        widget: text,
        bindings: Vec::new(),
        handlers: vec![Handler::new(
            component,
            Some(id),
            EventKind::Click,
            raise_call,
        )],
        props: Vec::new(),
    })
}

fn raising_view(
    theme: Option<TestTheme>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let script = Script::Generated {
        init: noop_init,
        dispatch: raise_dispatch,
    };
    let events = [runtime::ComponentEvent {
        name: "clicked",
        call: applied_call,
        params: &[],
        component: Rc::clone(component),
    }];
    let child = runtime::instance(
        &mut Build::new(),
        component,
        runtime::Instance {
            theme: theme.map(Theme::name),
            view: child_view,
            script: Some(script),
            name: "Child",
            names: NAMES,
            props: &[],
            defaults: &[],
            events: &events,
            slots: &[],
        },
    )?;
    let text = Widget::text("");
    let id = text.id();
    Ok(Built {
        widget: Widget::view(Direction::Column, vec![child.widget, text]),
        bindings: vec![Binding::new(
            component,
            id,
            BindingTarget::Text(text!("label")),
        )],
        handlers: child.handlers,
        props: Vec::new(),
    })
}

#[test]
fn a_handler_event_runs_after_the_handler_returns() {
    let (component, built, structures) =
        generated_built(raising_view, NAMES, None, &[], None).expect("launch");
    component
        .borrow_mut()
        .declare("label", Value::String("".into()), false)
        .expect("declared");
    let click = built.handlers[0].widget.expect("widget");
    let text = built.bindings[0].widget;
    component.borrow_mut().set_dispatch(applied_dispatch);
    let mut ui = Ui::assemble(component, built, raising_view, None, structures, None);

    ui.dispatch(&[UiEvent::widget(click, Event::click())]);

    assert_eq!(
        ui.component.borrow().read("label"),
        Ok(Value::String("after".into()))
    );
    assert!(!ui.tree.borrow_mut().set_text(text, "after".to_owned()));
}

fn child_timer_view(
    _theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let widget = Widget::text("child");
    let id = widget.id();
    Ok(Built {
        widget,
        bindings: Vec::new(),
        handlers: vec![Handler::new(component, Some(id), EventKind::Click, set_ran)],
        props: Vec::new(),
    })
}

fn schedule_child_tick(component: &mut Component) -> Result<(), Error> {
    component.declare("ran", Value::Bool(false), false)?;
    component.register_event_call("child_tick", set_ran);
    component.every(500.0, "child_tick")?;
    Ok(())
}

fn nested_timer_view(
    theme: Option<TestTheme>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    if theme.is_some() {
        return Err(unknown_theme());
    }
    let child = runtime::instance(
        build,
        component,
        runtime::Instance {
            theme: theme.map(Theme::name),
            view: child_timer_view,
            script: Some(script_with(schedule_child_tick)),
            name: "TimerChild",
            names: NAMES,
            props: &[],
            defaults: &[],
            events: &[],
            slots: &[],
        },
    )?;
    Ok(Built {
        widget: child.widget,
        bindings: Vec::new(),
        handlers: child.handlers,
        props: Vec::new(),
    })
}

#[test]
fn a_timer_from_a_nested_component_runs_in_its_component() {
    let root = component_with(&["ran"]);
    root.borrow_mut().register_event_call("child_tick", set_ran);
    let mut build = Build::new();
    let built = nested_timer_view(None, &mut build, &root).expect("view");
    let nested = build.take_components();
    let structures = build.take_structures();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::assemble_with(
        Rc::clone(&root),
        built,
        nested_timer_view,
        None,
        AssemblyParts {
            structures,
            capabilities: Vec::new(),
            nested,
            states: Rc::new(RefCell::new(crate::States::default())),
            event_sink: None,
        },
        Rc::clone(&commands),
        true,
    );
    assert_eq!(
        *commands.borrow(),
        vec![WindowCommand::Timer(TimerCommand::Start {
            id: brul_ident::TimerId::new(0),
            owner: TimerOwner::new(1),
            function: "child_tick".into(),
            interval: Duration::from_secs_f64(0.5),
            kind: brul_ident::TimerKind::Every,
        })]
    );
    let child = Rc::clone(&ui.components[1].component);
    ui.dispatch(&[UiEvent::window(Event::timer(
        TimerOwner::new(1),
        "child_tick".into(),
    ))]);
    assert_eq!(child.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_eq!(root.borrow().read("ran"), Ok(Value::Null));
}
