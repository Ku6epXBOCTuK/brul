use super::frame::boundary_before;
use super::*;

impl WidgetTree {
    pub(super) fn copy_input(&self, slot: Slot) -> Vec<DispatchEvent> {
        self.selected_text(slot).map_or_else(Vec::new, |text| {
            vec![DispatchEvent::semantic(
                self.nodes[slot.index()].id,
                Event::copy(text),
            )]
        })
    }

    pub(super) fn cut_input(&mut self, slot: Slot) -> Vec<DispatchEvent> {
        let Some(text) = self.selected_text(slot) else {
            return Vec::new();
        };
        let changed = self.delete_input(slot, true);
        let mut events = self.input_event(slot, changed);
        events.push(DispatchEvent::semantic(
            self.nodes[slot.index()].id,
            Event::copy(text),
        ));
        events
    }

    pub fn paste_text(&mut self, text: &str) -> Vec<DispatchEvent> {
        let Some(slot) = self.focused_input() else {
            return Vec::new();
        };
        let id = self.nodes[slot.index()].id;
        let text = self.field_text(slot, text);
        let changed = self.insert_input(slot, &text);
        let mut events = vec![DispatchEvent::raw(id, RawEvent::paste(text))];
        events.extend(self.input_event(slot, changed));
        events
    }

    pub fn ime(&mut self, kind: ImeKind) -> Vec<DispatchEvent> {
        let mut events = match &kind {
            ImeKind::Preedit(text) => {
                self.set_preedit(text);
                Vec::new()
            }
            ImeKind::Commit(text) => self.commit_text(text),
            ImeKind::DeleteSurrounding {
                before_bytes,
                after_bytes,
            } => self.delete_surrounding(*before_bytes, *after_bytes),
            ImeKind::Enabled | ImeKind::Disabled => Vec::new(),
        };
        if let Some(id) = self.focused() {
            events.push(DispatchEvent::raw(id, RawEvent::ime(kind)));
        }
        events
    }

    pub fn set_preedit(&mut self, text: &str) -> bool {
        let Some(slot) = self.focused_input() else {
            return false;
        };
        let Content::Input { preedit, .. } = &mut self.nodes[slot.index()].data else {
            return false;
        };
        if preedit == text {
            return false;
        }
        preedit.clear();
        preedit.push_str(text);
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn commit_text(&mut self, text: &str) -> Vec<DispatchEvent> {
        let Some(slot) = self.focused_input() else {
            return Vec::new();
        };
        self.clear_preedit(slot);
        let text = self.field_text(slot, text);
        let changed = self.insert_input(slot, &text);
        self.input_event(slot, changed)
    }

    pub fn delete_surrounding(&mut self, before: usize, after: usize) -> Vec<DispatchEvent> {
        let Some(slot) = self.focused_input() else {
            return Vec::new();
        };
        if self.selection(slot).is_some() {
            let changed = self.delete_input(slot, true);
            return self.input_event(slot, changed);
        }
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return Vec::new();
        };
        let start = boundary_before(value, caret.saturating_sub(before));
        let end = boundary_before(value, (*caret + after).min(value.len()));
        if start >= end {
            return Vec::new();
        }
        value.replace_range(start..end, "");
        *caret = start;
        *anchor = start;
        self.stale.insert(slot);
        self.invalidate_layout();
        self.input_event(slot, true)
    }

    pub(super) fn clear_preedit(&mut self, slot: Slot) -> bool {
        let Content::Input { preedit, .. } = &mut self.nodes[slot.index()].data else {
            return false;
        };
        if preedit.is_empty() {
            return false;
        }
        preedit.clear();
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub(super) fn focused_input(&self) -> Option<Slot> {
        let slot = self.focused_slot()?;
        matches!(self.nodes[slot.index()].data, Content::Input { .. }).then_some(slot)
    }

    pub(super) fn field_text(&self, slot: Slot, text: &str) -> String {
        let multiline = matches!(
            self.nodes[slot.index()].data,
            Content::Input {
                multiline: true,
                ..
            }
        );
        if multiline {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.chars()
                .filter(|character| *character != '\r' && *character != '\n')
                .collect()
        }
    }

    pub(super) fn insert_input(&mut self, slot: Slot, text: &str) -> bool {
        if text.is_empty()
            || text
                .chars()
                .any(|character| character != '\n' && character.is_control())
        {
            return false;
        }
        let selection = self.selection(slot);
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        if let Some((start, end)) = selection {
            value.replace_range(start..end, "");
            *caret = start;
        }
        value.insert_str(*caret, text);
        *caret += text.len();
        *anchor = *caret;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub(super) fn delete_input(&mut self, slot: Slot, before: bool) -> bool {
        let selection = self.selection(slot);
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        match selection {
            Some((start, end)) => {
                value.replace_range(start..end, "");
                *caret = start;
            }
            None if before => {
                let Some((index, _)) = value[..*caret].char_indices().next_back() else {
                    return false;
                };
                value.remove(index);
                *caret = index;
            }
            None => {
                if *caret >= value.len() {
                    return false;
                }
                value.remove(*caret);
            }
        }
        *anchor = *caret;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn set_input_text(&mut self, id: WidgetId, text: String) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        if *value == text {
            return false;
        }
        *value = text;
        *caret = (*caret).min(value.len());
        while !value.is_char_boundary(*caret) {
            *caret -= 1;
        }
        *anchor = *caret;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }
}
