use brul_gui::{WidgetEvent, WidgetId};

use crate::{ComponentHandle, HandlerEvent, Value};
pub(crate) fn call(component: &ComponentHandle, function: &str, arguments: Vec<Value>) {
    if let Err(error) = component.borrow_mut().call_function(function, arguments) {
        eprintln!("{error}");
    }
}

pub(crate) fn arguments(event: &WidgetEvent) -> Vec<Value> {
    match event {
        WidgetEvent::Key(code) => vec![Value::Object(vec![(
            "code".to_owned(),
            Value::String(code.clone()),
        )])],
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
        WidgetEvent::Key(_) => Some((None, HandlerEvent::Key)),
        WidgetEvent::Drag(_) => None,
        WidgetEvent::Timer(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ];
        for (event, expected) in cases {
            assert_eq!(handler_target(&event), Some((Some(id), expected)));
        }
    }

    #[test]
    fn handler_target_leaves_out_the_widget_for_keys_and_ignores_drag_and_timer() {
        assert_eq!(
            handler_target(&WidgetEvent::Key("Escape".to_owned())),
            Some((None, HandlerEvent::Key))
        );
        let id = Widget::text("x").id();
        assert_eq!(handler_target(&WidgetEvent::Drag(id)), None);
        assert_eq!(handler_target(&WidgetEvent::Timer("tick".to_owned())), None);
    }
}
