use std::ops::Range;

use super::*;
use crate::edit::{EditBuffer, Fragment, Position};

impl WidgetTree {
    pub fn focused_id(&self) -> Option<WidgetId> {
        self.focused()
    }

    pub fn focusables(&self) -> Vec<WidgetId> {
        self.focusable_slots()
            .into_iter()
            .map(|slot| self.nodes[slot.index()].id)
            .collect()
    }

    pub fn set_focused(&mut self, id: Option<WidgetId>) -> bool {
        let slot = match id {
            Some(id) => {
                let Some(slot) = self.index.get(&id).copied() else {
                    return false;
                };
                let node = &self.nodes[slot.index()];
                if !node.focusable || !node.state.visible || node.state.disabled {
                    return false;
                }
                Some(slot)
            }
            None => None,
        };
        if self.focused_slot() == slot {
            return false;
        }
        self.set_focused_slot(slot);
        true
    }

    pub fn modal_scope(&self) -> Option<WidgetId> {
        self.modal_scope_slot()
            .map(|slot| self.nodes[slot.index()].id)
    }

    pub fn within_scope(&self, scope: WidgetId, id: WidgetId) -> bool {
        let Some(scope) = self.index.get(&scope).copied() else {
            return false;
        };
        let Some(slot) = self.index.get(&id).copied() else {
            return false;
        };
        self.within_scope_slot(scope, slot)
    }

    pub fn hovered(&self, id: WidgetId) -> bool {
        self.node(id).is_some_and(|node| node.state.hovered)
    }

    pub fn pressed(&self, id: WidgetId) -> bool {
        self.node(id).is_some_and(|node| node.state.pressed)
    }

    pub fn set_hovered(&mut self, id: WidgetId, hovered: bool) -> bool {
        self.update_state(id, |state| state.hovered = hovered)
    }

    pub fn set_pressed(&mut self, id: WidgetId, pressed: bool) -> bool {
        self.update_state(id, |state| state.pressed = pressed)
    }

    pub fn scroll_by(&mut self, id: WidgetId, dy: i32) -> bool {
        if dy == 0 {
            return false;
        }
        let Some(slot) = self.index.get(&id).copied() else {
            return false;
        };
        if matches!(self.nodes[slot.index()].data, Content::Input { .. }) {
            let Content::Input { scroll, .. } = &mut self.nodes[slot.index()].data else {
                return false;
            };
            let next = scroll.saturating_add(dy).max(0);
            if next == *scroll {
                return false;
            }
            *scroll = next;
        } else if matches!(self.nodes[slot.index()].data, Content::View) {
            let Some(geometry) = self.geometry.as_ref() else {
                return false;
            };
            let Some(max) = render::max_scroll_offset(&self.nodes, slot, &geometry.boxes) else {
                return false;
            };
            let node = &mut self.nodes[slot.index()];
            let next = node.scroll_offset.saturating_add(dy).clamp(0, max);
            if next == node.scroll_offset {
                return false;
            }
            node.scroll_offset = next;
        } else {
            return false;
        }
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn caret_position(&self, id: WidgetId) -> Option<Position> {
        let node = self.node(id)?;
        let Content::Input { caret, .. } = &node.data else {
            return None;
        };
        Some(Position::new(0, *caret))
    }

    pub fn set_caret_position(&mut self, id: WidgetId, position: Position) -> bool {
        let Some(slot) = self.index.get(&id).copied() else {
            return false;
        };
        let Content::Input { value, .. } = &self.nodes[slot.index()].data else {
            return false;
        };
        if position.fragment != 0
            || position.offset > value.len()
            || !value.is_char_boundary(position.offset)
        {
            return false;
        }
        if let Content::Input { caret, .. } = &mut self.nodes[slot.index()].data {
            *caret = position.offset;
        }
        self.stale.insert(slot);
        self.reveal_caret();
        self.invalidate_snapshot();
        true
    }

    pub fn selection_range(&self, id: WidgetId) -> Option<Range<Position>> {
        let node = self.node(id)?;
        let Content::Input { caret, anchor, .. } = &node.data else {
            return None;
        };
        let start = (*anchor).min(*caret);
        let end = (*anchor).max(*caret);
        Some(Position::new(0, start)..Position::new(0, end))
    }

    pub fn edit_buffer(&self, id: WidgetId) -> Option<EditBuffer> {
        let node = self.node(id)?;
        let Content::Input {
            value,
            caret,
            anchor,
            scroll,
            preedit,
            ..
        } = &node.data
        else {
            return None;
        };
        let mut buffer = EditBuffer::from_text(value.clone());
        buffer.set_caret(Position::new(0, *caret));
        buffer.set_anchor(Position::new(0, *anchor));
        buffer.set_preedit(preedit.clone());
        buffer.set_scroll(*scroll);
        Some(buffer)
    }

    pub fn commit_edit(&mut self, id: WidgetId, buffer: &EditBuffer) -> bool {
        let [Fragment::Text(value)] = buffer.fragments() else {
            return false;
        };
        let caret = buffer.caret();
        let anchor = buffer.anchor();
        if caret.fragment != 0 || anchor.fragment != 0 {
            return false;
        }
        for position in [caret, anchor] {
            if position.offset > value.len() || !value.is_char_boundary(position.offset) {
                return false;
            }
        }
        let Some(slot) = self.index.get(&id).copied() else {
            return false;
        };
        let Content::Input {
            value: current,
            caret: current_caret,
            anchor: current_anchor,
            preedit,
            scroll,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        *current = value.clone();
        *current_caret = caret.offset;
        *current_anchor = anchor.offset;
        *preedit = buffer.preedit().to_owned();
        *scroll = buffer.scroll();
        self.stale.insert(slot);
        self.reveal_caret();
        self.invalidate_layout();
        true
    }
}
