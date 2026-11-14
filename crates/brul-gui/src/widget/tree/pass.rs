use super::frame::{build_snapshot, reveal_carets};
use super::render::{Pass, dirty_subtrees, flatten_boxes, move_children};
use super::style::Transition;
use super::*;
use crate::layout::{Available, LayoutBox, Size};
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

        let reuse = self
            .geometry
            .as_ref()
            .is_some_and(|geometry| geometry.viewport == viewport && geometry.scale == scale);
        let frame_changed = !reuse;
        if self.layout_pass(engine, viewport, scale, reuse, frame_changed)? {
            self.layout_pass(engine, viewport, scale, false, frame_changed)?;
        }
        Ok(())
    }

    fn layout_pass(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
        reuse: bool,
        frame_changed: bool,
    ) -> Result<bool, WidgetError> {
        let previous = self.geometry.take();
        let mut boxes = previous.map(|geometry| geometry.boxes).unwrap_or_default();
        boxes.resize(self.nodes.len(), LayoutBox::default());

        let (node, key, order, frozen) = {
            self.layouter.begin();
            let mut pass = Pass {
                tree: &self.nodes,
                engine,
                texts: &mut self.texts,
                layouter: &mut self.layouter,
                scale,
                cached: reuse.then_some(&boxes),
                blocked: dirty_subtrees(&self.nodes, self.root, &self.stale),
                order: Vec::with_capacity(self.nodes.len()),
                frozen: Vec::new(),
            };
            let (node, key) = pass.build(
                self.root,
                Available {
                    width: Some(viewport.width),
                    height: None,
                },
            )?;
            (node, key, pass.order, pass.frozen)
        };
        self.texts.retain(|id, _| self.index.contains_key(id));

        self.layouter.finish();
        self.layouter.compute(key, viewport);
        let placed = self.layouter.collect(key, &node);
        let mut cursor = 0;
        flatten_boxes(&placed, &order, &mut cursor, &mut boxes);

        let resized = frozen.iter().any(|(slot, cached)| {
            let placed = &boxes[slot.index()];
            placed.x != cached.x
                || placed.y != cached.y
                || placed.width != cached.width
                || placed.height != cached.height
        });
        if resized {
            return Ok(true);
        }
        for (slot, cached) in frozen {
            let placed = &boxes[slot.index()];
            let dx = placed.x - cached.x;
            let dy = placed.y - cached.y;
            if dx != 0 || dy != 0 {
                move_children(&self.nodes, slot, dx, dy, &mut boxes);
            }
        }

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
        Ok(false)
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

    pub fn snapshot(&mut self) -> Result<&WidgetSnapshot, WidgetError> {
        if self.snapshot.is_none() {
            let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
            let snapshot =
                build_snapshot(&self.nodes, self.root, geometry, &self.texts, self.focused);
            self.snapshot = Some(snapshot);
        }
        Ok(self.snapshot.as_ref().expect("snapshot built"))
    }

    pub fn snapshot_dirty(&self) -> bool {
        self.snapshot.is_none()
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
        self.index
            .values()
            .any(|slot| self.nodes[slot.index()].transition.is_some())
    }

    pub fn advance(&mut self, delta: Duration) -> bool {
        if delta.is_zero() {
            return self.animating();
        }
        let slots: Vec<Slot> = self.index.values().copied().collect();
        let mut running = false;
        let mut changed = false;
        for slot in slots {
            let node = &mut self.nodes[slot.index()];
            let Some(transition) = node.transition.as_mut() else {
                continue;
            };
            let active = transition.advance(delta);
            if !active {
                node.transition = None;
            }
            let id = node.id;
            running |= active;
            changed = true;
            self.mark_dirty(id);
        }
        if changed {
            self.snapshot = None;
        }
        running
    }

    pub(super) fn retarget(&mut self, slot: Slot) {
        let styles = self.nodes[slot.index()].styles();
        let node = &mut self.nodes[slot.index()];
        let durations = styles.transitions();
        let target = styles.resolve(node.state);
        let from = match node.transition.take() {
            Some(transition) if transition.to != target => transition.style(),
            Some(transition) => {
                node.transition = Some(transition);
                return;
            }
            None => node.shown,
        };
        node.shown = target;
        if durations.is_empty() {
            node.transition = None;
            return;
        }
        node.transition = (from != target).then(|| Transition::new(from, target, durations));
    }
}
