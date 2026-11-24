use super::frame::reveal_carets;
use super::*;
use crate::layout;

impl WidgetTree {
    pub(super) fn reveal_caret(&mut self) {
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        reveal_carets(&mut self.nodes, &self.texts, &geometry.boxes);
    }

    pub fn caret_area(&self) -> Option<CaretArea> {
        let slot = self.focused_input()?;
        let geometry = self.geometry.as_ref()?;
        let node = &self.nodes[slot.index()];
        let Content::Input { caret, scroll, .. } = &node.data else {
            return None;
        };
        let cached = self.texts.get(&node.id)?;
        let placed = geometry.boxes.get(slot.index())?;
        let style = node.layout_style();
        let content_x = placed
            .x
            .saturating_add(style.border_width.left.saturating_add(style.padding.left) as i32);
        let content_y = placed
            .y
            .saturating_add(style.border_width.top.saturating_add(style.padding.top) as i32);
        let content_height = placed.height.saturating_sub(layout::inset_y(&style));
        let offset = node
            .styles
            .resolve(node.state)
            .vertical_align
            .offset(content_height, cached.variants.base.size.height) as i32;
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
}
