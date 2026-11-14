use super::*;
use crate::layout::{self, LayoutBox, Size};
use crate::rect::ClipRect;
use std::collections::HashMap;

pub(super) fn boundary_before(value: &str, mut index: usize) -> usize {
    index = index.min(value.len());
    while !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

pub(super) fn reveal_carets(
    nodes: &mut [Node],
    texts: &HashMap<WidgetId, ShapedText>,
    boxes: &[LayoutBox],
) {
    for (slot, node) in nodes.iter_mut().enumerate() {
        let style = node.layout_style();
        let Content::Input {
            caret,
            multiline: true,
            scroll,
            ..
        } = &mut node.data
        else {
            continue;
        };
        let Some(cached) = texts.get(&node.id) else {
            continue;
        };
        let Some(placed) = boxes.get(slot) else {
            continue;
        };
        let content_height = placed.height.saturating_sub(layout::inset_y(&style)) as i32;
        let position = cached.variants.base.caret(*caret);
        let height = cached.variants.base.size.height as i32;
        let max = height.saturating_sub(content_height);
        let mut next = *scroll;
        if position.y < next {
            next = position.y;
        }
        if position.y.saturating_add(position.height) > next.saturating_add(content_height) {
            next = position
                .y
                .saturating_add(position.height)
                .saturating_sub(content_height);
        }
        *scroll = next.clamp(0, max.max(0));
    }
}

pub(super) fn build_snapshot(
    tree: &[Node],
    root: Slot,
    geometry: &Geometry,
    texts: &HashMap<WidgetId, ShapedText>,
    focused: Option<Slot>,
) -> WidgetSnapshot {
    let mut nodes = Vec::new();
    snapshot_widget(tree, root, &geometry.boxes, texts, &[], focused, &mut nodes);
    let root_box = &geometry.boxes[root.index()];
    WidgetSnapshot {
        size: Size {
            width: root_box.width,
            height: root_box.height,
        },
        nodes,
    }
}

pub(super) fn snapshot_widget(
    tree: &[Node],
    slot: Slot,
    boxes: &[LayoutBox],
    texts: &HashMap<WidgetId, ShapedText>,
    clips: &[ClipRect],
    focused: Option<Slot>,
    nodes: &mut Vec<WidgetSnapshotNode>,
) {
    let node = &tree[slot.index()];
    if !node.state.visible {
        return;
    }

    let layout_box = &boxes[slot.index()];
    let layout_style = node.layout_style();
    let style = node.style();
    let content_x = layout_box.x.saturating_add(
        layout_style
            .border_width
            .saturating_add(layout_style.padding.left) as i32,
    );
    let content_y = layout_box.y.saturating_add(
        layout_style
            .border_width
            .saturating_add(layout_style.padding.top) as i32,
    );
    let content_width = layout_box
        .width
        .saturating_sub(layout::inset_x(&layout_style));
    let content_height = layout_box
        .height
        .saturating_sub(layout::inset_y(&layout_style));
    let mut node_clips = clips.to_vec();
    if matches!(node.data, Content::Input { .. }) && content_width > 0 && content_height > 0 {
        node_clips.push(ClipRect::new(
            content_x,
            content_y,
            content_width,
            content_height,
            0,
        ));
    }
    let text = match &node.data {
        Content::Text { .. } | Content::Input { .. } => texts
            .get(&node.id)
            .map(|cached| cached.variants.resolve(node.state)),
        _ => None,
    };
    let preedit = match &node.data {
        Content::Input { .. } if focused == Some(slot) => texts
            .get(&node.id)
            .and_then(|cached| cached.preedit.clone()),
        _ => None,
    };
    let caret = match &node.data {
        Content::Input { caret, .. } if focused == Some(slot) => {
            texts.get(&node.id).map(|cached| {
                let mut position = cached.variants.base.caret(*caret);
                if let Some(preedit) = &cached.preedit {
                    position.x += preedit.size.width as i32;
                }
                position
            })
        }
        _ => None,
    };
    let selection = match &node.data {
        Content::Input { caret, anchor, .. } if focused == Some(slot) => {
            texts.get(&node.id).map_or_else(Vec::new, |cached| {
                cached.variants.base.selection(*anchor, *caret)
            })
        }
        _ => Vec::new(),
    };
    let scroll = match &node.data {
        Content::Input { scroll, .. } => *scroll,
        _ => 0,
    };
    let image = match &node.data {
        Content::Image(image) => Some(image.clone()),
        _ => None,
    };

    nodes.push(WidgetSnapshotNode {
        id: node.id,
        kind: node.data.kind(),
        x: layout_box.x,
        y: layout_box.y,
        width: layout_box.width,
        height: layout_box.height,
        border_width: layout_style.border_width,
        content_x,
        content_y,
        content_width,
        content_height,
        state: node.state,
        style,
        text,
        preedit,
        caret,
        selection,
        scroll,
        image,
        clips: node_clips,
    });

    let mut child_clips = clips.to_vec();
    if style.clip {
        child_clips.push(ClipRect::new(
            layout_box.x,
            layout_box.y,
            layout_box.width,
            layout_box.height,
            style.border_radius,
        ));
    }
    for &child in &node.children {
        snapshot_widget(tree, child, boxes, texts, &child_clips, focused, nodes);
    }
}

pub(super) fn collect_hit(
    tree: &[Node],
    slot: Slot,
    boxes: &[LayoutBox],
    clips: &[ClipRect],
    x: i32,
    y: i32,
    path: &mut Vec<Slot>,
) -> bool {
    let node = &tree[slot.index()];
    if !node.state.visible
        || !contains(&boxes[slot.index()], x, y)
        || !clips.iter().all(|clip| clip.contains(x, y))
    {
        return false;
    }

    let style = node.styles.resolve(node.state);
    let mut child_clips = clips.to_vec();
    if style.clip {
        let layout_box = &boxes[slot.index()];
        child_clips.push(ClipRect::new(
            layout_box.x,
            layout_box.y,
            layout_box.width,
            layout_box.height,
            style.border_radius,
        ));
    }

    path.push(slot);
    for &child in node.children.iter().rev() {
        if collect_hit(tree, child, boxes, &child_clips, x, y, path) {
            return true;
        }
    }
    true
}

pub(super) fn contains(layout_box: &LayoutBox, x: i32, y: i32) -> bool {
    let right = i64::from(layout_box.x) + i64::from(layout_box.width);
    let bottom = i64::from(layout_box.y) + i64::from(layout_box.height);
    i64::from(x) >= i64::from(layout_box.x)
        && i64::from(y) >= i64::from(layout_box.y)
        && i64::from(x) < right
        && i64::from(y) < bottom
}

pub(super) fn common_prefix(left: &[Slot], right: &[Slot]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}
