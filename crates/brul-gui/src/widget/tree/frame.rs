use super::*;
use crate::layout::{self, LayoutBox, Size};
use crate::rect::ClipRect;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct HitScratch {
    pub clips: Vec<ClipRect>,
    pub path: Vec<Slot>,
    pub active: Vec<Slot>,
}

impl HitScratch {
    pub fn clear(&mut self) {
        self.clips.clear();
        self.path.clear();
    }
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
            wrap: true,
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
    let index = nodes
        .iter()
        .enumerate()
        .map(|(at, node)| (node.id, at))
        .collect();
    let root_box = &geometry.boxes[root.index()];
    WidgetSnapshot {
        size: Size {
            width: root_box.width,
            height: root_box.height,
        },
        nodes,
        index,
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
    if matches!(node.data, Content::Structure) {
        for &child in node
            .children
            .iter()
            .filter(|child| !tree[child.index()].layer)
        {
            snapshot_widget(tree, child, boxes, texts, clips, focused, nodes);
        }
        for &child in node
            .children
            .iter()
            .filter(|child| tree[child.index()].layer)
        {
            snapshot_widget(tree, child, boxes, texts, clips, focused, nodes);
        }
        return;
    }

    let layout_box = &boxes[slot.index()];
    let style = node.style();
    nodes.push(snapshot_node(node, slot, layout_box, texts, clips, focused));

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
    for &child in node
        .children
        .iter()
        .filter(|child| !tree[child.index()].layer)
    {
        snapshot_widget(tree, child, boxes, texts, &child_clips, focused, nodes);
    }
    for &child in node
        .children
        .iter()
        .filter(|child| tree[child.index()].layer)
    {
        snapshot_widget(tree, child, boxes, texts, &child_clips, focused, nodes);
    }
}

pub(super) fn snapshot_node(
    node: &Node,
    slot: Slot,
    layout_box: &LayoutBox,
    texts: &HashMap<WidgetId, ShapedText>,
    clips: &[ClipRect],
    focused: Option<Slot>,
) -> WidgetSnapshotNode {
    let layout_style = node.layout_style();
    let style = node.style();
    let content_x = layout_box.x.saturating_add(
        layout_style
            .border_width
            .left
            .saturating_add(layout_style.padding.left) as i32,
    );
    let content_y = layout_box.y.saturating_add(
        layout_style
            .border_width
            .top
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
    let platform_view = match &node.data {
        Content::PlatformView(spec) => Some(spec.clone()),
        _ => None,
    };

    WidgetSnapshotNode {
        id: node.id,
        kind: node.data.kind(),
        x: layout_box.x,
        y: layout_box.y,
        width: layout_box.width,
        height: layout_box.height,
        border: crate::Border::new(layout_style.border_width, style.border),
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
        platform_view,
        effect: node.effect.clone(),
        clips: node_clips,
    }
}

pub(super) fn collect_hit(
    tree: &[Node],
    slot: Slot,
    boxes: &[LayoutBox],
    scratch: &mut HitScratch,
    x: i32,
    y: i32,
) -> bool {
    let node = &tree[slot.index()];
    let structure = matches!(node.data, Content::Structure);
    if !node.state.visible
        || matches!(node.data, Content::PlatformView(_))
        || (!structure && !contains(&boxes[slot.index()], x, y))
        || !scratch.clips.iter().all(|clip| clip.contains(x, y))
    {
        return false;
    }

    let style = node.style();
    let clip = style.clip.then(|| {
        let layout_box = &boxes[slot.index()];
        ClipRect::new(
            layout_box.x,
            layout_box.y,
            layout_box.width,
            layout_box.height,
            style.border_radius,
        )
    });
    if let Some(clip) = clip {
        scratch.clips.push(clip);
    }
    scratch.path.push(slot);

    let mut hit = false;
    for layer in [true, false] {
        for &child in node.children.iter().rev() {
            if tree[child.index()].layer != layer {
                continue;
            }
            if collect_hit(tree, child, boxes, scratch, x, y) {
                hit = true;
                break;
            }
        }
        if hit {
            break;
        }
    }

    if clip.is_some() {
        scratch.clips.pop();
    }
    if hit {
        return true;
    }
    if node.layer || structure {
        scratch.path.pop();
        return false;
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
