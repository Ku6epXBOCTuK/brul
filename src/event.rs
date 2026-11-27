use brul_gui::{Event, EventTarget, KeyCode, UiEvent};

use crate::{ComponentHandle, Dyn, EventCall, Object};

pub(crate) fn call(component: &ComponentHandle, function: EventCall, arguments: Vec<Dyn>) {
    tracing::debug!(arguments = arguments.len(), "calling script function");
    if let Err(error) = function(&mut component.borrow_mut(), &arguments) {
        tracing::error!(%error, "script function failed");
    }
}

pub(crate) fn arguments(event: &UiEvent) -> Vec<Dyn> {
    let object = match &event.event {
        Event::Key { key, modifiers } => {
            let code = match &key.code {
                KeyCode::Character(text) => Dyn::text(text.clone()),
                KeyCode::Named(named) => Dyn::text(named.to_string()),
            };
            let physical = key
                .physical
                .map_or_else(Dyn::null, |code| Dyn::text(code.to_string()));
            Object::builder()
                .field("code", code)
                .field("physical", physical)
                .field("ctrl", Dyn::flag(modifiers.control))
                .field("shift", Dyn::flag(modifiers.shift))
                .field("alt", Dyn::flag(modifiers.alt))
                .field("meta", Dyn::flag(modifiers.meta))
        }
        Event::Input { value, caret } => Object::builder()
            .field("value", Dyn::text(value.as_str()))
            .field("caret", Dyn::num(*caret as f64)),
        _ => return Vec::new(),
    };
    vec![Dyn::object(
        object.field("target", target_value(&event.target)),
    )]
}

fn target_value(target: &EventTarget) -> Dyn {
    match target {
        EventTarget::Widget(id) => Dyn::num(id.get() as f64),
        EventTarget::Window => Dyn::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use brul_gui::{KeyCode, KeyModifiers, KeyPress, NamedKey, PhysicalKeyCode, Widget, WidgetId};

    fn target(id: WidgetId) -> Dyn {
        Dyn::num(id.get() as f64)
    }

    #[test]
    fn an_input_event_carries_the_value_and_the_caret() {
        let id = Widget::input("x").id();
        let arguments = arguments(&UiEvent::widget(id, Event::input("xy".to_owned(), 2)));
        assert_eq!(
            arguments,
            vec![Dyn::object(
                Object::builder()
                    .field("value", Dyn::text("xy"))
                    .field("caret", Dyn::num(2.0))
                    .field("target", target(id)),
            )]
        );
    }

    #[test]
    fn a_key_event_carries_the_modifiers() {
        let event = Event::key(
            KeyPress {
                code: KeyCode::Named(NamedKey::Enter),
                physical: Some(PhysicalKeyCode::KeyQ),
            },
            KeyModifiers {
                control: true,
                ..KeyModifiers::default()
            },
        );
        assert_eq!(
            arguments(&UiEvent::window(event)),
            vec![Dyn::object(
                Object::builder()
                    .field("code", Dyn::text("Enter"))
                    .field("physical", Dyn::text("KeyQ"))
                    .field("ctrl", Dyn::flag(true))
                    .field("shift", Dyn::flag(false))
                    .field("alt", Dyn::flag(false))
                    .field("meta", Dyn::flag(false))
                    .field("target", Dyn::null()),
            )]
        );
    }

    #[test]
    fn a_logical_only_key_event_has_no_physical_code() {
        let event = Event::key(KeyPress::from("a"), KeyModifiers::default());
        assert_eq!(
            arguments(&UiEvent::window(event)),
            vec![Dyn::object(
                Object::builder()
                    .field("code", Dyn::text("a"))
                    .field("physical", Dyn::null())
                    .field("ctrl", Dyn::flag(false))
                    .field("shift", Dyn::flag(false))
                    .field("alt", Dyn::flag(false))
                    .field("meta", Dyn::flag(false))
                    .field("target", Dyn::null()),
            )]
        );
    }

    #[test]
    fn a_function_key_keeps_its_script_string() {
        let event = Event::key(KeyPress::from("F1"), KeyModifiers::default());
        let arguments = arguments(&UiEvent::window(event));
        assert_eq!(arguments[0].field("code").unwrap(), Dyn::text("F1"));
    }
}
