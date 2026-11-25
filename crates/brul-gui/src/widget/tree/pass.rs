use super::frame::{build_snapshot, reveal_carets, snapshot_node};
use super::render::{Pass, measure_text};
use super::style::Transition;
use super::*;
use crate::Rect;
use crate::layout::{AnchorAlign, AnchorSide, LayoutBox, Size};
use crate::text::TextEngine;
use std::time::Duration;

impl WidgetTree {
    pub fn layout(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
    ) -> Result<(), WidgetError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(WidgetError::InvalidScale);
        }

        let frame_changed = !self
            .geometry
            .as_ref()
            .is_some_and(|geometry| geometry.viewport == viewport && geometry.scale == scale);
        self.layout_pass(engine, viewport, scale, frame_changed)
    }

    fn layout_pass(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
        frame_changed: bool,
    ) -> Result<(), WidgetError> {
        let previous = self.geometry.take();
        let mut boxes = previous.map(|geometry| geometry.boxes).unwrap_or_default();
        boxes.resize(self.nodes.len(), LayoutBox::default());

        self.layouter.begin();
        let key = {
            let mut pass = Pass {
                tree: &self.nodes,
                layouter: &mut self.layouter,
                stale: &self.stale,
                viewport,
            };
            pass.build(self.root)?
        };
        self.texts.retain(|id, _| self.index.contains_key(id));

        let mut failed = None;
        self.layouter.finish();
        self.layouter.compute(key, viewport, |content, width| {
            let size = measure_text(
                &self.nodes[content.index()],
                engine,
                &mut self.texts,
                width,
                scale,
            );
            match size {
                Ok(size) => size,
                Err(error) => {
                    failed = Some(error);
                    Size::default()
                }
            }
        });
        if let Some(error) = failed {
            return Err(error);
        }
        self.layouter.collect_into(key, &mut boxes);
        resolve_anchors(&self.nodes, self.root, &mut boxes, viewport);

        self.geometry = Some(Geometry {
            viewport,
            scale,
            boxes,
        });
        if let Some(geometry) = self.geometry.as_ref() {
            reveal_carets(&mut self.nodes, &self.texts, &geometry.boxes);
        }
        self.stale.clear();
        self.snapshot = None;
        self.full_dirty = true;
        self.layout_dirty = false;
        self.dirty.clear();
        if frame_changed {
            self.clear_interaction();
        }
        Ok(())
    }

    pub fn relayout(&mut self, engine: &mut TextEngine) -> Result<(), WidgetError> {
        let Some((viewport, scale)) = self
            .geometry
            .as_ref()
            .map(|geometry| (geometry.viewport, geometry.scale))
        else {
            return Ok(());
        };
        self.layout(engine, viewport, scale)
    }

    pub fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }

    pub fn has_layout(&self) -> bool {
        self.geometry.is_some()
    }

    pub fn rect(&self, id: WidgetId) -> Option<Rect> {
        let geometry = self.geometry.as_ref()?;
        let slot = self.index.get(&id)?;
        let bounds = geometry.boxes.get(slot.index())?;
        Some(Rect::new(bounds.x, bounds.y, bounds.width, bounds.height))
    }

    pub fn snapshot(&mut self) -> Result<&WidgetSnapshot, WidgetError> {
        if self.snapshot.is_none() || (self.snapshot_stale && !self.patch_snapshot()) {
            let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
            let snapshot = build_snapshot(
                &self.nodes,
                self.root,
                geometry,
                &self.texts,
                &self.painters,
                self.focused_slot(),
            );
            self.snapshot = Some(snapshot);
        }
        self.snapshot_stale = false;
        self.pending.clear();
        Ok(self.snapshot.as_ref().expect("snapshot built"))
    }

    fn patch_snapshot(&mut self) -> bool {
        let Some(geometry) = self.geometry.as_ref() else {
            return false;
        };
        let focused = self.focused_slot();
        let Some(snapshot) = self.snapshot.as_mut() else {
            return false;
        };
        for id in std::mem::take(&mut self.pending) {
            let Some(&slot) = self.index.get(&id) else {
                return false;
            };
            let Some(&at) = snapshot.index.get(&id) else {
                return false;
            };
            let node = &self.nodes[slot.index()];
            if !node.state.visible {
                return false;
            }
            let clips = std::mem::take(&mut snapshot.nodes[at].clips);
            let fresh = snapshot_node(
                node,
                slot,
                &geometry.boxes[slot.index()],
                &self.texts,
                &self.painters,
                &clips,
                focused,
            );
            let old = &snapshot.nodes[at];
            if !old.same_child_clip(&fresh) {
                return false;
            }
            snapshot.nodes[at] = fresh;
        }
        true
    }

    pub fn snapshot_dirty(&self) -> bool {
        self.snapshot.is_none() || self.snapshot_stale
    }

    pub fn take_dirty(&mut self) -> Dirty {
        let full = std::mem::take(&mut self.full_dirty);
        Dirty {
            full,
            ids: if full {
                self.dirty.clear();
                Vec::new()
            } else {
                self.dirty.drain().collect()
            },
        }
    }

    pub fn animating(&self) -> bool {
        self.active_transitions > 0 || self.animated_effects()
    }

    fn animated_effects(&self) -> bool {
        self.nodes.iter().any(|node| {
            if node.effect.as_ref().is_some_and(|effect| effect.animated) {
                return true;
            }
            matches!(&node.data, Content::Image(image) if image.is_external())
                || matches!(&node.data, Content::Canvas(spec) if spec.animated)
        })
    }

    pub fn advance(&mut self, delta: Duration) -> bool {
        if delta.is_zero() {
            return self.animating();
        }
        let mut running = false;
        let animated: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| {
                node.effect.as_ref().is_some_and(|effect| effect.animated)
                    || matches!(&node.data, Content::Image(image) if image.is_external())
                    || matches!(&node.data, Content::Canvas(spec) if spec.animated)
            })
            .map(|node| node.id)
            .collect();
        for &id in &animated {
            self.mark_dirty(id);
        }
        for index in 0..self.nodes.len() {
            let (id, active) = {
                let node = &mut self.nodes[index];
                let Some(transition) = node.transition.as_mut() else {
                    continue;
                };
                let active = transition.advance(delta);
                if !active {
                    node.transition = None;
                }
                (node.id, active)
            };
            if !active {
                self.active_transitions -= 1;
            }
            running |= active;
            self.mark_dirty(id);
        }
        running || !animated.is_empty()
    }

    pub(super) fn retarget(&mut self, slot: Slot) {
        let styles = self.nodes[slot.index()].styles();
        let state = self.nodes[slot.index()].state;
        let durations = styles.transitions();
        let target = styles.resolve(state);
        let from = match self.nodes[slot.index()].transition.take() {
            Some(transition) if transition.to != target => {
                self.active_transitions -= 1;
                transition.style()
            }
            Some(transition) => {
                self.set_transition(slot, Some(transition));
                return;
            }
            None => self.nodes[slot.index()].shown,
        };
        self.nodes[slot.index()].shown = target;
        if durations.is_empty() {
            return;
        }
        let transition = (from != target).then(|| Transition::new(from, target, durations));
        self.set_transition(slot, transition);
    }
}

#[derive(Clone, Copy)]
struct Placement {
    side: AnchorSide,
    gap: i32,
    align: AnchorAlign,
    flip: bool,
}

fn resolve_anchors(nodes: &[Node], root: Slot, boxes: &mut [LayoutBox], viewport: Size) {
    let mut parents = HashMap::with_capacity(nodes.len());
    for (parent, node) in nodes.iter().enumerate() {
        for &child in &node.children {
            parents.insert(child, Slot(parent as u32));
        }
    }
    let anchors: Vec<(Slot, Anchor)> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.anchor.map(|anchor| (Slot(index as u32), anchor)))
        .collect();
    for (slot, anchor) in anchors {
        let Some(target) = find_target(nodes, root, slot, anchor.target, &parents) else {
            continue;
        };
        let layout = nodes[slot.index()].layout_style();
        let placement = Placement {
            side: layout.anchor_side,
            gap: layout.anchor_gap,
            align: layout.anchor_align,
            flip: layout.anchor_flip,
        };
        let target_box = boxes[target.index()].clone();
        let own_box = boxes[slot.index()].clone();
        let (x, y) = anchor_position(placement, &target_box, &own_box, viewport);
        shift_layout(slot, boxes, x - own_box.x, y - own_box.y, nodes);
    }
}

fn find_target(
    nodes: &[Node],
    root: Slot,
    slot: Slot,
    target: WidgetId,
    parents: &HashMap<Slot, Slot>,
) -> Option<Slot> {
    let mut ancestor = parents.get(&slot).copied();
    while let Some(current) = ancestor {
        if let Some(found) = find_in_subtree(nodes, current, slot, target) {
            return Some(found);
        }
        if current == root {
            break;
        }
        ancestor = parents.get(&current).copied();
    }
    None
}

fn find_in_subtree(nodes: &[Node], slot: Slot, skip: Slot, target: WidgetId) -> Option<Slot> {
    if slot != skip && nodes[slot.index()].id == target {
        return Some(slot);
    }
    nodes[slot.index()]
        .children
        .iter()
        .copied()
        .find_map(|child| find_in_subtree(nodes, child, skip, target))
}

fn shift_layout(slot: Slot, boxes: &mut [LayoutBox], dx: i32, dy: i32, nodes: &[Node]) {
    let own = &mut boxes[slot.index()];
    own.x += dx;
    own.y += dy;
    for &child in &nodes[slot.index()].children {
        if nodes[child.index()].layer {
            continue;
        }
        shift_layout(child, boxes, dx, dy, nodes);
    }
}

fn anchor_position(
    placement: Placement,
    target: &LayoutBox,
    own: &LayoutBox,
    viewport: Size,
) -> (i32, i32) {
    let point = anchor_point(placement, target, own);
    if placement.flip && overflows(point, own, viewport) {
        let flipped = anchor_point(
            Placement {
                side: placement.side.opposite(),
                ..placement
            },
            target,
            own,
        );
        return clamp_point(flipped, own, viewport);
    }
    clamp_point(point, own, viewport)
}

fn anchor_point(placement: Placement, target: &LayoutBox, own: &LayoutBox) -> (i32, i32) {
    let x = i64::from(target.x);
    let y = i64::from(target.y);
    let width = i64::from(target.width);
    let height = i64::from(target.height);
    let own_width = i64::from(own.width);
    let own_height = i64::from(own.height);
    let gap = i64::from(placement.gap);
    let (x, y) = match placement.side {
        AnchorSide::Top => (
            aligned(x, width, own_width, placement.align),
            y - gap - own_height,
        ),
        AnchorSide::Bottom => (
            aligned(x, width, own_width, placement.align),
            y + height + gap,
        ),
        AnchorSide::Left => (
            x - gap - own_width,
            aligned(y, height, own_height, placement.align),
        ),
        AnchorSide::Right => (
            x + width + gap,
            aligned(y, height, own_height, placement.align),
        ),
    };
    (clamp_i32(x), clamp_i32(y))
}

fn aligned(origin: i64, length: i64, own: i64, align: AnchorAlign) -> i64 {
    match align {
        AnchorAlign::Start => origin,
        AnchorAlign::Center => origin + (length - own) / 2,
        AnchorAlign::End => origin + length - own,
    }
}

fn overflows(point: (i32, i32), own: &LayoutBox, viewport: Size) -> bool {
    point.0 < 0
        || point.1 < 0
        || i64::from(point.0) + i64::from(own.width) > i64::from(viewport.width)
        || i64::from(point.1) + i64::from(own.height) > i64::from(viewport.height)
}

fn clamp_point(point: (i32, i32), own: &LayoutBox, viewport: Size) -> (i32, i32) {
    let max_x = i64::from(viewport.width) - i64::from(own.width);
    let max_y = i64::from(viewport.height) - i64::from(own.height);
    (
        clamp_i32(i64::from(point.0).clamp(0, max_x.max(0))),
        clamp_i32(i64::from(point.1).clamp(0, max_y.max(0))),
    )
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
