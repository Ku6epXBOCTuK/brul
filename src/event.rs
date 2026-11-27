use brul_gui::{Event, EventTarget, KeyCode, UiEvent};

use crate::{ComponentHandle, Dyn, EventCall};

pub(crate) fn call(component: &ComponentHandle, function: EventCall, arguments: Vec<Dyn>) {
    tracing::debug!(arguments = arguments.len(), "calling script function");
    if let Err(error) = function(&mut component.borrow_mut(), &arguments) {
        tracing::error!(%error, "script function failed");
    }
}

pub(crate) fn arguments(event: &UiEvent) -> Vec<Dyn> {
    let mut object = match &event.event {
        Event::Key { key, modifiers } => {
            let code = match &key.code {
                KeyCode::Character(text) => Dyn::text(text.clone()),
                KeyCode::Named(named) => Dyn::text(named.to_string()),
            };
            let physical = key
                .physical
                .map_or_else(Dyn::null, |code| Dyn::text(code.to_string()));
            vec![
                ("code".to_owned(), code),
                ("physical".to_owned(), physical),
                ("ctrl".to_owned(), Dyn::flag(modifiers.control)),
                ("shift".to_owned(), Dyn::flag(modifiers.shift)),
                ("alt".to_owned(), Dyn::flag(modifiers.alt)),
                ("meta".to_owned(), Dyn::flag(modifiers.meta)),
            ]
        }
        Event::Input { value, caret } => vec![
            ("value".to_owned(), Dyn::text(value.as_str())),
            ("caret".to_owned(), Dyn::num(*caret as f64)),
        ],
        _ => return Vec::new(),
    };
    object.push(("target".to_owned(), target_value(&event.target)));
    vec![Dyn::object(object)]
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
            vec![Dyn::object(vec![
                ("value".to_owned(), Dyn::text("xy")),
                ("caret".to_owned(), Dyn::num(2.0)),
                ("target".to_owned(), target(id)),
            ])]
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
            vec![Dyn::object(vec![
                ("code".to_owned(), Dyn::text("Enter")),
                ("physical".to_owned(), Dyn::text("KeyQ")),
                ("ctrl".to_owned(), Dyn::flag(true)),
                ("shift".to_owned(), Dyn::flag(false)),
                ("alt".to_owned(), Dyn::flag(false)),
                ("meta".to_owned(), Dyn::flag(false)),
                ("target".to_owned(), Dyn::null()),
            ])]
        );
    }

    #[test]
    fn a_logical_only_key_event_has_no_physical_code() {
        let event = Event::key(KeyPress::from("a"), KeyModifiers::default());
        assert_eq!(
            arguments(&UiEvent::window(event)),
            vec![Dyn::object(vec![
                ("code".to_owned(), Dyn::text("a")),
                ("physical".to_owned(), Dyn::null()),
                ("ctrl".to_owned(), Dyn::flag(false)),
                ("shift".to_owned(), Dyn::flag(false)),
                ("alt".to_owned(), Dyn::flag(false)),
                ("meta".to_owned(), Dyn::flag(false)),
                ("target".to_owned(), Dyn::null()),
            ])]
        );
    }

    #[test]
    fn a_function_key_keeps_its_script_string() {
        let event = Event::key(KeyPress::from("F1"), KeyModifiers::default());
        let arguments = arguments(&UiEvent::window(event));
        assert_eq!(arguments[0].field("code").unwrap(), Dyn::text("F1"));
    }
}
