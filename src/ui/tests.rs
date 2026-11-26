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
use crate::{
    Binding, BindingTarget, Component, Dyn, Error, EventSet, RawEventSet, RowInfo, RowKey, Script,
};

const NAMES: &[&str] = &[
    "shown", "label", "colour", "title", "on", "off", "x", "y", "ran", "seen", "code", "accent",
    "missing", "icon", "value", "script", "cleaned", "attach", "window", "first", "second",
    "handled",
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
    let mut build = Build::with_slots(structures[0].slots.clone(), Services::default());
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

#[path = "tests/bindings.rs"]
mod bindings;
#[path = "tests/commands.rs"]
mod commands;
#[path = "tests/dispatch.rs"]
mod dispatch;
#[path = "tests/event_policy.rs"]
mod event_policy;
#[path = "tests/handler_events.rs"]
mod handler_events;
#[path = "tests/timers.rs"]
mod timers;

use bindings::*;
use commands::*;
use dispatch::*;
use event_policy::*;
use handler_events::*;
use timers::*;
