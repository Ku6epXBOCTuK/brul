use crate::{
    CapabilityContext, Dispatch, Event, RawEvent, RawUiEvent, UiEvent, ViewError,
    gui::{KeyCode, NamedKey, WidgetId},
};

pub struct KeyboardPolicy;

#[brul::attach(name = "keyboard", raw_events(Key), raw_event = keyboard_raw)]
pub fn keyboard(_ctx: &CapabilityContext) -> Result<KeyboardPolicy, ViewError> {
    Ok(KeyboardPolicy)
}

fn keyboard_raw(
    _state: &mut KeyboardPolicy,
    build: &CapabilityContext,
    event: &RawUiEvent,
) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    let (key, modifiers) = match &event.event {
        RawEvent::Key { key, modifiers } | RawEvent::KeyNoop { key, modifiers } => {
            (key, *modifiers)
        }
        _ => return Dispatch::CONTINUE,
    };

    if key.code == KeyCode::Named(NamedKey::Escape)
        && let Some(scope) = build.modal_scope()
    {
        build.emit_event(UiEvent::widget(scope, Event::dismiss()));
        return Dispatch::stop();
    }

    if key.code == KeyCode::Named(NamedKey::Tab) {
        build.enter_modal_scope();
        let focusables = build.focusables();
        let current = build.focused();
        let Some(next) = next_focus(&focusables, current, modifiers.shift) else {
            return Dispatch::stop();
        };
        if Some(next) == current {
            return Dispatch::stop();
        }
        if let Some(id) = current {
            build.emit_event(UiEvent::widget(id, Event::blur()));
        }
        build.emit_event(UiEvent::widget(next, Event::focus()));
        build.focus(Some(next));
        return Dispatch::stop();
    }

    let activate = key.code == KeyCode::Named(NamedKey::Enter)
        || matches!(&key.code, KeyCode::Character(text) if text == " ");
    if activate
        && let Some(focused) = build.focused()
        && build.focusables().contains(&focused)
        && build
            .element_for(focused)
            .is_some_and(|element| !element.is_input())
    {
        for id in build.path_to(focused).into_iter().rev() {
            build.emit_event(UiEvent::widget(id, Event::click()));
        }
        return Dispatch::stop();
    }

    build.emit_event(UiEvent::at(
        event.target,
        Event::key(key.clone(), modifiers),
    ));
    Dispatch::CONTINUE
}

fn next_focus(
    focusables: &[WidgetId],
    current: Option<WidgetId>,
    backwards: bool,
) -> Option<WidgetId> {
    if focusables.is_empty() {
        return None;
    }
    let step = if backwards { focusables.len() - 1 } else { 1 };
    match current.and_then(|id| focusables.iter().position(|candidate| *candidate == id)) {
        Some(index) => Some(focusables[(index + step) % focusables.len()]),
        None if backwards => focusables.last().copied(),
        None => focusables.first().copied(),
    }
}
