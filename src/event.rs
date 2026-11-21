use brul_gui::{WidgetEvent, WidgetId};

use crate::{ComponentHandle, Dyn, EventCall, HandlerEvent};
pub(crate) fn call(component: &ComponentHandle, function: EventCall, arguments: Vec<Dyn>) {
    tracing::debug!(arguments = arguments.len(), "calling script function");
    if let Err(error) = function(&mut component.borrow_mut(), &arguments) {
        tracing::error!(%error, "script function failed");
    }
}

pub(crate) fn arguments(event: &WidgetEvent) -> Vec<Dyn> {
    match event {
        WidgetEvent::Key { key, modifiers } => vec![Dyn::object(vec![
            ("code".to_owned(), Dyn::text(key.code.as_str())),
            (
                "physical".to_owned(),
                key.physical.as_deref().map_or_else(Dyn::null, Dyn::text),
            ),
            ("ctrl".to_owned(), Dyn::flag(modifiers.control)),
            ("shift".to_owned(), Dyn::flag(modifiers.shift)),
            ("alt".to_owned(), Dyn::flag(modifiers.alt)),
            ("meta".to_owned(), Dyn::flag(modifiers.meta)),
        ])],
        WidgetEvent::Input(_, value, caret) => vec![Dyn::object(vec![
            ("value".to_owned(), Dyn::text(value.as_str())),
            ("caret".to_owned(), Dyn::num(*caret as f64)),
        ])],
        _ => Vec::new(),
    }
}

pub(crate) fn handler_target(event: &WidgetEvent) -> Option<(Option<WidgetId>, HandlerEvent)> {
    match event {
        WidgetEvent::Click(id) => Some((Some(*id), HandlerEvent::Click)),
        WidgetEvent::Press(id) => Some((Some(*id), HandlerEvent::Press)),
        WidgetEvent::Release(id) => Some((Some(*id), HandlerEvent::Release)),
        WidgetEvent::PointerEnter(id) => Some((Some(*id), HandlerEvent::Enter)),
        WidgetEvent::PointerLeave(id) => Some((Some(*id), HandlerEvent::Leave)),
        WidgetEvent::Focus(id) => Some((Some(*id), HandlerEvent::Focus)),
        WidgetEvent::Blur(id) => Some((Some(*id), HandlerEvent::Blur)),
        WidgetEvent::Input(id, _, _) => Some((Some(*id), HandlerEvent::Input)),
        WidgetEvent::Submit(id) => Some((Some(*id), HandlerEvent::Submit)),
        WidgetEvent::Dismiss(id) => Some((Some(*id), HandlerEvent::Dismiss)),
        WidgetEvent::Key { .. } => Some((None, HandlerEvent::Key)),
        WidgetEvent::Close => Some((None, HandlerEvent::Close)),
        WidgetEvent::Drag(_) => None,
        WidgetEvent::Timer(..) => None,
        WidgetEvent::Copy(_) => None,
        WidgetEvent::Paste => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use brul_gui::KeyModifiers;
    use brul_gui::KeyPress;

    use brul_gui::Widget;

    #[test]
    fn handler_target_names_the_widget_of_a_pointer_event() {
        let id = Widget::text("x").id();
        let cases = [
            (WidgetEvent::Click(id), HandlerEvent::Click),
            (WidgetEvent::Press(id), HandlerEvent::Press),
            (WidgetEvent::Release(id), HandlerEvent::Release),
            (WidgetEvent::PointerEnter(id), HandlerEvent::Enter),
            (WidgetEvent::PointerLeave(id), HandlerEvent::Leave),
            (WidgetEvent::Focus(id), HandlerEvent::Focus),
            (WidgetEvent::Blur(id), HandlerEvent::Blur),
            (
                WidgetEvent::Input(id, "x".to_owned(), 1),
                HandlerEvent::Input,
            ),
            (WidgetEvent::Submit(id), HandlerEvent::Submit),
            (WidgetEvent::Dismiss(id), HandlerEvent::Dismiss),
        ];
        for (event, expected) in cases {
            assert_eq!(handler_target(&event), Some((Some(id), expected)));
        }
    }

    #[test]
    fn an_input_event_carries_the_value_and_the_caret() {
        let id = Widget::input("x").id();
        let arguments = arguments(&WidgetEvent::Input(id, "xy".to_owned(), 2));
        assert_eq!(
            arguments,
            vec![Dyn::object(vec![
                ("value".to_owned(), Dyn::text("xy")),
                ("caret".to_owned(), Dyn::num(2.0)),
            ])]
        );
    }

    #[test]
    fn a_key_event_carries_the_modifiers() {
        let event = WidgetEvent::Key {
            key: KeyPress {
                code: "Enter".to_owned(),
                physical: Some("KeyQ".to_owned()),
            },
            modifiers: KeyModifiers {
                control: true,
                ..KeyModifiers::default()
            },
        };
        assert_eq!(
            arguments(&event),
            vec![Dyn::object(vec![
                ("code".to_owned(), Dyn::text("Enter")),
                ("physical".to_owned(), Dyn::text("KeyQ")),
                ("ctrl".to_owned(), Dyn::flag(true)),
                ("shift".to_owned(), Dyn::flag(false)),
                ("alt".to_owned(), Dyn::flag(false)),
                ("meta".to_owned(), Dyn::flag(false)),
            ])]
        );
    }

    #[test]
    fn a_logical_only_key_event_has_no_physical_code() {
        let event = WidgetEvent::Key {
            key: KeyPress::from("a"),
            modifiers: KeyModifiers::default(),
        };
        assert_eq!(
            arguments(&event),
            vec![Dyn::object(vec![
                ("code".to_owned(), Dyn::text("a")),
                ("physical".to_owned(), Dyn::null()),
                ("ctrl".to_owned(), Dyn::flag(false)),
                ("shift".to_owned(), Dyn::flag(false)),
                ("alt".to_owned(), Dyn::flag(false)),
                ("meta".to_owned(), Dyn::flag(false)),
            ])]
        );
    }

    #[test]
    fn handler_target_names_window_events_without_a_widget() {
        assert_eq!(
            handler_target(&WidgetEvent::Key {
                key: KeyPress::from("Escape"),
                modifiers: KeyModifiers::default(),
            }),
            Some((None, HandlerEvent::Key))
        );
        assert_eq!(
            handler_target(&WidgetEvent::Close),
            Some((None, HandlerEvent::Close))
        );
        let id = Widget::text("x").id();
        assert_eq!(handler_target(&WidgetEvent::Drag(id)), None);
        assert_eq!(
            handler_target(&WidgetEvent::Timer(
                brul_gui::TimerOwner::new(0),
                "tick".into()
            )),
            None
        );
    }
}
