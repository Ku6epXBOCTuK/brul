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

    pub fn raw_key(&mut self, key: impl Into<KeyPress>) -> Vec<DispatchEvent> {
        let key = key.into();
        vec![self.key_event(&key.code, key.physical.as_deref())]
    }
}
