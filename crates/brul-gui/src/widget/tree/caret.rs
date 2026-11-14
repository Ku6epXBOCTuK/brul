use super::frame::reveal_carets;
use super::*;
use crate::layout;

impl WidgetTree {
    pub(super) fn place_caret(&mut self, slot: Slot, x: i32, y: i32, extend: bool) {
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        let node = &self.nodes[slot.index()];
        let Content::Input { scroll, .. } = &node.data else {
            return;
        };
        let Some(text) = self.texts.get(&node.id) else {
            return;
        };
        let layout_style = node.layout_style();
        let boxed = &geometry.boxes[slot.index()];
        let local_x = x.saturating_sub(boxed.x).saturating_sub(
            layout_style
                .border_width
                .saturating_add(layout_style.padding.left) as i32,
        );
        let content_height = boxed.height.saturating_sub(layout::inset_y(&layout_style));
        let local_y = y
            .saturating_sub(boxed.y)
            .saturating_sub(
                layout_style
                    .border_width
                    .saturating_add(layout_style.padding.top) as i32,
            )
            .saturating_add(*scroll)
            .saturating_sub(
                node.styles
                    .resolve(node.state)
                    .vertical_align
                    .offset(content_height, text.variants.base.size.height) as i32,
            );
        let byte = text.variants.base.caret_at(local_x, local_y);
        let Content::Input { caret, anchor, .. } = &mut self.nodes[slot.index()].data else {
            return;
        };
        let moved = *caret != byte || (!extend && *anchor != byte);
        *caret = byte;
        if !extend {
            *anchor = byte;
        }
        self.reveal_caret();
        if moved {
            self.invalidate_snapshot();
        }
    }

    pub(super) fn reveal_caret(&mut self) {
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        reveal_carets(&mut self.nodes, &self.texts, &geometry.boxes);
    }

    pub(super) fn selection(&self, slot: Slot) -> Option<(usize, usize)> {
        let Content::Input { caret, anchor, .. } = &self.nodes[slot.index()].data else {
            return None;
        };
        let (start, end) = if anchor <= caret {
            (*anchor, *caret)
        } else {
            (*caret, *anchor)
        };
        (start != end).then_some((start, end))
    }

    pub(super) fn select_all(&mut self, slot: Slot) {
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return;
        };
        *anchor = 0;
        *caret = value.len();
        self.stale.insert(slot);
        self.invalidate_snapshot();
    }

    pub(super) fn selected_text(&self, slot: Slot) -> Option<String> {
        let (start, end) = self.selection(slot)?;
        let Content::Input { value, .. } = &self.nodes[slot.index()].data else {
            return None;
        };
        value.get(start..end).map(str::to_owned)
    }

    pub(super) fn move_caret(&mut self, slot: Slot, right: bool, extend: bool) {
        let selection = self.selection(slot);
        let Content::Input {
            value,
            caret,
            anchor,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return;
        };
        let target = match selection {
            Some((start, end)) if !extend => {
                if right {
                    end
                } else {
                    start
                }
            }
            _ if right => value[*caret..]
                .chars()
                .next()
                .map_or(*caret, |character| *caret + character.len_utf8()),
            _ => value[..*caret]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index),
        };
        *caret = target;
        if !extend {
            *anchor = target;
        }
        self.stale.insert(slot);
        self.invalidate_snapshot();
    }

    pub(super) fn move_caret_edge(&mut self, slot: Slot, home: bool, extend: bool) {
        let id = self.nodes[slot.index()].id;
        let layout = self.texts.get(&id).map(|cached| &cached.variants.base);
        let Content::Input {
            value,
            caret,
            anchor,
            multiline,
            ..
        } = &mut self.nodes[slot.index()].data
        else {
            return;
        };
        *caret = if *multiline {
            let Some(layout) = layout else {
                return;
            };
            if home {
                layout.line_start(*caret)
            } else {
                layout.line_end(*caret)
            }
        } else if home {
            0
        } else {
            value.len()
        };
        if !extend {
            *anchor = *caret;
        }
        self.stale.insert(slot);
        self.invalidate_snapshot();
    }

    pub(super) fn move_caret_line(&mut self, slot: Slot, up: bool, extend: bool) {
        let id = self.nodes[slot.index()].id;
        let Some(cached) = self.texts.get(&id) else {
            return;
        };
        let layout = &cached.variants.base;
        let Content::Input { caret, .. } = &self.nodes[slot.index()].data else {
            return;
        };
        let Some(target) = (if up {
            layout.caret_above(*caret)
        } else {
            layout.caret_below(*caret)
        }) else {
            return;
        };
        if let Content::Input { caret, anchor, .. } = &mut self.nodes[slot.index()].data {
            *caret = target;
            if !extend {
                *anchor = target;
            }
        }
        self.reveal_caret();
        self.stale.insert(slot);
        self.invalidate_snapshot();
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
            .saturating_add(style.border_width.saturating_add(style.padding.left) as i32);
        let content_y = placed
            .y
            .saturating_add(style.border_width.saturating_add(style.padding.top) as i32);
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
