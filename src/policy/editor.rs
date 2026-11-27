use crate::{
    CapabilityContext, Dispatch, Event, RawEvent, RawUiEvent, UiEvent, ViewError,
    gui::{EventTarget, ImeKind, KeyCode, KeyModifiers, NamedKey, Position},
};

pub struct PlainEditor;

#[brul::attach(
    name = "editor",
    raw_events(Key, Ime, Paste),
    raw_event = editor_raw
)]
pub fn editor(_build: &CapabilityContext) -> Result<PlainEditor, ViewError> {
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

fn editor_raw(_state: &mut PlainEditor, build: &CapabilityContext, event: &RawUiEvent) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    match &event.event {
        RawEvent::Key { key, modifiers } => editor_key(build, event.target, key, *modifiers),
        RawEvent::KeyNoop { .. } => Dispatch::CONTINUE,
        RawEvent::Paste { text } => editor_paste(build, event.target, text),
        RawEvent::Ime { kind } => editor_ime(build, event.target, kind),
        _ => Dispatch::CONTINUE,
    }
}

fn editor_key(
    build: &CapabilityContext,
    target: EventTarget,
    key: &crate::gui::KeyPress,
    modifiers: KeyModifiers,
) -> Dispatch {
    let Some(id) = build.focused() else {
        return Dispatch::CONTINUE;
    };
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

    dispatch(build, id, target, key, modifiers, action)
}

fn focused_input(build: &CapabilityContext) -> Option<(crate::gui::WidgetId, bool)> {
    let id = build.focused()?;
    let element = build.element_for(id)?;
    element.is_input().then_some((id, element.wrap()))
}

fn normalize_paste(text: &str, single_line: bool) -> String {
    if single_line {
        text.chars()
            .filter(|character| !character.is_control())
            .collect()
    } else {
        text.replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|character| *character == '\n' || !character.is_control())
            .collect()
    }
}

enum ImeAction {
    Preedit(String),
    Commit(String),
    DeleteSurrounding { before: usize, after: usize },
    Pass,
}

fn ime_action(kind: &ImeKind, single_line: bool) -> ImeAction {
    match kind {
        ImeKind::Enabled | ImeKind::Disabled => ImeAction::Pass,
        ImeKind::Preedit(text) => ImeAction::Preedit(text.clone()),
        ImeKind::Commit(text) => ImeAction::Commit(normalize_paste(text, single_line)),
        ImeKind::DeleteSurrounding {
            before_bytes,
            after_bytes,
        } => ImeAction::DeleteSurrounding {
            before: *before_bytes,
            after: *after_bytes,
        },
    }
}

fn editor_paste(build: &CapabilityContext, target: EventTarget, text: &str) -> Dispatch {
    if text.is_empty() {
        return Dispatch::CONTINUE;
    }
    let Some((id, wrap)) = focused_input(build) else {
        return Dispatch::CONTINUE;
    };
    let text = normalize_paste(text, !wrap);
    if text.is_empty() {
        return Dispatch::CONTINUE;
    }

    let edited = with_build(build, id, |edit| {
        let (start, _) = replace_selection(edit);
        edit.insert_text(Position::new(0, start), &text);
        Some((edit.text().to_owned(), edit.caret().offset))
    });
    if let Some(Some((value, caret))) = edited {
        build.emit_event(UiEvent::at(target, Event::input(value, caret)));
        Dispatch::handled()
    } else {
        Dispatch::CONTINUE
    }
}

fn editor_ime(build: &CapabilityContext, target: EventTarget, kind: &ImeKind) -> Dispatch {
    let Some((id, wrap)) = focused_input(build) else {
        return Dispatch::CONTINUE;
    };
    match ime_action(kind, !wrap) {
        ImeAction::Pass => Dispatch::CONTINUE,
        ImeAction::Preedit(text) => {
            let edited = with_build(build, id, |edit| {
                let changed = edit.preedit() != text;
                edit.set_preedit(text);
                changed
            });
            if matches!(edited, Some(true)) {
                Dispatch::handled()
            } else {
                Dispatch::CONTINUE
            }
        }
        ImeAction::Commit(text) => {
            let edited = with_build(build, id, |edit| {
                let had_preedit = !edit.preedit().is_empty();
                edit.set_preedit("");
                let before = edit.text().to_owned();
                if !text.is_empty() {
                    let (start, _) = replace_selection(edit);
                    edit.insert_text(Position::new(0, start), &text);
                }
                let after = edit.text().to_owned();
                let caret = edit.caret().offset;
                let changed = after != before;
                (changed || had_preedit, (changed).then_some((after, caret)))
            });
            let Some((changed, value)) = edited else {
                return Dispatch::CONTINUE;
            };
            if let Some((value, caret)) = value {
                build.emit_event(UiEvent::at(target, Event::input(value, caret)));
            }
            if changed {
                Dispatch::handled()
            } else {
                Dispatch::CONTINUE
            }
        }
        ImeAction::DeleteSurrounding { before, after } => {
            let edited = with_build(build, id, |edit| {
                let before_text = edit.text().to_owned();
                let (_, removed) = replace_selection(edit);
                if !removed {
                    edit.delete_surrounding(before, after);
                }
                let value = edit.text().to_owned();
                let caret = edit.caret().offset;
                (removed || value != before_text, value, caret)
            });
            let Some((true, value, caret)) = edited else {
                return Dispatch::CONTINUE;
            };
            build.emit_event(UiEvent::at(target, Event::input(value, caret)));
            Dispatch::handled()
        }
    }
}

fn classify(
    code: &KeyCode,
    modifiers: KeyModifiers,
    buffer: &crate::EditBuffer,
    lines: &Lines,
    wrap: bool,
) -> Action {
    let byte = buffer.caret().offset;
    let text = buffer.text();

    if matches!(code, KeyCode::Named(NamedKey::Escape | NamedKey::Tab)) {
        return Action::Pass;
    }
    if modifiers.control {
        return match code {
            KeyCode::Character(text) => match text.as_str() {
                "a" => Action::SelectAll,
                "c" => Action::Copy,
                "x" => Action::Cut,
                "v" => Action::Paste,
                _ => Action::Pass,
            },
            _ => Action::Pass,
        };
    }
    if modifiers.alt || modifiers.meta {
        return Action::Pass;
    }

    match code {
        KeyCode::Named(NamedKey::Enter) => {
            if wrap {
                Action::Insert("\n".into())
            } else {
                Action::Submit
            }
        }
        KeyCode::Named(NamedKey::Backspace) => Action::DeleteBackward,
        KeyCode::Named(NamedKey::Delete) => Action::DeleteForward,
        KeyCode::Named(NamedKey::ArrowLeft) => Action::Move {
            byte: movement_left(text, byte, buffer),
            extend: modifiers.shift,
        },
        KeyCode::Named(NamedKey::ArrowRight) => Action::Move {
            byte: movement_right(text, byte, buffer),
            extend: modifiers.shift,
        },
        KeyCode::Named(NamedKey::Home) => Action::Move {
            byte: lines.start,
            extend: modifiers.shift,
        },
        KeyCode::Named(NamedKey::End) => Action::Move {
            byte: lines.end,
            extend: modifiers.shift,
        },
        KeyCode::Named(NamedKey::ArrowUp) if wrap => {
            lines.above.map_or(Action::Pass, |byte| Action::Move {
                byte,
                extend: modifiers.shift,
            })
        }
        KeyCode::Named(NamedKey::ArrowDown) if wrap => {
            lines.below.map_or(Action::Pass, |byte| Action::Move {
                byte,
                extend: modifiers.shift,
            })
        }
        KeyCode::Character(text)
            if text.chars().count() == 1 && !text.chars().next().is_some_and(char::is_control) =>
        {
            Action::Insert(text.clone())
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
    build: &CapabilityContext,
    id: crate::gui::WidgetId,
    target: EventTarget,
    key: &crate::gui::KeyPress,
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
                    RawEvent::key_noop(key.clone(), modifiers),
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
    build: &CapabilityContext,
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
