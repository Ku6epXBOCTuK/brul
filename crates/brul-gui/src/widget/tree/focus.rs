use super::*;

impl WidgetTree {
    pub(super) fn focus_next(&mut self) -> Vec<WidgetEvent> {
        self.enter_modal_scope();
        let slots = self.focusable_slots();
        let Some(first) = slots.first().copied() else {
            return self.focus_events(None);
        };
        let target = match self
            .focused_slot()
            .and_then(|current| slots.iter().position(|slot| *slot == current))
        {
            Some(index) => slots[(index + 1) % slots.len()],
            None => first,
        };
        self.focus_events(Some(target))
    }

    pub(super) fn focus_previous(&mut self) -> Vec<WidgetEvent> {
        self.enter_modal_scope();
        let slots = self.focusable_slots();
        if slots.is_empty() {
            return self.focus_events(None);
        }
        let target = match self
            .focused_slot()
            .and_then(|current| slots.iter().position(|slot| *slot == current))
        {
            Some(index) => slots[(index + slots.len() - 1) % slots.len()],
            None => slots[slots.len() - 1],
        };
        self.focus_events(Some(target))
    }

    pub(super) fn path_to(&self, target: Slot) -> Vec<Slot> {
        let mut path = Vec::new();
        self.collect_path(self.root, target, &mut path);
        path
    }

    fn collect_path(&self, slot: Slot, target: Slot, path: &mut Vec<Slot>) -> bool {
        path.push(slot);
        if slot == target {
            return true;
        }
        for &child in &self.nodes[slot.index()].children {
            if self.collect_path(child, target, path) {
                return true;
            }
        }
        path.pop();
        false
    }

    pub(super) fn focusable_slots(&self) -> Vec<Slot> {
        let mut slots: Vec<Slot> = self
            .index
            .values()
            .copied()
            .filter(|slot| {
                let node = &self.nodes[slot.index()];
                node.focusable && node.state.visible && !node.state.disabled
            })
            .collect();
        if let Some(scope) = self.modal_scope() {
            slots.retain(|slot| self.within_scope(scope, *slot));
        }
        slots.sort_by_key(|slot| slot.index());
        slots
    }

    pub(super) fn modal_scope(&self) -> Option<Slot> {
        self.topmost_modal(self.root)
    }

    pub(super) fn sync_modal_focus(&mut self) {
        let Some(scope) = self.modal_scope() else {
            let Some(previous) = self.modal_focus.take() else {
                return;
            };
            let Some(slot) = self.index.get(&previous).copied() else {
                return;
            };
            let node = &self.nodes[slot.index()];
            let restore = node.focusable && node.state.visible && !node.state.disabled;
            let target = restore.then_some(slot);
            self.set_focused(target);
            return;
        };
        if self
            .focused_slot()
            .is_some_and(|focused| self.within_scope(scope, focused))
        {
            return;
        }
        if self.modal_focus.is_none() {
            self.modal_focus = self.focused;
        }
        let target = self.scope_focusable(scope).first().copied();
        self.set_focused(target);
    }

    fn enter_modal_scope(&mut self) {
        let Some(scope) = self.modal_scope() else {
            return;
        };
        let outside = !self
            .focused_slot()
            .is_some_and(|focused| self.within_scope(scope, focused));
        if outside && self.modal_focus.is_none() {
            self.modal_focus = self.focused;
        }
    }

    fn topmost_modal(&self, slot: Slot) -> Option<Slot> {
        let node = &self.nodes[slot.index()];
        if !node.state.visible {
            return None;
        }
        let mut found = None;
        for &child in node
            .children
            .iter()
            .filter(|child| !self.nodes[child.index()].layer)
        {
            found = self.topmost_modal(child).or(found);
        }
        for &child in node
            .children
            .iter()
            .filter(|child| self.nodes[child.index()].layer)
        {
            found = self.topmost_modal(child).or(found);
        }
        if found.is_none() && node.layer && node.modal {
            Some(slot)
        } else {
            found
        }
    }

    pub(super) fn within_scope(&self, scope: Slot, slot: Slot) -> bool {
        if scope == slot {
            return true;
        }
        self.nodes[scope.index()]
            .children
            .iter()
            .any(|child| self.within_scope(*child, slot))
    }

    fn scope_focusable(&self, scope: Slot) -> Vec<Slot> {
        let mut slots = Vec::new();
        self.collect_focusable(scope, &mut slots);
        slots
    }

    fn collect_focusable(&self, slot: Slot, slots: &mut Vec<Slot>) {
        let node = &self.nodes[slot.index()];
        if !node.state.visible {
            return;
        }
        if node.focusable && !node.state.disabled {
            slots.push(slot);
        }
        for child in &node.children {
            self.collect_focusable(*child, slots);
        }
    }

    pub(super) fn focus_events(&mut self, slot: Option<Slot>) -> Vec<WidgetEvent> {
        if self.focused_slot() == slot {
            return Vec::new();
        }
        let mut events = Vec::with_capacity(2);
        if let Some(previous) = self.focused {
            events.push(WidgetEvent::Blur(previous));
        }
        if let Some(next) = slot {
            events.push(WidgetEvent::Focus(self.nodes[next.index()].id));
        }
        self.set_focused(slot);
        events
    }

    fn set_focused(&mut self, slot: Option<Slot>) {
        if self.focused_slot() == slot {
            return;
        }
        if let Some(previous) = self.focused_slot() {
            self.nodes[previous.index()].state.focused = false;
        }
        if let Some(next) = slot {
            self.nodes[next.index()].state.focused = true;
        }
        self.focused = slot.map(|slot| self.nodes[slot.index()].id);
        self.invalidate_snapshot();
    }
}
