use super::frame::{HitScratch, collect_hit};
use super::*;

impl WidgetTree {
    pub fn hit(&self, x: i32, y: i32) -> Result<Option<WidgetId>, WidgetError> {
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let mut scratch = HitScratch::default();
        collect_hit(&self.nodes, self.root, &geometry.boxes, &mut scratch, x, y);
        Ok(scratch.path.last().map(|slot| self.nodes[slot.index()].id))
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    pub fn set_modifiers(&mut self, modifiers: KeyModifiers) {
        self.modifiers = modifiers;
    }

    pub(super) fn key_event(&self, code: &str) -> WidgetEvent {
        WidgetEvent::Key {
            code: code.to_owned(),
            modifiers: self.modifiers,
        }
    }

    pub fn press_key(&mut self, code: &str) -> Vec<WidgetEvent> {
        if code == "Escape"
            && let Some(scope) = self.modal_scope()
        {
            return vec![WidgetEvent::Dismiss(self.nodes[scope.index()].id)];
        }
        if code == "Tab" {
            return if self.modifiers.shift {
                self.focus_previous()
            } else {
                self.focus_next()
            };
        }
        if matches!(code, "Enter" | " ")
            && let Some(slot) = self.focused_slot().filter(|slot| {
                let node = &self.nodes[slot.index()];
                node.focusable
                    && node.state.visible
                    && !node.state.disabled
                    && !matches!(node.data, Content::Input { .. })
            })
        {
            return self
                .path_to(slot)
                .into_iter()
                .rev()
                .map(|slot| WidgetEvent::Click(self.nodes[slot.index()].id))
                .collect();
        }
        let Some(slot) = self
            .focused_slot()
            .filter(|slot| matches!(self.nodes[slot.index()].data, Content::Input { .. }))
        else {
            return vec![self.key_event(code)];
        };
        let multiline = matches!(
            self.nodes[slot.index()].data,
            Content::Input {
                multiline: true,
                ..
            }
        );
        let extend = self.modifiers.shift;
        match code {
            "Escape" => vec![self.key_event(code)],
            "ArrowUp" | "ArrowDown" if multiline => {
                self.move_caret_line(slot, code == "ArrowUp", extend);
                Vec::new()
            }
            "ArrowUp" | "ArrowDown" => vec![self.key_event(code)],
            "ArrowLeft" => {
                self.move_caret(slot, false, extend);
                Vec::new()
            }
            "ArrowRight" => {
                self.move_caret(slot, true, extend);
                Vec::new()
            }
            "Home" => {
                self.move_caret_edge(slot, true, extend);
                Vec::new()
            }
            "End" => {
                self.move_caret_edge(slot, false, extend);
                Vec::new()
            }
            "Backspace" => {
                let changed = self.delete_input(slot, true);
                self.input_event(slot, changed)
            }
            "Delete" => {
                let changed = self.delete_input(slot, false);
                self.input_event(slot, changed)
            }
            "Enter" if !multiline => vec![WidgetEvent::Submit(self.nodes[slot.index()].id)],
            "Enter" if self.modifiers.control => vec![self.key_event(code)],
            _ if self.modifiers.control => match code.to_ascii_uppercase().as_str() {
                "A" => {
                    self.select_all(slot);
                    Vec::new()
                }
                "C" => self.copy_input(slot),
                "X" => self.cut_input(slot),
                "V" => vec![WidgetEvent::Paste],
                _ => Vec::new(),
            },
            text => {
                let text = if text == "Enter" { "\n" } else { text };
                let changed = self.insert_input(slot, text);
                self.input_event(slot, changed)
            }
        }
    }

    pub(super) fn input_event(&mut self, slot: Slot, changed: bool) -> Vec<WidgetEvent> {
        if !changed {
            return Vec::new();
        }
        let node = &self.nodes[slot.index()];
        let Content::Input { value, caret, .. } = &node.data else {
            return Vec::new();
        };
        vec![WidgetEvent::Input(node.id, value.clone(), *caret)]
    }
}
