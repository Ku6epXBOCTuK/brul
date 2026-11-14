use super::*;

impl WidgetTree {
    pub(super) fn focus_next(&mut self) -> Vec<WidgetEvent> {
        let slots = self.focusable_slots();
        let Some(first) = slots.first().copied() else {
            return self.focus_events(None);
        };
        let target = match self
            .focused
            .and_then(|current| slots.iter().position(|slot| *slot == current))
        {
            Some(index) => slots[(index + 1) % slots.len()],
            None => first,
        };
        self.focus_events(Some(target))
    }

    pub(super) fn focus_previous(&mut self) -> Vec<WidgetEvent> {
        let slots = self.focusable_slots();
        if slots.is_empty() {
            return self.focus_events(None);
        }
        let target = match self
            .focused
            .and_then(|current| slots.iter().position(|slot| *slot == current))
        {
            Some(index) => slots[(index + slots.len() - 1) % slots.len()],
            None => slots[slots.len() - 1],
        };
        self.focus_events(Some(target))
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
        slots.sort_by_key(|slot| slot.index());
        slots
    }

    pub(super) fn focus_events(&mut self, slot: Option<Slot>) -> Vec<WidgetEvent> {
        if self.focused == slot {
            return Vec::new();
        }
        let mut events = Vec::with_capacity(2);
        if let Some(previous) = self.focused {
            events.push(WidgetEvent::Blur(self.nodes[previous.index()].id));
        }
        if let Some(next) = slot {
            events.push(WidgetEvent::Focus(self.nodes[next.index()].id));
        }
        self.focused = slot;
        self.invalidate_snapshot();
        events
    }
}
