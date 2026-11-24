use super::frame::reveal_carets;
use super::*;
use crate::edit::Position;
use crate::layout;

impl WidgetTree {
    fn input_text_origin(&self, slot: Slot, text_height: u32) -> Option<(i32, i32, i32)> {
        let geometry = self.geometry.as_ref()?;
        let node = &self.nodes[slot.index()];
        let placed = geometry.boxes.get(slot.index())?;
        let style = node.layout_style();
        let content_height = placed.height.saturating_sub(layout::inset_y(&style));
        let content_x = placed
            .x
            .saturating_add(style.border_width.left.saturating_add(style.padding.left) as i32);
        let content_y = placed
            .y
            .saturating_add(style.border_width.top.saturating_add(style.padding.top) as i32);
        let offset = node
            .styles
            .resolve(node.state)
            .vertical_align
            .offset(content_height, text_height) as i32;
        Some((content_x, content_y, offset))
    }

    pub(super) fn reveal_caret(&mut self) {
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        reveal_carets(&mut self.nodes, &self.texts, &geometry.boxes);
    }

    pub fn caret_area(&self) -> Option<CaretArea> {
        let slot = self.focused_input()?;
        let node = &self.nodes[slot.index()];
        let Content::Input { caret, scroll, .. } = &node.data else {
            return None;
        };
        let cached = self.texts.get(&node.id)?;
        let (content_x, content_y, offset) =
            self.input_text_origin(slot, cached.variants.base.size.height)?;
        let position = cached.variants.base.caret(*caret);
        let x = content_x.saturating_add(position.x);
        let y = content_y
            .saturating_add(position.y)
            .saturating_add(offset)
            .saturating_sub(*scroll);
        Some(CaretArea {
            x,
            y,
            width: 1,
            height: position.height.max(1) as u32,
        })
    }

    pub fn caret_at_pixel(&self, id: WidgetId, x: i32, y: i32) -> Option<Position> {
        let slot = self.index.get(&id).copied()?;
        let node = &self.nodes[slot.index()];
        let Content::Input { scroll, .. } = &node.data else {
            return None;
        };
        let cached = self.texts.get(&id)?;
        let (content_x, content_y, offset) =
            self.input_text_origin(slot, cached.variants.base.size.height)?;
        let local_x = x.saturating_sub(content_x);
        let local_y = y
            .saturating_sub(content_y)
            .saturating_add(*scroll)
            .saturating_sub(offset);
        Some(Position::new(
            0,
            cached.variants.base.caret_at(local_x, local_y),
        ))
    }
}
