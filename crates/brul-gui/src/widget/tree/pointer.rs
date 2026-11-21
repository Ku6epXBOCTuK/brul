use super::frame::{HitScratch, collect_hit, common_prefix};
use super::render::max_scroll_offset;
use super::*;

impl WidgetTree {
    pub fn wheel(&mut self, x: i32, y: i32, dy: i32) -> bool {
        if dy == 0 {
            return false;
        }
        if self.hit_path(x, y).is_err() {
            return false;
        }
        let target = self.hit_scratch.path.iter().rev().copied().find(|slot| {
            let node = &self.nodes[slot.index()];
            matches!(node.data, Content::View)
                && (node.style().scroll || (self.root_scroll && *slot == self.root))
        });
        let Some(target) = target.filter(|target| {
            self.modal_scope()
                .is_none_or(|scope| self.within_scope(scope, *target))
        }) else {
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
        self.hit_path(x, y)?;
        self.collect_active_path();
        let old = std::mem::take(&mut self.hovered);
        let new = std::mem::take(&mut self.hit_scratch.active);
        let common = common_prefix(&old, &new);
        if common == old.len() && common == new.len() {
            self.hovered = new;
            self.hit_scratch.active = old;
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        for slot in old[common..].iter().rev().copied() {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &slot in &new[common..] {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = true;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerEnter(id));
        }
        self.hovered = new;
        self.hit_scratch.active = old;
        Ok(events)
    }

    pub fn pointer_down(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        self.hit_path(x, y)?;
        self.collect_active_path();
        let path = std::mem::take(&mut self.hit_scratch.active);
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
        self.pressed = path;
        Ok(events)
    }

    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        self.hit_path(x, y)?;
        self.collect_active_path();
        let pressed = std::mem::take(&mut self.pressed);
        let path = std::mem::take(&mut self.hit_scratch.active);

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
        self.hit_scratch.active = path;
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
        self.mark_dirty(id);
        true
    }

    pub(super) fn mark_dirty(&mut self, id: WidgetId) {
        if !self.full_dirty {
            self.dirty.insert(id);
        }
        self.pending.push(id);
        self.snapshot_stale = true;
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
        let slots = std::mem::take(&mut self.pressed);
        for &slot in &slots {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = pressed;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
        }
    }

    fn collect_active_path(&mut self) {
        let HitScratch { path, active, .. } = &mut self.hit_scratch;
        active_path(&self.nodes, path, active);
    }

    pub(super) fn hit_path(&mut self, x: i32, y: i32) -> Result<(), WidgetError> {
        self.hit_scratch.clear();
        let nodes = &self.nodes;
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let scratch = &mut self.hit_scratch;
        let root = self.root;
        collect_hit(nodes, root, &geometry.boxes, scratch, x, y);
        Ok(())
    }
}

fn active_path(nodes: &[Node], path: &[Slot], out: &mut Vec<Slot>) {
    out.clear();
    for &slot in path {
        let Some(node) = nodes.get(slot.index()) else {
            break;
        };
        if node.state.disabled || !node.state.visible {
            break;
        }
        out.push(slot);
    }
}
