use std::collections::HashMap;
use std::rc::Rc;

use brul_gpu::{CLIP_INSTANCE_SIZE, ClipRange, INSTANCE_SIZE, TEXTURE_INSTANCE_SIZE};
use brul_gui::{Atlas, Dirty, Texture, WidgetId, WidgetSnapshot};

pub(crate) const CLIP: usize = 0;
pub(crate) const SHAPE: usize = 1;
pub(crate) const DECOR: usize = 2;
pub(crate) const GLYPH: usize = 3;
pub(crate) const IMAGE: usize = 4;
const SIZES: [u64; 5] = [
    CLIP_INSTANCE_SIZE,
    INSTANCE_SIZE,
    INSTANCE_SIZE,
    TEXTURE_INSTANCE_SIZE,
    TEXTURE_INSTANCE_SIZE,
];

#[derive(Clone, Copy, Default)]
struct Slot {
    start: usize,
    cap: usize,
}

#[derive(Clone, Copy, Default)]
struct Node {
    slots: [Slot; 5],
}

#[derive(Default)]
pub(crate) struct FrameCache {
    buffers: [Vec<u8>; 5],
    nodes: HashMap<WidgetId, Node>,
    textures: HashMap<WidgetId, Rc<Texture>>,
    images: Vec<(u32, u32, Rc<Texture>)>,
}

impl FrameCache {
    pub(crate) fn buffer(&self, index: usize) -> &[u8] {
        &self.buffers[index]
    }

    pub(crate) fn images(&self) -> &[(u32, u32, Rc<Texture>)] {
        &self.images
    }

    pub(crate) fn update(&mut self, snapshot: &WidgetSnapshot, atlas: &Atlas, dirty: &Dirty) {
        let mut ids: Vec<WidgetId> = if dirty.full {
            snapshot.nodes.iter().map(|node| node.id).collect()
        } else {
            dirty
                .ids
                .iter()
                .copied()
                .filter(|id| snapshot.index.contains_key(id))
                .collect()
        };
        ids.extend(
            snapshot
                .nodes
                .iter()
                .map(|node| node.id)
                .filter(|id| !self.nodes.contains_key(id)),
        );
        ids.sort_by_key(|id| snapshot.index.get(id).copied().unwrap_or(usize::MAX));
        ids.dedup();

        for id in ids {
            let Some(&index) = snapshot.index.get(&id) else {
                continue;
            };
            let one = Dirty {
                full: false,
                ids: vec![id],
            };
            let set = brul_gpu::clip_set(snapshot, &one);
            let mut shapes = brul_gpu::shape_instances(snapshot, &set, &one);
            let mut decor = brul_gpu::decoration_instances(snapshot, &set, &one);
            let mut textures = brul_gpu::texture_instances(snapshot, atlas, &set, &one);
            let current = self.nodes.get(&id).copied().unwrap_or_default();
            let range = ClipRange {
                offset: (current.slots[CLIP].start / CLIP_INSTANCE_SIZE as usize) as u32,
                count: set.ranges[index].count,
            };
            for shape in shapes.iter_mut().chain(decor.iter_mut()) {
                shape.padding[1] = range.offset as f32;
                shape.padding[2] = range.count as f32;
            }
            for instance in textures.glyphs.iter_mut().chain(
                textures
                    .images
                    .iter_mut()
                    .flat_map(|batch| batch.instances.iter_mut()),
            ) {
                instance.params[2] = range.offset as f32;
                instance.params[3] = range.count as f32;
            }
            let image = textures.images.into_iter().next().map(|batch| {
                (
                    brul_gpu::texture_instance_bytes(&batch.instances),
                    Rc::clone(&batch.texture),
                )
            });
            let image_bytes = image.as_ref().map_or_else(Vec::new, |part| part.0.clone());
            let parts = [
                brul_gpu::clip_bytes(&set.clips),
                brul_gpu::instance_bytes(&shapes),
                brul_gpu::instance_bytes(&decor),
                brul_gpu::texture_instance_bytes(&textures.glyphs),
                image_bytes,
            ];
            let mut next = current;
            for (buffer, bytes) in parts.iter().enumerate() {
                let slot = self.write(buffer, current.slots[buffer], bytes);
                next.slots[buffer] = slot;
            }
            if let Some((_, texture)) = image {
                self.textures.insert(id, texture);
            } else {
                self.textures.remove(&id);
            }
            self.nodes.insert(id, next);
        }

        let stale: Vec<WidgetId> = self
            .nodes
            .keys()
            .copied()
            .filter(|id| !snapshot.index.contains_key(id))
            .collect();
        for id in stale {
            let Some(node) = self.nodes.remove(&id) else {
                continue;
            };
            self.textures.remove(&id);
            for (buffer, slot) in node.slots.into_iter().enumerate() {
                self.write(buffer, slot, &[]);
            }
        }

        let images: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|(&id, node)| {
                let texture = self.textures.get(&id)?;
                (node.slots[IMAGE].cap > 0).then(|| {
                    (
                        node.slots[IMAGE].start as u32,
                        node.slots[IMAGE].cap as u32,
                        Rc::clone(texture),
                    )
                })
            })
            .collect();
        self.images = images;
    }

    fn write(&mut self, buffer: usize, slot: Slot, bytes: &[u8]) -> Slot {
        if slot.cap == 0 && bytes.is_empty() {
            return slot;
        }
        let size = SIZES[buffer] as usize;
        let needed = bytes.len().div_ceil(size);
        let target = &mut self.buffers[buffer];
        if needed <= slot.cap {
            let end = slot.start + bytes.len();
            let unchanged = target[slot.start..end].iter().eq(bytes.iter())
                && target[end..slot.start + slot.cap * size]
                    .iter()
                    .all(|&byte| byte == 0);
            if unchanged {
                return slot;
            }
            let mut padded = bytes.to_vec();
            padded.resize(slot.cap * size, 0);
            target[slot.start..slot.start + padded.len()].copy_from_slice(&padded);
            slot
        } else {
            let cap = needed.max(slot.cap * 2).max(1);
            let start = target.len();
            target.resize(start + cap * size, 0);
            let mut padded = bytes.to_vec();
            padded.resize(cap * size, 0);
            target[start..].copy_from_slice(&padded);
            if slot.cap > 0 {
                let zeros = vec![0u8; slot.cap * size];
                target[slot.start..slot.start + zeros.len()].copy_from_slice(&zeros);
            }
            Slot { start, cap }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::{
        ClipRect, Color, Dimension, LayoutStyle, Paint, Shadow, Size, StyleSet, TextEngine, Widget,
        WidgetStyle, WidgetTree,
    };

    fn snapshot() -> WidgetSnapshot {
        let solid = |color| {
            Widget::column(Vec::new())
                .with_layout(LayoutStyle {
                    width: Dimension::Px(10),
                    height: Dimension::Px(10),
                    ..LayoutStyle::default()
                })
                .with_styles(StyleSet::new(WidgetStyle {
                    background: Paint::Color(color),
                    ..WidgetStyle::default()
                }))
        };
        let mut engine = TextEngine::new();
        let mut tree = WidgetTree::new(Widget::column(vec![
            solid(Color::rgb(255, 0, 0)),
            solid(Color::rgb(0, 255, 0)),
        ]));
        tree.layout(
            &mut engine,
            Size {
                width: 64,
                height: 64,
            },
            1.0,
        )
        .expect("layout");
        tree.snapshot().expect("snapshot").clone()
    }

    #[test]
    fn a_full_update_keeps_absolute_clip_offsets() {
        let mut snapshot = snapshot();
        snapshot.nodes[1].clips = vec![ClipRect::new(1, 2, 3, 4, 0)];
        let engine = TextEngine::new();
        let mut cache = FrameCache::default();

        cache.update(&snapshot, engine.atlas(), &Dirty::all());

        let id = snapshot.nodes[1].id;
        let one = Dirty {
            full: false,
            ids: vec![id],
        };
        let clips = brul_gpu::clip_set(&snapshot, &one);
        let shapes = brul_gpu::shape_instances(&snapshot, &clips, &one);
        let slot = cache.nodes[&id].slots[SHAPE];
        let bytes = &cache.buffer(SHAPE)[slot.start..slot.start + INSTANCE_SIZE as usize];
        assert_eq!(bytes, brul_gpu::instance_bytes(&shapes[..1]));
        let clip_slot = cache.nodes[&id].slots[CLIP];
        assert_eq!(clip_slot.start, 0);
    }

    #[test]
    fn a_partial_update_leaves_other_nodes_bytes_alone() {
        let before = snapshot();
        let mut after = before.clone();
        after.nodes[1].style.background = Paint::Color(Color::rgb(0, 0, 255));
        let engine = TextEngine::new();
        let mut cache = FrameCache::default();
        cache.update(&before, engine.atlas(), &Dirty::all());
        let other = cache.nodes[&before.nodes[0].id].slots[SHAPE];
        let other_bytes = cache.buffer(SHAPE)
            [other.start..other.start + other.cap * INSTANCE_SIZE as usize]
            .to_vec();
        let clips = cache.buffer(CLIP).to_vec();

        cache.update(
            &after,
            engine.atlas(),
            &Dirty {
                full: false,
                ids: vec![after.nodes[1].id],
            },
        );

        let other_after = cache.nodes[&before.nodes[0].id].slots[SHAPE];
        assert_eq!(other_after.start, other.start);
        assert_eq!(
            &cache.buffer(SHAPE)[other.start..other.start + other.cap * INSTANCE_SIZE as usize],
            other_bytes.as_slice()
        );
        assert_eq!(cache.buffer(CLIP), clips.as_slice());
    }

    #[test]
    fn a_grown_slot_does_not_move_its_neighbours() {
        let before = snapshot();
        let mut after = before.clone();
        after.nodes[1].style.shadow = Shadow {
            color: Color::rgba(0, 0, 0, 128),
            blur: 1,
            ..Shadow::default()
        };
        let engine = TextEngine::new();
        let mut cache = FrameCache::default();
        cache.update(&before, engine.atlas(), &Dirty::all());
        let neighbour = cache.nodes[&before.nodes[0].id].slots;
        let clip = neighbour[CLIP];

        cache.update(
            &after,
            engine.atlas(),
            &Dirty {
                full: false,
                ids: vec![after.nodes[1].id],
            },
        );

        let grown = cache.nodes[&after.nodes[1].id].slots[SHAPE];
        assert_eq!(grown.cap, 2);
        assert!(grown.start > neighbour[SHAPE].start);
        let neighbour_after = cache.nodes[&before.nodes[0].id].slots;
        assert_eq!(neighbour_after[SHAPE].start, neighbour[SHAPE].start);
        assert_eq!(neighbour_after[CLIP].start, clip.start);
    }
}
