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

    pub fn event_target(&self) -> EventTarget {
        self.focused
            .map_or(EventTarget::Window, EventTarget::Widget)
    }

    pub fn set_modifiers(&mut self, modifiers: KeyModifiers) {
        self.modifiers = modifiers;
    }

    pub(super) fn key_event(&self, code: &str, physical: Option<&str>) -> DispatchEvent {
        DispatchEvent::Raw(RawUiEvent::at(
            self.event_target(),
            RawEvent::key(
                KeyPress {
                    code: code.to_owned(),
                    physical: physical.map(str::to_owned),
                },
                self.modifiers,
            ),
        ))
    }

    pub(super) fn key_unchanged_event(&self, key: &KeyPress) -> DispatchEvent {
        DispatchEvent::Raw(RawUiEvent::at(
            self.event_target(),
            RawEvent::key_unchanged(key.clone(), self.modifiers),
        ))
    }

    pub fn press_key(&mut self, key: impl Into<KeyPress>) -> Vec<DispatchEvent> {
        let key = key.into();
        let code = key.code.as_str();
        let physical = key.physical.as_deref();
        if code == "Escape"
            && let Some(scope) = self.modal_scope()
        {
            return vec![DispatchEvent::semantic(
                self.nodes[scope.index()].id,
                Event::dismiss(),
            )];
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
                .map(|slot| DispatchEvent::semantic(self.nodes[slot.index()].id, Event::click()))
                .collect();
        }
        let Some(slot) = self
            .focused_slot()
            .filter(|slot| matches!(self.nodes[slot.index()].data, Content::Input { .. }))
        else {
            return vec![self.key_event(code, physical)];
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
            "Escape" => vec![self.key_event(code, physical)],
            "ArrowUp" | "ArrowDown" if multiline => {
                self.move_caret_line(slot, code == "ArrowUp", extend);
                Vec::new()
            }
            "ArrowUp" | "ArrowDown" => vec![self.key_event(code, physical)],
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
                if self.delete_input(slot, true) {
                    self.input_event(slot, true)
                } else {
                    vec![self.key_unchanged_event(&key)]
                }
            }
            "Delete" => {
                if self.delete_input(slot, false) {
                    self.input_event(slot, true)
                } else {
                    vec![self.key_unchanged_event(&key)]
                }
            }
            "Enter" if !multiline => vec![DispatchEvent::semantic(
                self.nodes[slot.index()].id,
                Event::submit(),
            )],
            "Enter" if self.modifiers.control => vec![self.key_event(code, physical)],
            _ if self.modifiers.control => match code.to_ascii_uppercase().as_str() {
                "A" => {
                    self.select_all(slot);
                    Vec::new()
                }
                "C" => self.copy_input(slot),
                "X" => self.cut_input(slot),
                "V" => vec![DispatchEvent::raw(
                    self.nodes[slot.index()].id,
                    RawEvent::paste(String::new()),
                )],
                _ => Vec::new(),
            },
            text => {
                let text = if text == "Enter" { "\n" } else { text };
                let changed = self.insert_input(slot, text);
                self.input_event(slot, changed)
            }
        }
    }

    pub(super) fn input_event(&mut self, slot: Slot, changed: bool) -> Vec<DispatchEvent> {
        if !changed {
            return Vec::new();
        }
        let node = &self.nodes[slot.index()];
        let Content::Input { value, caret, .. } = &node.data else {
            return Vec::new();
        };
        vec![DispatchEvent::semantic(
            node.id,
            Event::input(value.clone(), *caret),
        )]
    }
}
