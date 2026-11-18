use std::rc::Rc;

use brul_gui::{
    Atlas, AtlasFormat, Color, Image, Paint, Texture, WidgetKind, WidgetSnapshot,
    WidgetSnapshotNode,
};

pub const SHADER: &str = include_str!("shader.wgsl");
pub const INSTANCE_SIZE: u64 = size_of::<ShapeInstance>() as u64;
pub const TEXTURE_INSTANCE_SIZE: u64 = size_of::<TextureInstance>() as u64;
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

pub fn shape_instances(snapshot: &WidgetSnapshot) -> Vec<ShapeInstance> {
    let mut shapes = Vec::new();
    for node in &snapshot.nodes {
        if node.width == 0 || node.height == 0 {
            continue;
        }
        let Some(opacity) = node_opacity(node) else {
            continue;
        };

        let bounds = [
            node.x as f32,
            node.y as f32,
            node.width as f32,
            node.height as f32,
        ];
        let shadow = node.style.shadow;
        if shadow.color.a > 0 {
            shapes.push(ShapeInstance {
                bounds,
                params: [
                    node.style.border_radius as f32,
                    0.0,
                    shadow.blur as f32,
                    opacity,
                ],
                fill: rgba(shadow.color),
                border: [0.0; 4],
                from: [0.0; 4],
                to: [0.0; 4],
                extra: [0.0, shadow.x as f32, shadow.y as f32, 2.0],
                padding: [shadow.blur as f32 + 1.0, 0.0, 0.0, 0.0],
            });
        }

        let (fill, from, to, angle) = match node.style.background {
            Paint::Color(color) => (rgba(color), [0.0; 4], [0.0; 4], 0.0),
            Paint::Gradient(gradient) => match gradient {
                brul_gui::Gradient::Linear { angle, from, to } => {
                    ([0.0; 4], rgba(from), rgba(to), angle)
                }
            },
        };
        if fill[3] > 0.0 || (angle != 0.0 && from[3] > 0.0) {
            shapes.push(ShapeInstance {
                bounds,
                params: [
                    node.style.border_radius as f32,
                    node.border_width as f32,
                    0.0,
                    opacity,
                ],
                fill,
                border: rgba(node.style.border),
                from,
                to,
                extra: [angle, 0.0, 0.0, 0.0],
                padding: [1.0, 0.0, 0.0, 0.0],
            });
        }
    }
    shapes
}

pub fn texture_instances(snapshot: &WidgetSnapshot, atlas: &Atlas) -> TextureBatches {
    let mut glyphs = Vec::new();
    let mut images = Vec::new();
    for node in &snapshot.nodes {
        if node.width == 0 || node.height == 0 {
            continue;
        }
        let Some(opacity) = node_opacity(node) else {
            continue;
        };
        match node.kind {
            WidgetKind::Text => {
                if let Some(text) = &node.text {
                    push_glyphs(node, text, 0, atlas, opacity, &mut glyphs);
                }
            }
            WidgetKind::Input => {
                if let Some(text) = &node.text {
                    push_glyphs(node, text, 0, atlas, opacity, &mut glyphs);
                }
                if let Some(preedit) = &node.preedit {
                    let shift = node
                        .caret
                        .map_or(0, |caret| caret.x.saturating_sub(preedit.size.width as i32));
                    push_glyphs(node, preedit, shift, atlas, opacity, &mut glyphs);
                }
            }
            WidgetKind::Image => {
                if let Some(image) = &node.image {
                    push_image(node, image, opacity, &mut images);
                }
            }
            WidgetKind::View | WidgetKind::Structure => {}
        }
    }
    TextureBatches { glyphs, images }
}

fn push_glyphs(
    node: &WidgetSnapshotNode,
    text: &brul_gui::TextLayout,
    shift: i32,
    atlas: &Atlas,
    opacity: f32,
    out: &mut Vec<TextureInstance>,
) {
    let offset = node
        .style
        .vertical_align
        .offset(node.content_height, text.size.height) as i32
        - node.scroll;
    for glyph in &text.glyphs {
        let slot = glyph.slot;
        if slot.width == 0 || slot.height == 0 {
            continue;
        }
        let x = node.content_x.saturating_add(shift).saturating_add(glyph.x);
        let y = node
            .content_y
            .saturating_add(glyph.y)
            .saturating_add(offset);
        let (tint, mode) = match slot.format {
            AtlasFormat::Mask => (rgba(node.style.text), 0.0),
            AtlasFormat::Color => ([1.0; 4], 1.0),
        };
        out.push(TextureInstance {
            bounds: [x as f32, y as f32, slot.width as f32, slot.height as f32],
            uv: [
                slot.x as f32 / atlas.width() as f32,
                slot.y as f32 / atlas.height() as f32,
                (slot.x + slot.width) as f32 / atlas.width() as f32,
                (slot.y + slot.height) as f32 / atlas.height() as f32,
            ],
            tint,
            params: [mode, opacity, 0.0, 0.0],
        });
    }
}

fn push_image(node: &WidgetSnapshotNode, image: &Image, opacity: f32, out: &mut Vec<TextureBatch>) {
    if node.content_width == 0 || node.content_height == 0 {
        return;
    }
    let (texture, tint) = match image {
        Image::Raster(texture) => {
            let tint = match (texture.tint(), node.style.tint) {
                (Some(texture), Some(style)) => Some(texture.multiply(style)),
                (Some(texture), None) => Some(texture),
                (None, Some(style)) => Some(style),
                (None, None) => None,
            };
            (Rc::clone(texture), tint)
        }
        Image::Vector(vector) => {
            let current = node.style.tint.unwrap_or(node.style.text);
            let Some(texture) = vector.raster(node.content_width, node.content_height, current)
            else {
                return;
            };
            (texture, None)
        }
    };
    let batch = match out
        .iter_mut()
        .find(|batch| Rc::as_ptr(&batch.texture) == Rc::as_ptr(&texture))
    {
        Some(batch) => batch,
        None => {
            out.push(TextureBatch {
                texture: Rc::clone(&texture),
                instances: Vec::new(),
            });
            out.last_mut().expect("batch")
        }
    };
    batch.instances.push(TextureInstance {
        bounds: [
            node.content_x as f32,
            node.content_y as f32,
            node.content_width as f32,
            node.content_height as f32,
        ],
        uv: [0.0, 0.0, 1.0, 1.0],
        tint: tint.map(rgba).unwrap_or([1.0; 4]),
        params: [1.0, opacity, 0.0, 0.0],
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

fn node_opacity(node: &WidgetSnapshotNode) -> Option<f32> {
    let opacity = if node.style.opacity.is_finite() {
        node.style.opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    (opacity > 0.0).then_some(opacity)
}

fn rgba(color: Color) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ]
}
