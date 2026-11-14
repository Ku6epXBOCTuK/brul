use super::frame::{collect_hit, common_prefix};
use super::render::max_scroll_offset;
use super::*;

impl WidgetTree {
    pub fn wheel(&mut self, x: i32, y: i32, dy: i32) -> bool {
        if dy == 0 {
            return false;
        }
        let Ok(path) = self.hit_path(x, y) else {
            return false;
        };
        let target = path.iter().rev().copied().find(|slot| {
            let node = &self.nodes[slot.index()];
            matches!(node.data, Content::View { .. })
                && (node.styles.resolve(node.state).scroll
                    || (self.root_scroll && *slot == self.root))
        });
        let Some(target) = target else {
            return false;
        };
        let Some(geometry) = self.geometry.as_ref() else {
            return false;
        };
        let Some(max) = max_scroll_offset(&self.nodes, target, &geometry.boxes) else {
            return false;
        };
        let node = &mut self.nodes[target.index()];
        let scroll_offset = node.scroll_offset.saturating_add(dy).clamp(0, max);
        if scroll_offset == node.scroll_offset {
            return false;
        }
        node.scroll_offset = scroll_offset;
        self.stale.insert(target);
        self.invalidate();
        true
    }

    pub fn pointer_moved(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        if let Some(slot) = self
            .pressed
            .iter()
            .rev()
            .copied()
            .find(|slot| matches!(self.nodes[slot.index()].data, Content::Input { .. }))
        {
            self.place_caret(slot, x, y, true);
        }
        let path = self.active_path(&self.hit_path(x, y)?);
        let common = common_prefix(&self.hovered, &path);
        if common == self.hovered.len() && common == path.len() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let leaving: Vec<Slot> = self.hovered[common..].iter().rev().copied().collect();
        for slot in leaving {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &slot in &path[common..] {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = true;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerEnter(id));
        }
        self.hovered = path;
        self.snapshot = None;
        Ok(events)
    }

    pub fn pointer_down(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        self.set_pressed(false);

        let focus = path
            .iter()
            .rev()
            .copied()
            .find(|slot| self.nodes[slot.index()].focusable);
        let mut events = self.focus_events(focus);
        if let Some(slot) = focus {
            self.place_caret(slot, x, y, self.modifiers.shift);
        }
        for &slot in path.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = true;
            let id = node.id;
            let draggable = node.draggable;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::Press(id));
            if draggable {
                events.push(WidgetEvent::Drag(id));
            }
        }
        if !path.is_empty() {
            self.snapshot = None;
        }
        self.pressed = path;
        Ok(events)
    }

    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let pressed = std::mem::take(&mut self.pressed);

        let mut events = Vec::new();
        for &slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::Release(id));
        }
        if !pressed.is_empty() && pressed.last() == path.last() {
            for &slot in path.iter().rev() {
                events.push(WidgetEvent::Click(self.nodes[slot.index()].id));
            }
        }
        if !pressed.is_empty() {
            self.snapshot = None;
        }
        Ok(events)
    }

    pub fn pointer_leave(&mut self) -> Vec<WidgetEvent> {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        let mut events = Vec::with_capacity(hovered.len() + pressed.len());

        for slot in hovered.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(*slot);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(*slot);
            events.push(WidgetEvent::Release(id));
        }
        if !hovered.is_empty() || !pressed.is_empty() {
            self.snapshot = None;
        }
        events
    }

    pub(super) fn update_state(
        &mut self,
        id: WidgetId,
        update: impl FnOnce(&mut WidgetState),
    ) -> bool {
        let changed = {
            let Some(state) = self.node_mut(id).map(|node| &mut node.state) else {
                return false;
            };
            let before = *state;
            update(state);
            *state != before
        };
        if !changed {
            return false;
        }
        if let Some(&slot) = self.index.get(&id) {
            self.retarget(slot);
        }
        self.snapshot = None;
        self.mark_dirty(id);
        true
    }

    pub(super) fn mark_dirty(&mut self, id: WidgetId) {
        if !self.full_dirty {
            self.dirty.insert(id);
        }
    }

    pub(super) fn clear_interaction(&mut self) {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        for slot in hovered {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.hovered = false;
                node.shown = node.styles.resolve(node.state);
            }
        }
        for slot in pressed {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.pressed = false;
                node.shown = node.styles.resolve(node.state);
            }
        }
    }

    pub(super) fn set_pressed(&mut self, pressed: bool) {
        let had_pressed = !self.pressed.is_empty();
        let slots = std::mem::take(&mut self.pressed);
        for &slot in &slots {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = pressed;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
        }
        if had_pressed {
            self.snapshot = None;
        }
    }

    pub(super) fn hit_path(&self, x: i32, y: i32) -> Result<Vec<Slot>, WidgetError> {
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let mut path = Vec::new();
        collect_hit(
            &self.nodes,
            self.root,
            &geometry.boxes,
            &[],
            x,
            y,
            &mut path,
        );
        Ok(path)
    }

    pub(super) fn active_path(&self, path: &[Slot]) -> Vec<Slot> {
        let mut active = Vec::with_capacity(path.len());
        for &slot in path {
            let Some(node) = self.nodes.get(slot.index()) else {
                break;
            };
            if node.state.disabled || !node.state.visible {
                break;
            }
            active.push(slot);
        }
        active
    }
}
