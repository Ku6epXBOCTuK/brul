use super::frame::{build_snapshot, reveal_carets};
use super::render::{Pass, measure_text};
use super::style::Transition;
use super::*;
use crate::layout::{LayoutBox, Size};
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
