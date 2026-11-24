use crate::{
    Build, Dispatch, Event, RawEvent, RawUiEvent, UiEvent, ViewError,
    gui::{EventTarget, KeyModifiers, Position},
};

pub struct PlainEditor;

#[brul::attach(
    name = "editor",
    raw_events(Key, KeyNoop),
    raw_event = editor_raw
)]
pub fn editor(_build: &mut Build) -> Result<PlainEditor, ViewError> {
    Ok(PlainEditor)
}

enum Action {
    Insert(String),
    DeleteBackward,
    DeleteForward,
    Move { byte: usize, extend: bool },
    SelectAll,
    Copy,
    Cut,
    Paste,
    Submit,
    Pass,
}

struct Lines {
    start: usize,
    end: usize,
    above: Option<usize>,
    below: Option<usize>,
}

fn editor_raw(_state: &mut PlainEditor, build: &mut Build, event: &RawUiEvent) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    let (key, modifiers) = match &event.event {
        RawEvent::Key { key, modifiers } => (key, *modifiers),
        RawEvent::KeyNoop { .. } => return Dispatch::CONTINUE,
        _ => return Dispatch::CONTINUE,
    };
    let Some(id) = build.focused() else {
        return Dispatch::CONTINUE;
    };
    let target = event.target;

    let action = {
        let Some(element) = build.element_for(id) else {
            return Dispatch::CONTINUE;
        };
        if !element.is_input() {
            return Dispatch::CONTINUE;
        }
        let Some(buffer) = element.edit_buffer() else {
            return Dispatch::CONTINUE;
        };
        let byte = buffer.caret().offset;
        let lines = Lines {
            start: element.line_start(byte).unwrap_or(0),
            end: element.line_end(byte).unwrap_or(buffer.text().len()),
            above: element.caret_above(byte),
            below: element.caret_below(byte),
        };
        classify(&key.code, modifiers, &buffer, &lines, element.wrap())
    };

    dispatch(build, id, target, key.code.clone(), modifiers, action)
}

fn classify(
    code: &str,
    modifiers: KeyModifiers,
    buffer: &crate::EditBuffer,
    lines: &Lines,
    wrap: bool,
) -> Action {
    let byte = buffer.caret().offset;
    let text = buffer.text();

    if code == "Escape" || code == "Tab" {
        return Action::Pass;
    }
    if modifiers.control {
        return match code {
            "a" => Action::SelectAll,
            "c" => Action::Copy,
            "x" => Action::Cut,
            "v" => Action::Paste,
            _ => Action::Pass,
        };
    }
    if modifiers.alt || modifiers.meta {
        return Action::Pass;
    }

    match code {
        "Enter" => {
            if wrap {
                Action::Insert("\n".into())
            } else {
                Action::Submit
            }
        }
        "Backspace" => Action::DeleteBackward,
        "Delete" => Action::DeleteForward,
        "ArrowLeft" => Action::Move {
            byte: movement_left(text, byte, buffer),
            extend: modifiers.shift,
        },
        "ArrowRight" => Action::Move {
            byte: movement_right(text, byte, buffer),
            extend: modifiers.shift,
        },
        "Home" => Action::Move {
            byte: lines.start,
            extend: modifiers.shift,
        },
        "End" => Action::Move {
            byte: lines.end,
            extend: modifiers.shift,
        },
        "ArrowUp" if wrap => lines.above.map_or(Action::Pass, |byte| Action::Move {
            byte,
            extend: modifiers.shift,
        }),
        "ArrowDown" if wrap => lines.below.map_or(Action::Pass, |byte| Action::Move {
            byte,
            extend: modifiers.shift,
        }),
        _ if code.chars().count() == 1 && !code.chars().next().is_some_and(char::is_control) => {
            Action::Insert(code.into())
        }
        _ => Action::Pass,
    }
}

fn movement_left(text: &str, byte: usize, buffer: &crate::EditBuffer) -> usize {
    let selection = buffer.selection();
    if selection.start != selection.end {
        return selection.start.offset;
    }
    let mut at = byte.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    text[..at]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn movement_right(text: &str, byte: usize, buffer: &crate::EditBuffer) -> usize {
    let selection = buffer.selection();
    if selection.start != selection.end {
        return selection.end.offset;
    }
    let mut at = byte.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    text[at..]
        .char_indices()
        .nth(1)
        .map_or(text.len(), |(index, _)| at + index)
}

fn dispatch(
    build: &mut Build,
    id: crate::gui::WidgetId,
    target: EventTarget,
    code: String,
    modifiers: KeyModifiers,
    action: Action,
) -> Dispatch {
    match action {
        Action::Insert(text) => {
            let edited = with_build(build, id, |edit| {
                let (start, _) = replace_selection(edit);
                edit.insert_text(Position::new(0, start), &text);
                Some((edit.text().to_owned(), edit.caret().offset))
            });
            if let Some((value, caret)) = edited.flatten() {
                build.emit_event(UiEvent::at(target, Event::input(value, caret)));
                Dispatch::handled()
            } else {
                Dispatch::CONTINUE
            }
        }
        Action::DeleteBackward | Action::DeleteForward => {
            let backward = matches!(action, Action::DeleteBackward);
            let edited = with_build(build, id, |edit| {
                let (_, removed) = replace_selection(edit);
                let byte = edit.caret().offset;
                let before = backward && byte > 0;
                let after = !backward && byte < edit.text().len();
                if before {
                    edit.delete_surrounding(1, 0);
                } else if after {
                    edit.delete_surrounding(0, 1);
                }
                removed || before || after
            });
            if let Some(true) = edited {
                let value = with_build(build, id, |edit| {
                    Some((edit.text().to_owned(), edit.caret().offset))
                });
                if let Some((value, caret)) = value.flatten() {
                    build.emit_event(UiEvent::at(target, Event::input(value, caret)));
                }
                Dispatch::handled()
            } else {
                build.emit_raw_event(RawUiEvent::at(
                    target,
                    RawEvent::key_noop(crate::gui::KeyPress::from(code.as_str()), modifiers),
                ));
                Dispatch::handled()
            }
        }
        Action::Move { byte, extend } => {
            let edited = with_build(build, id, |edit| {
                let position = Position::new(0, byte);
                edit.set_caret(position);
                if !extend {
                    edit.set_anchor(position);
                }
                true
            });
            if edited.is_some() {
                Dispatch::handled()
            } else {
                Dispatch::CONTINUE
            }
        }
        Action::SelectAll => {
            let edited = with_build(build, id, |edit| {
                let end = edit.text().len();
                edit.set_anchor(Position::new(0, 0));
                edit.set_caret(Position::new(0, end));
                true
            });
            if edited.is_some() {
                Dispatch::handled()
            } else {
                Dispatch::CONTINUE
            }
        }
        Action::Copy => {
            let value = build
                .element_for(id)
                .and_then(|element| element.edit_buffer())
                .and_then(|buffer| buffer.selected_text().map(str::to_owned));
            if let Some(text) = value {
                build.emit_event(UiEvent::at(target, Event::copy(text)));
            }
            Dispatch::handled()
        }
        Action::Cut => {
            let value = build
                .element_for(id)
                .and_then(|element| element.edit_buffer())
                .and_then(|buffer| buffer.selected_text().map(str::to_owned));
            let edited = with_build(build, id, |edit| {
                let (start, _) = replace_selection(edit);
                Some((edit.text().to_owned(), edit.caret().offset.max(start)))
            });
            if let (Some(text), Some((value, caret))) = (value, edited.flatten()) {
                build.emit_event(UiEvent::at(target, Event::input(value, caret)));
                build.emit_event(UiEvent::at(target, Event::copy(text)));
            }
            Dispatch::handled()
        }
        Action::Paste => {
            build.emit_raw_event(RawUiEvent::at(target, RawEvent::paste(String::new())));
            Dispatch::handled()
        }
        Action::Submit => {
            build.emit_event(UiEvent::at(target, Event::submit()));
            Dispatch::handled()
        }
        Action::Pass => Dispatch::CONTINUE,
    }
}

fn with_build<T>(
    build: &Build,
    id: crate::gui::WidgetId,
    edit: impl FnOnce(&mut crate::EditBuffer) -> T,
) -> Option<T> {
    build.element_for(id)?.with_edit(edit)
}

fn replace_selection(edit: &mut crate::EditBuffer) -> (usize, bool) {
    let selection = edit.selection();
    if selection.start == selection.end {
        return (edit.caret().offset, false);
    }
    let start = selection.start.offset;
    edit.remove(selection);
    edit.set_caret(Position::new(0, start));
    edit.set_anchor(Position::new(0, start));
    (start, true)
}
