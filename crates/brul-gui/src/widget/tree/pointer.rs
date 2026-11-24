use super::frame::{HitScratch, collect_hit, common_prefix};
use super::*;
use crate::Cursor;

impl WidgetTree {
    pub fn raw_wheel(&mut self, x: i32, y: i32, dy: i32) -> Vec<DispatchEvent> {
        if dy == 0 {
            return Vec::new();
        }
        let target = self
            .hit(x, y)
            .ok()
            .flatten()
            .map_or(EventTarget::Window, EventTarget::Widget);
        vec![DispatchEvent::Raw(RawUiEvent::at(
            target,
            RawEvent::wheel(dy),
        ))]
    }

    pub fn cursor(&self) -> Cursor {
        self.hovered
            .iter()
            .rev()
            .find_map(|slot| {
                let cursor = self.nodes[slot.index()].style().cursor;
                (cursor != Cursor::Default).then_some(cursor)
            })
            .unwrap_or_default()
    }

    pub fn pointer_raw_moved(&mut self, x: i32, y: i32) -> Result<Vec<DispatchEvent>, WidgetError> {
        self.hit_path(x, y)?;
        self.collect_active_path();
        let old = std::mem::take(&mut self.hovered);
        let new = std::mem::take(&mut self.hit_scratch.active);
        let common = common_prefix(&old, &new);

        let mut events = Vec::new();
        if common != old.len() || common != new.len() {
            for &slot in old[common..].iter().rev() {
                let id = self.nodes[slot.index()].id;
                self.retarget(slot);
                events.push(DispatchEvent::raw(id, RawEvent::pointer_leave()));
            }
            for &slot in &new[common..] {
                let id = self.nodes[slot.index()].id;
                self.retarget(slot);
                events.push(DispatchEvent::raw(id, RawEvent::pointer_enter()));
            }
        }
        self.hovered = new;
        self.hit_scratch.active = old;

        if !self.pressed.is_empty() {
            let modifiers = self.modifiers;
            for &slot in self.pressed.iter().rev() {
                let id = self.nodes[slot.index()].id;
                events.push(DispatchEvent::raw(
                    id,
                    RawEvent::pointer_move(x, y, modifiers),
                ));
            }
        }
        Ok(events)
    }

    pub fn pointer_raw_down(&mut self, x: i32, y: i32) -> Result<Vec<DispatchEvent>, WidgetError> {
        self.hit_path(x, y)?;
        self.collect_active_path();
        let path = std::mem::take(&mut self.hit_scratch.active);
        self.pressed.clear();
        let modifiers = self.modifiers;

        let mut events = Vec::new();
        for &slot in path.iter().rev() {
            let node = &self.nodes[slot.index()];
            let id = node.id;
            let draggable = node.draggable;
            self.retarget(slot);
            events.push(DispatchEvent::raw(id, RawEvent::press(x, y, modifiers)));
            if draggable {
                events.push(DispatchEvent::raw(id, RawEvent::drag(x, y)));
            }
        }
        self.pressed = path;
        Ok(events)
    }

    pub fn pointer_raw_up(&mut self, x: i32, y: i32) -> Result<Vec<DispatchEvent>, WidgetError> {
        self.hit_path(x, y)?;
        self.collect_active_path();
        let pressed = std::mem::take(&mut self.pressed);
        let path = std::mem::take(&mut self.hit_scratch.active);

        let mut events = Vec::new();
        for &slot in pressed.iter().rev() {
            let id = self.nodes[slot.index()].id;
            self.retarget(slot);
            events.push(DispatchEvent::raw(id, RawEvent::release()));
        }
        self.hit_scratch.active = path;
        Ok(events)
    }

    pub fn pointer_raw_leave(&mut self) -> Vec<DispatchEvent> {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        let mut events = Vec::with_capacity(hovered.len() + pressed.len());

        for &slot in hovered.iter().rev() {
            let id = self.nodes[slot.index()].id;
            self.retarget(slot);
            events.push(DispatchEvent::raw(id, RawEvent::pointer_leave()));
        }
        for &slot in pressed.iter().rev() {
            let id = self.nodes[slot.index()].id;
            self.retarget(slot);
            events.push(DispatchEvent::raw(id, RawEvent::release()));
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
