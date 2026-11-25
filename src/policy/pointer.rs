use crate::{Build, Dispatch, Event, RawEvent, RawUiEvent, UiEvent, ViewError, gui::WidgetId};

pub struct PointerPolicy {
    pressed: Option<WidgetId>,
}

#[brul::attach(
    name = "pointer",
    raw_events(
        Press,
        Release,
        PointerMove,
        PointerEnter,
        PointerLeave,
        PointerCancel,
        Drag
    ),
    raw_event = pointer_raw
)]
pub fn pointer(_build: &mut Build) -> Result<PointerPolicy, ViewError> {
    Ok(PointerPolicy { pressed: None })
}

fn pointer_raw(state: &mut PointerPolicy, build: &mut Build, event: &RawUiEvent) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    match &event.event {
        RawEvent::Press { x, y, .. } => press(state, build, event.target.widget(), *x, *y),
        RawEvent::Release { x, y, .. } => release(state, build, event.target.widget(), *x, *y),
        RawEvent::PointerMove { x, y, .. } => {
            move_pointer(state, build, event.target.widget(), *x, *y)
        }
        RawEvent::PointerEnter {} => hover(build, event.target.widget(), true),
        RawEvent::PointerLeave {} => hover(build, event.target.widget(), false),
        RawEvent::PointerCancel {} => cancel(state, build, event.target.widget()),
        RawEvent::Drag { .. } => Dispatch::CONTINUE,
        _ => Dispatch::CONTINUE,
    }
}

fn press(
    state: &mut PointerPolicy,
    build: &mut Build,
    target: Option<WidgetId>,
    x: i32,
    y: i32,
) -> Dispatch {
    let Some(id) = target else {
        return Dispatch::CONTINUE;
    };
    let primary = build.hit(x, y) == Some(id);
    if primary {
        state.pressed = Some(id);
        focus(build, id);
        place_caret(build, id, x, y, false);
    }
    changed(
        build
            .element_for(id)
            .is_some_and(|element| element.set_pressed(true)),
    )
}

fn release(
    state: &mut PointerPolicy,
    build: &mut Build,
    target: Option<WidgetId>,
    x: i32,
    y: i32,
) -> Dispatch {
    let Some(id) = target else {
        return Dispatch::CONTINUE;
    };
    let Some(anchor) = state.pressed else {
        return changed(
            build
                .element_for(id)
                .is_some_and(|element| element.set_pressed(false)),
        );
    };
    let last = build.path_to(anchor).first() == Some(&id);
    let clicks = last && build.hit(x, y) == Some(anchor);
    if last {
        state.pressed = None;
    }
    let was_changed = build
        .element_for(id)
        .is_some_and(|element| element.set_pressed(false));
    if clicks {
        for path_id in build.path_to(anchor).into_iter().rev() {
            build.emit_event(UiEvent::widget(path_id, Event::click()));
        }
    }
    changed(was_changed || clicks)
}

fn move_pointer(
    state: &mut PointerPolicy,
    build: &mut Build,
    target: Option<WidgetId>,
    x: i32,
    y: i32,
) -> Dispatch {
    if state.pressed != target || target.is_none() {
        return Dispatch::CONTINUE;
    }
    let Some(id) = target else {
        return Dispatch::CONTINUE;
    };
    changed(place_caret(build, id, x, y, true))
}

fn hover(build: &mut Build, target: Option<WidgetId>, value: bool) -> Dispatch {
    let Some(id) = target else {
        return Dispatch::CONTINUE;
    };
    changed(
        build
            .element_for(id)
            .is_some_and(|element| element.set_hovered(value)),
    )
}

fn cancel(state: &mut PointerPolicy, build: &mut Build, target: Option<WidgetId>) -> Dispatch {
    let Some(id) = target else {
        return Dispatch::CONTINUE;
    };
    state.pressed = None;
    changed(
        build
            .element_for(id)
            .is_some_and(|element| element.set_pressed(false)),
    )
}

fn focus(build: &mut Build, id: WidgetId) {
    build.enter_modal_scope();
    let focusables = build.focusables();
    let target = build
        .path_to(id)
        .into_iter()
        .rev()
        .find(|candidate| focusables.contains(candidate));
    if target == build.focused() {
        return;
    }
    if let Some(current) = build.focused() {
        build.emit_event(UiEvent::widget(current, Event::blur()));
    }
    if let Some(next) = target {
        build.emit_event(UiEvent::widget(next, Event::focus()));
    }
    build.focus(target);
}

fn place_caret(build: &Build, id: WidgetId, x: i32, y: i32, extend: bool) -> bool {
    let Some(element) = build.element_for(id) else {
        return false;
    };
    let Some(position) = element.caret_at_pixel(x, y) else {
        return false;
    };
    element
        .with_edit(|edit| {
            edit.set_caret(position);
            if !extend {
                edit.set_anchor(position);
            }
        })
        .is_some()
}

fn changed(changed: bool) -> Dispatch {
    if changed {
        Dispatch::handled()
    } else {
        Dispatch::CONTINUE
    }
}
