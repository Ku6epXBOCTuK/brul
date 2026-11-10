use winit::event::{ButtonSource, ElementState, MouseButton, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::window::Theme;

use crate::window::ColorScheme;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Input {
    Exit,
    Key { code: String, exit: bool },
    Moved(f64, f64),
    Left,
    Down(f64, f64),
    Up(f64, f64),
    Theme(ColorScheme),
    Surface,
    Redraw,
}

const NAMED: &[(NamedKey, &str)] = &[
    (NamedKey::Escape, "Escape"),
    (NamedKey::Enter, "Enter"),
    (NamedKey::Tab, "Tab"),
    (NamedKey::Backspace, "Backspace"),
    (NamedKey::Delete, "Delete"),
    (NamedKey::ArrowUp, "ArrowUp"),
    (NamedKey::ArrowDown, "ArrowDown"),
    (NamedKey::ArrowLeft, "ArrowLeft"),
    (NamedKey::ArrowRight, "ArrowRight"),
];

pub(crate) fn key_code(key: &Key) -> Option<String> {
    match key {
        Key::Named(named) => NAMED
            .iter()
            .find(|(value, _)| value == named)
            .map(|(_, code)| (*code).to_owned()),
        Key::Character(text) => Some(text.to_string()),
        _ => None,
    }
}

pub(crate) fn scheme(theme: &Theme) -> ColorScheme {
    match theme {
        Theme::Light => ColorScheme::Light,
        _ => ColorScheme::Dark,
    }
}

pub(crate) fn system_scheme(theme: Option<&Theme>) -> ColorScheme {
    theme.map_or(ColorScheme::Dark, scheme)
}

pub(crate) fn point(x: f64, y: f64) -> (i32, i32) {
    (x.floor() as i32, y.floor() as i32)
}

pub(crate) fn window_input(event: &WindowEvent) -> Option<Input> {
    match event {
        WindowEvent::CloseRequested => Some(Input::Exit),
        WindowEvent::ThemeChanged(theme) => Some(Input::Theme(scheme(theme))),
        WindowEvent::SurfaceResized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            Some(Input::Surface)
        }
        WindowEvent::RedrawRequested => Some(Input::Redraw),
        WindowEvent::Occluded(false) => Some(Input::Redraw),
        _ => None,
    }
}

pub(crate) fn event_input(event: &WindowEvent) -> Option<Input> {
    match event {
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
            let code = key_code(&event.logical_key)?;
            Some(Input::Key {
                code,
                exit: event.logical_key == Key::Named(NamedKey::Escape),
            })
        }
        WindowEvent::PointerMoved { position, .. } => Some(Input::Moved(position.x, position.y)),
        WindowEvent::PointerLeft { .. } => Some(Input::Left),
        WindowEvent::PointerButton {
            state,
            position,
            primary: true,
            button,
            is_macos_activation_click: false,
            ..
        } if is_left(button) => {
            let (x, y) = (position.x, position.y);
            match state {
                ElementState::Pressed => Some(Input::Down(x, y)),
                ElementState::Released => Some(Input::Up(x, y)),
            }
        }
        _ => None,
    }
}

fn is_left(button: &ButtonSource) -> bool {
    matches!(
        button,
        ButtonSource::Mouse(MouseButton::Left) | ButtonSource::Touch { .. }
    )
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;
    use winit::event::{
        ButtonSource, ElementState, KeyEvent, MouseButton, PointerKind, PointerSource, WindowEvent,
    };
    use winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};
    use winit::window::Theme;

    use super::*;

    fn key_event(logical: Key, state: ElementState) -> WindowEvent {
        WindowEvent::KeyboardInput {
            device_id: None,
            event: KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                logical_key: logical.clone(),
                text: None,
                location: KeyLocation::Standard,
                state,
                repeat: false,
                text_with_all_modifiers: None,
                key_without_modifiers: logical,
            },
            is_synthetic: false,
        }
    }

    fn button_event(
        state: ElementState,
        button: MouseButton,
        primary: bool,
        is_macos_activation_click: bool,
    ) -> WindowEvent {
        WindowEvent::PointerButton {
            device_id: None,
            state,
            position: PhysicalPosition::new(3.0, 4.0),
            primary,
            button: ButtonSource::Mouse(button),
            is_macos_activation_click,
        }
    }

    #[test]
    fn every_named_key_has_its_own_code() {
        let mut codes: Vec<&str> = NAMED.iter().map(|(_, code)| *code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), NAMED.len());

        for (named, code) in NAMED {
            let key = Key::Named(*named);
            assert_eq!(key_code(&key).as_deref(), Some(*code));
        }
    }

    #[test]
    fn a_character_key_gives_its_text() {
        let key = Key::Character("\u{439}".into());
        assert_eq!(key_code(&key).as_deref(), Some("\u{439}"));
    }

    #[test]
    fn a_key_outside_the_table_has_no_code() {
        assert_eq!(key_code(&Key::Named(NamedKey::F1)), None);
    }

    #[test]
    fn the_two_themes_map_to_their_schemes() {
        assert_eq!(scheme(&Theme::Light), ColorScheme::Light);
        assert_eq!(scheme(&Theme::Dark), ColorScheme::Dark);
    }

    #[test]
    fn a_missing_system_theme_is_dark() {
        assert_eq!(system_scheme(None), ColorScheme::Dark);
        assert_eq!(system_scheme(Some(&Theme::Light)), ColorScheme::Light);
        assert_eq!(system_scheme(Some(&Theme::Dark)), ColorScheme::Dark);
    }

    #[test]
    fn a_point_is_floored_to_pixels() {
        assert_eq!(point(1.9, 2.1), (1, 2));
        assert_eq!(point(-0.5, -1.1), (-1, -2));
    }

    #[test]
    fn window_events_become_window_input() {
        assert_eq!(
            window_input(&WindowEvent::CloseRequested),
            Some(Input::Exit)
        );
        assert_eq!(
            window_input(&WindowEvent::ThemeChanged(Theme::Light)),
            Some(Input::Theme(ColorScheme::Light))
        );
        assert_eq!(
            window_input(&WindowEvent::SurfaceResized(winit::dpi::PhysicalSize::new(
                4, 5
            ))),
            Some(Input::Surface)
        );
        assert_eq!(
            window_input(&WindowEvent::RedrawRequested),
            Some(Input::Redraw)
        );
        assert_eq!(
            window_input(&WindowEvent::Occluded(false)),
            Some(Input::Redraw)
        );
        assert_eq!(window_input(&WindowEvent::Occluded(true)), None);
        assert_eq!(window_input(&WindowEvent::Focused(true)), None);
    }

    #[test]
    fn escape_is_the_key_that_leaves() {
        assert_eq!(
            event_input(&key_event(
                Key::Named(NamedKey::Escape),
                ElementState::Pressed
            )),
            Some(Input::Key {
                code: "Escape".to_owned(),
                exit: true
            })
        );
        assert_eq!(
            event_input(&key_event(
                Key::Character("a".into()),
                ElementState::Pressed
            )),
            Some(Input::Key {
                code: "a".to_owned(),
                exit: false
            })
        );
    }

    #[test]
    fn a_released_key_is_ignored() {
        let event = key_event(Key::Named(NamedKey::Escape), ElementState::Released);
        assert_eq!(event_input(&event), None);
    }

    #[test]
    fn the_pointer_moves_and_leaves() {
        let moved = WindowEvent::PointerMoved {
            device_id: None,
            position: PhysicalPosition::new(7.5, 8.25),
            primary: true,
            source: PointerSource::Mouse,
        };
        assert_eq!(event_input(&moved), Some(Input::Moved(7.5, 8.25)));

        let left = WindowEvent::PointerLeft {
            device_id: None,
            position: Some(PhysicalPosition::new(7.5, 8.25)),
            primary: true,
            kind: PointerKind::Mouse,
        };
        assert_eq!(event_input(&left), Some(Input::Left));
    }

    #[test]
    fn only_the_left_button_becomes_a_press() {
        assert_eq!(
            event_input(&button_event(
                ElementState::Pressed,
                MouseButton::Left,
                true,
                false
            )),
            Some(Input::Down(3.0, 4.0))
        );
        assert_eq!(
            event_input(&button_event(
                ElementState::Released,
                MouseButton::Left,
                true,
                false
            )),
            Some(Input::Up(3.0, 4.0))
        );
        assert_eq!(
            event_input(&button_event(
                ElementState::Pressed,
                MouseButton::Right,
                true,
                false
            )),
            None
        );
        assert_eq!(
            event_input(&button_event(
                ElementState::Pressed,
                MouseButton::Left,
                false,
                false
            )),
            None
        );
        assert_eq!(
            event_input(&button_event(
                ElementState::Pressed,
                MouseButton::Left,
                true,
                true
            )),
            None
        );
    }
}
