use brul_gui::{Color, Paint, WidgetSnapshot};

pub const SHADER: &str = include_str!("shader.wgsl");
pub const INSTANCE_SIZE: u64 = size_of::<ShapeInstance>() as u64;
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

pub fn shape_instances(snapshot: &WidgetSnapshot) -> Vec<ShapeInstance> {
    let mut shapes = Vec::new();
    for node in &snapshot.nodes {
        if node.width == 0 || node.height == 0 {
            continue;
        }
        let opacity = if node.style.opacity.is_finite() {
            node.style.opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        if opacity <= 0.0 {
            continue;
        }

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

fn rgba(color: Color) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ]
}
