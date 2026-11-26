use std::hash::{DefaultHasher, Hash, Hasher};
use std::process::ExitCode;

use brul::gui::WidgetId;
use brul::{Build, Dispatch, Event, Position, RawEvent, RawUiEvent, UiEvent, Value, ViewError};

mod generated;

#[derive(Clone, Copy)]
enum Mode {
    Mask,
    Reverse,
    Digest,
}

impl std::str::FromStr for Mode {
    type Err = ViewError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "mask" => Ok(Self::Mask),
            "reverse" => Ok(Self::Reverse),
            "digest" => Ok(Self::Digest),
            other => Err(ViewError::Message(format!("unknown input mode '{other}'"))),
        }
    }
}

fn byte_at_char(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

fn char_at_byte(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].chars().count()
}

fn clamp(text: &str, mut byte: usize) -> usize {
    byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

impl Mode {
    fn display(self, raw: &str) -> String {
        match self {
            Self::Mask => raw.chars().map(|_| '\u{2022}').collect(),
            Self::Reverse => raw.chars().rev().collect(),
            Self::Digest => {
                let mut hasher = DefaultHasher::new();
                raw.hash(&mut hasher);
                format!("{:016x}", hasher.finish())
            }
        }
    }

    fn display_caret(self, raw: &str, caret: usize) -> usize {
        let index = char_at_byte(raw, clamp(raw, caret));
        let display = self.display(raw);
        let target = match self {
            Self::Mask => index,
            Self::Reverse => display.chars().count() - index,
            Self::Digest => display.chars().count(),
        };
        byte_at_char(&display, target)
    }

    fn raw_caret(self, raw: &str, display_byte: usize) -> usize {
        let display = self.display(raw);
        let index = char_at_byte(&display, clamp(&display, display_byte));
        let target = match self {
            Self::Mask => index,
            Self::Reverse => display.chars().count() - index,
            Self::Digest => raw.chars().count(),
        };
        byte_at_char(raw, target)
    }
}

pub struct Editor {
    raw: String,
    caret: usize,
    mode: Mode,
}

fn repaint(state: &Editor, build: &Build) {
    let element = build.element();
    element.set_input(state.mode.display(&state.raw));
    let caret = state.mode.display_caret(&state.raw, state.caret);
    element.with_edit(|buffer| {
        let position = Position::new(0, caret);
        buffer.set_caret(position);
        buffer.set_anchor(position);
    });
}

#[brul::attach(
    name = "masked_editor",
    raw_events(Key, Paste),
    update = editor_update,
    raw_event = editor_raw
)]
pub fn masked_editor(_ctx: &mut Build, value: String, mode: String) -> Result<Editor, ViewError> {
    Ok(Editor {
        raw: value,
        caret: 0,
        mode: mode.parse()?,
    })
}

fn editor_update(
    state: &mut Editor,
    build: &mut Build,
    value: String,
    mode: String,
) -> Result<(), ViewError> {
    state.mode = mode.parse()?;
    if state.raw != value {
        state.raw = value;
        state.caret = state.raw.len();
    } else {
        state.caret = clamp(&state.raw, state.caret);
    }
    repaint(state, build);
    Ok(())
}

enum Action {
    Insert(String),
    DeleteBackward,
    DeleteForward,
    MoveLeft,
    MoveRight,
    Home,
    End,
    Pass,
}

fn classify(event: &RawUiEvent) -> Action {
    match &event.event {
        RawEvent::Key { key, modifiers } => {
            if modifiers.control || modifiers.alt || modifiers.meta {
                return Action::Pass;
            }
            match key.code.as_str() {
                "Backspace" => Action::DeleteBackward,
                "Delete" => Action::DeleteForward,
                "ArrowLeft" => Action::MoveLeft,
                "ArrowRight" => Action::MoveRight,
                "Home" => Action::Home,
                "End" => Action::End,
                code if code.chars().count() == 1
                    && !code.chars().next().is_some_and(char::is_control) =>
                {
                    Action::Insert(code.to_owned())
                }
                _ => Action::Pass,
            }
        }
        RawEvent::Paste { text } => {
            let text: String = text
                .chars()
                .filter(|character| !character.is_control())
                .collect();
            if text.is_empty() {
                Action::Pass
            } else {
                Action::Insert(text)
            }
        }
        _ => Action::Pass,
    }
}

fn editor_raw(state: &mut Editor, build: &mut Build, event: &RawUiEvent) -> Dispatch {
    if build.handled() || build.focused() != Some(build.element().id()) {
        return Dispatch::CONTINUE;
    }
    let action = classify(event);
    if matches!(action, Action::Pass) {
        return Dispatch::CONTINUE;
    }

    let display_byte = build.element().caret().map_or(0, |caret| caret.offset);
    state.caret = state.mode.raw_caret(&state.raw, display_byte);
    match action {
        Action::Insert(text) => {
            state.raw.insert_str(state.caret, &text);
            state.caret += text.len();
        }
        Action::DeleteBackward => {
            if let Some((byte, _)) = state.raw[..state.caret].char_indices().next_back() {
                state.raw.remove(byte);
                state.caret = byte;
            }
        }
        Action::DeleteForward => {
            if state.caret < state.raw.len() {
                state.raw.remove(state.caret);
            }
        }
        Action::MoveLeft => {
            state.caret = state.raw[..state.caret]
                .char_indices()
                .next_back()
                .map_or(0, |(byte, _)| byte);
        }
        Action::MoveRight => {
            state.caret = state.raw[state.caret..]
                .char_indices()
                .nth(1)
                .map_or(state.raw.len(), |(byte, _)| state.caret + byte);
        }
        Action::Home => state.caret = 0,
        Action::End => state.caret = state.raw.len(),
        Action::Pass => return Dispatch::CONTINUE,
    }

    repaint(state, build);
    build.emit_event(UiEvent::at(
        event.target,
        Event::input(state.raw.clone(), state.caret),
    ));
    Dispatch::handled()
}

pub struct Keys;

#[brul::attach(name = "custom_keys", raw_events(Key), raw_event = keys_raw)]
pub fn custom_keys(_ctx: &mut Build) -> Result<Keys, ViewError> {
    Ok(Keys)
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

fn keys_raw(_state: &mut Keys, build: &mut Build, event: &RawUiEvent) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    let (key, modifiers) = match &event.event {
        RawEvent::Key { key, modifiers } => (key, *modifiers),
        _ => return Dispatch::CONTINUE,
    };

    if key.code == "Tab" {
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

    if key.code == "Escape" {
        if let Some(current) = build.focused() {
            build.emit_event(UiEvent::widget(current, Event::blur()));
        }
        build.focus(None);
        build.emit_event(UiEvent::at(
            event.target,
            Event::key(key.clone(), modifiers),
        ));
        build.emit("escape", Vec::new());
        return Dispatch::stop();
    }

    Dispatch::CONTINUE
}

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("custom_text_input: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let seeds = [
        ("secret", Value::String(String::new().into())),
        ("mode", Value::String("mask".into())),
        ("caret", Value::String(String::new().into())),
        ("status", Value::String("mask".into())),
    ];
    let backend = match std::env::var("BRUL_EXAMPLE_BACKEND") {
        Ok(value) => Some(value.parse::<brul::Backend>()?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let mut windows = generated::windows();
    if let Some(backend) = backend {
        for window in &mut windows {
            window.config.backend = backend;
        }
    }
    brul::run_generated(
        windows,
        generated::script(),
        generated::names(),
        None,
        &seeds,
        None,
    )
}

#[cfg(test)]
#[path = "parts/main_tests.rs"]
mod tests;
