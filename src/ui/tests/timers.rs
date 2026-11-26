use super::*;
#[test]
pub(super) fn a_handler_event_runs_after_the_handler_returns() {
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

    ui.dispatch(&[DispatchEvent::semantic(click, Event::click())]);

    assert_eq!(
        ui.component.borrow().read("label"),
        Ok(Value::String("after".into()))
    );
    assert!(!ui.tree.borrow_mut().set_text(text, "after".to_owned()));
}

pub(super) fn child_timer_view(
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

pub(super) fn schedule_child_tick(component: &mut Component) -> Result<(), Error> {
    component.declare("ran", Value::Bool(false), false)?;
    component.register_event_call("child_tick", set_ran);
    component.every(500.0, "child_tick")?;
    Ok(())
}

pub(super) fn nested_timer_view(
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
pub(super) fn a_timer_from_a_nested_component_runs_in_its_component() {
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
    ui.dispatch(&[UiEvent::window(Event::timer(TimerOwner::new(1), "child_tick".into())).into()]);
    assert_eq!(child.borrow().read("ran"), Ok(Value::Bool(true)));
    assert_eq!(root.borrow().read("ran"), Ok(Value::Null));
}
#[test]
pub(super) fn dispatch_runs_a_timer() {
    let component = component_with(&["ran"]);
    let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
    ui.dispatch(&[UiEvent::window(Event::timer(TimerOwner::new(0), "tick".into())).into()]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
}

#[test]
pub(super) fn owners_are_unique_across_rebuilds() {
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
pub(super) fn a_timer_from_a_rebuilt_component_does_not_reach_the_new_one() {
    let component = component_with(&["ran"]);
    component.borrow_mut().every(500.0, "tick").expect("every");
    let built = themed_view(None, &mut Build::new(), &component).expect("view");
    let mut ui = Ui::assemble(component, built, themed_view, None, Vec::new(), None);
    let stale = ui.components[0].owner;
    ui.set_theme(TestTheme::Dark).expect("theme changes");
    assert_ne!(ui.components[0].owner, stale);
    ui.dispatch(&[UiEvent::window(Event::timer(stale, "tick".into())).into()]);
    assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Null));
}

#[test]
pub(super) fn a_vanished_if_branch_cancels_its_component_timer() {
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
pub(super) fn a_removed_each_row_cancels_only_its_component_timer() {
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

pub(super) fn raising_view(
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
