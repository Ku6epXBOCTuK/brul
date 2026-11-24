use super::*;

impl WidgetTree {
    pub fn raw_paste(&mut self, text: &str) -> Vec<DispatchEvent> {
        vec![DispatchEvent::Raw(RawUiEvent::at(
            self.event_target(),
            RawEvent::paste(text.to_owned()),
        ))]
    }

    pub fn raw_ime(&mut self, kind: ImeKind) -> Vec<DispatchEvent> {
        self.focused().map_or_else(Vec::new, |id| {
            vec![DispatchEvent::raw(id, RawEvent::ime(kind))]
        })
    }

    pub(super) fn focused_input(&self) -> Option<Slot> {
        let slot = self.focused_slot()?;
        matches!(self.nodes[slot.index()].data, Content::Input { .. }).then_some(slot)
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
