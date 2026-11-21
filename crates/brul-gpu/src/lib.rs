use std::rc::Rc;

use brul_gui::{
    Atlas, Color, Dirty, DrawCommand, Gradient, Paint, Texture, WidgetId, WidgetSnapshot,
    draw_commands,
};

pub const SHADER: &str = include_str!("shader.wgsl");
pub const INSTANCE_SIZE: u64 = size_of::<ShapeInstance>() as u64;
pub const TEXTURE_INSTANCE_SIZE: u64 = size_of::<TextureInstance>() as u64;
pub const CLIP_INSTANCE_SIZE: u64 = size_of::<ClipInstance>() as u64;
pub fn instance_usage() -> wgpu::BufferUsages {
    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ShapeInstance {
    pub bounds: [f32; 4],
    pub params: [f32; 4],
    pub fill: [f32; 4],
    pub border: [f32; 4],
    pub border_widths: [f32; 4],
    pub from: [f32; 4],
    pub to: [f32; 4],
    pub extra: [f32; 4],
    pub padding: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct TextureInstance {
    pub bounds: [f32; 4],
    pub uv: [f32; 4],
    pub tint: [f32; 4],
    pub params: [f32; 4],
}

pub struct TextureBatch {
    pub texture: Rc<Texture>,
    pub instances: Vec<TextureInstance>,
}

pub struct TextureBatches {
    pub glyphs: Vec<TextureInstance>,
    pub images: Vec<TextureBatch>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ClipInstance {
    pub bounds: [f32; 4],
    pub radius: [f32; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ClipRange {
    pub offset: u32,
    pub count: u32,
}

#[derive(Debug, Default)]
pub struct ClipSet {
    pub clips: Vec<ClipInstance>,
    pub ranges: Vec<ClipRange>,
}

pub fn clip_set(snapshot: &WidgetSnapshot, dirty: &Dirty) -> ClipSet {
    let mut set = ClipSet {
        ranges: vec![ClipRange::default(); snapshot.nodes.len()],
        ..ClipSet::default()
    };
    for (index, node) in snapshot.nodes.iter().enumerate() {
        if !node_is_dirty(dirty, node.id) {
            continue;
        }
        let offset = set.clips.len() as u32;
        for clip in &node.clips {
            set.clips.push(ClipInstance {
                bounds: [
                    clip.x as f32,
                    clip.y as f32,
                    clip.width as f32,
                    clip.height as f32,
                ],
                radius: [clip.radius as f32, 0.0, 0.0, 0.0],
            });
        }
        let count = set.clips.len() as u32 - offset;
        set.ranges[index] = ClipRange { offset, count };
    }
    set
}

pub fn clip_bytes(clips: &[ClipInstance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(clips.len() * CLIP_INSTANCE_SIZE as usize);
    for clip in clips {
        for field in [clip.bounds, clip.radius] {
            for value in field {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
    }
    bytes
}

pub fn shape_instances(
    snapshot: &WidgetSnapshot,
    clips: &ClipSet,
    dirty: &Dirty,
) -> Vec<ShapeInstance> {
    let mut commands = Vec::new();
    draw_commands(snapshot, dirty, &mut commands);
    let mut shapes = Vec::new();
    for command in commands {
        let DrawCommand::Shape(shape) = command else {
            continue;
        };
        let rect = shape.rect;
        let bounds = [
            rect.x as f32,
            rect.y as f32,
            rect.width as f32,
            rect.height as f32,
        ];
        let clip = clip_range(clips, shape.node);
        if let Some(shadow) = shape.shadow {
            shapes.push(ShapeInstance {
                bounds,
                params: [rect.radius as f32, 0.0, shadow.blur as f32, shape.opacity],
                fill: rgba(shadow.color),
                border: [0.0; 4],
                border_widths: [0.0; 4],
                from: [0.0; 4],
                to: [0.0; 4],
                extra: [0.0, shadow.x as f32, shadow.y as f32, 2.0],
                padding: [
                    shadow.blur as f32 + 1.0,
                    clip.offset as f32,
                    clip.count as f32,
                    0.0,
                ],
            });
        }
        if !shape.draw_fill {
            continue;
        }
        let (fill, from, to, angle) = match rect.fill {
            Paint::Color(color) => (rgba(color), [0.0; 4], [0.0; 4], 0.0),
            Paint::Gradient(Gradient::Linear { angle, from, to }) => {
                ([0.0; 4], rgba(from), rgba(to), angle)
            }
        };
        shapes.push(ShapeInstance {
            bounds,
            params: [rect.radius as f32, 0.0, 0.0, shape.opacity],
            fill,
            border: rgba(rect.border_color),
            border_widths: [
                rect.border_width.left as f32,
                rect.border_width.top as f32,
                rect.border_width.right as f32,
                rect.border_width.bottom as f32,
            ],
            from,
            to,
            extra: [angle, 0.0, 0.0, 0.0],
            padding: [1.0, clip.offset as f32, clip.count as f32, 0.0],
        });
    }
    shapes
}

pub fn texture_instances(
    snapshot: &WidgetSnapshot,
    atlas: &Atlas,
    clips: &ClipSet,
    dirty: &Dirty,
) -> TextureBatches {
    let mut commands = Vec::new();
    draw_commands(snapshot, dirty, &mut commands);
    let mut glyphs = Vec::new();
    let mut images: Vec<TextureBatch> = Vec::new();
    for command in commands {
        match command {
            DrawCommand::Glyph(glyph) => {
                let slot = glyph.slot;
                let (tint, mode) = match glyph.tint {
                    Some(color) => (rgba(color), 0.0),
                    None => ([1.0; 4], 1.0),
                };
                let clip = clip_range(clips, glyph.node);
                glyphs.push(TextureInstance {
                    bounds: [
                        glyph.x as f32,
                        glyph.y as f32,
                        slot.width as f32,
                        slot.height as f32,
                    ],
                    uv: [
                        slot.x as f32 / atlas.width() as f32,
                        slot.y as f32 / atlas.height() as f32,
                        (slot.x + slot.width) as f32 / atlas.width() as f32,
                        (slot.y + slot.height) as f32 / atlas.height() as f32,
                    ],
                    tint,
                    params: [mode, glyph.opacity, clip.offset as f32, clip.count as f32],
                });
            }
            DrawCommand::Image(image) => {
                let clip = clip_range(clips, image.node);
                let batch = match images
                    .iter_mut()
                    .find(|batch| Rc::ptr_eq(&batch.texture, &image.texture))
                {
                    Some(batch) => batch,
                    None => {
                        images.push(TextureBatch {
                            texture: Rc::clone(&image.texture),
                            instances: Vec::new(),
                        });
                        images.last_mut().expect("batch")
                    }
                };
                batch.instances.push(TextureInstance {
                    bounds: image.rect.map(|value| value as f32),
                    uv: [0.0, 0.0, 1.0, 1.0],
                    tint: image.tint.map(rgba).unwrap_or([1.0; 4]),
                    params: [1.0, image.opacity, clip.offset as f32, clip.count as f32],
                });
            }
            DrawCommand::Shape(_) | DrawCommand::Rect(_) => {}
        }
    }
    TextureBatches { glyphs, images }
}

pub fn decoration_instances(
    snapshot: &WidgetSnapshot,
    clips: &ClipSet,
    dirty: &Dirty,
) -> Vec<ShapeInstance> {
    let mut commands = Vec::new();
    draw_commands(snapshot, dirty, &mut commands);
    let mut shapes = Vec::new();
    for command in commands {
        let DrawCommand::Rect(decoration) = command else {
            continue;
        };
        push_decoration(
            &mut shapes,
            decoration.rect,
            decoration.color,
            decoration.opacity,
            clip_range(clips, decoration.node),
        );
    }
    shapes
}

fn push_decoration(
    out: &mut Vec<ShapeInstance>,
    rect: [i32; 4],
    color: Color,
    opacity: f32,
    clip: ClipRange,
) {
    if rect[2] <= 0 || rect[3] <= 0 {
        return;
    }
    out.push(ShapeInstance {
        bounds: [
            rect[0] as f32,
            rect[1] as f32,
            rect[2] as f32,
            rect[3] as f32,
        ],
        params: [0.0, 0.0, 0.0, opacity],
        fill: rgba(color),
        border: [0.0; 4],
        border_widths: [0.0; 4],
        from: [0.0; 4],
        to: [0.0; 4],
        extra: [0.0; 4],
        padding: [0.0, clip.offset as f32, clip.count as f32, 0.0],
    });
}

pub fn instance_bytes(instances: &[ShapeInstance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * INSTANCE_SIZE as usize);
    for shape in instances {
        for field in [
            shape.bounds,
            shape.params,
            shape.fill,
            shape.border,
            shape.border_widths,
            shape.from,
            shape.to,
            shape.extra,
            shape.padding,
        ] {
            for value in field {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
    }
    bytes
}

pub fn texture_instance_bytes(instances: &[TextureInstance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * TEXTURE_INSTANCE_SIZE as usize);
    for instance in instances {
        for field in [instance.bounds, instance.uv, instance.tint, instance.params] {
            for value in field {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
    }
    bytes
}

fn clip_range(clips: &ClipSet, index: usize) -> ClipRange {
    clips.ranges.get(index).copied().unwrap_or_default()
}

fn node_is_dirty(dirty: &Dirty, id: WidgetId) -> bool {
    dirty.full || dirty.ids.contains(&id)
}

fn rgba(color: Color) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ]
}
