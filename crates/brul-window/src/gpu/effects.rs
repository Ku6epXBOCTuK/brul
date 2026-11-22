use std::collections::HashMap;

use brul_gpu::effect::{EffectDef, EffectError, EffectInstance, Effects};
use brul_gui::WidgetSnapshot;
use wgpu::{
    BindGroup, BindGroupLayout, Device, RenderPipeline, Sampler, Texture, TextureFormat,
    TextureView,
};

use super::device::{premultiplied_blend, unorm_format};
use crate::surface::FrameInfo;

pub(crate) struct EffectPipeline {
    pub main: RenderPipeline,
    pub advance: Option<RenderPipeline>,
}

pub(crate) struct EffectRange {
    pub name: &'static str,
    pub start: u32,
    pub count: u32,
}

pub(crate) struct EffectTargets {
    pub backdrop: Texture,
    pub backdrop_view: TextureView,
    pub history: [Texture; 2],
    pub history_views: [TextureView; 2],
}

pub(crate) fn history_format(format: TextureFormat) -> TextureFormat {
    unorm_format(format)
}

pub(crate) fn targets(
    device: &Device,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> EffectTargets {
    let backdrop = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("brul-effect-backdrop"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let backdrop_view = backdrop.create_view(&wgpu::TextureViewDescriptor::default());
    let history_format = history_format(format);
    let mut history = Vec::with_capacity(2);
    let mut history_views = Vec::with_capacity(2);
    for index in 0..2 {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(if index == 0 {
                "brul-effect-history-a"
            } else {
                "brul-effect-history-b"
            }),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: history_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        history_views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        history.push(texture);
    }
    EffectTargets {
        backdrop,
        backdrop_view,
        history: [history.remove(0), history.remove(0)],
        history_views: [history_views.remove(0), history_views.remove(0)],
    }
}

pub(crate) fn pipeline(
    device: &Device,
    empty_layout: &BindGroupLayout,
    effect_layout: &BindGroupLayout,
    def: &EffectDef,
    format: TextureFormat,
) -> Result<EffectPipeline, EffectError> {
    let shader = def.compile()?;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(def.name),
        source: wgpu::ShaderSource::Wgsl(shader.source().to_owned().into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("brul-effect-pipeline-layout"),
        bind_group_layouts: &[Some(empty_layout), Some(effect_layout)],
        immediate_size: 0,
    });
    let main = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("brul-effect-main"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("effect_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("effect_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(premultiplied_blend()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let advance = shader.has_advance().then(|| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("brul-effect-advance"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("effect_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("effect_advance_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: history_format(format),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    });
    Ok(EffectPipeline { main, advance })
}

pub(crate) fn bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    uniform: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    backdrop: &TextureView,
    history: &TextureView,
    sampler: &Sampler,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("brul-effect-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: instances,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(backdrop),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(history),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub(crate) fn instances(
    snapshot: &WidgetSnapshot,
    effects: &Effects,
    info: FrameInfo,
) -> (Vec<EffectInstance>, Vec<EffectRange>) {
    let mut grouped: HashMap<&'static str, Vec<EffectInstance>> = HashMap::new();
    for node in &snapshot.nodes {
        let Some(spec) = &node.effect else {
            continue;
        };
        if effects.get(spec.name).is_none() || node.width == 0 || node.height == 0 {
            continue;
        }
        grouped.entry(spec.name).or_default().push(EffectInstance {
            bounds: [
                node.x as f32,
                node.y as f32,
                node.width as f32,
                node.height as f32,
            ],
            uv: [0.0, 0.0, 1.0, 1.0],
            params: spec.params,
            meta: [info.time, 1.0, 1.0, 0.0],
            flags: [0; 4],
        });
    }
    let mut result = Vec::new();
    let mut ranges = Vec::new();
    for def in effects.definitions() {
        let Some(group) = grouped.remove(def.name) else {
            continue;
        };
        let start = result.len() as u32;
        let count = group.len() as u32;
        result.extend(group);
        ranges.push(EffectRange {
            name: def.name,
            start,
            count,
        });
    }
    (result, ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::{EffectSpec, Size, TextEngine, Widget, WidgetTree};

    fn spec(name: &'static str) -> EffectSpec {
        EffectSpec {
            name,
            params: [[1.0, 2.0, 3.0, 4.0]; 16],
            animated: false,
        }
    }

    #[test]
    fn effect_instances_are_grouped_by_registered_name() {
        let first = Widget::text("first").effect(spec("a"));
        let second = Widget::text("second").effect(spec("b"));
        let third = Widget::text("third").effect(spec("a"));
        let mut tree = WidgetTree::new(Widget::column(vec![first, second, third]));
        let mut engine = TextEngine::new();
        tree.layout(
            &mut engine,
            Size {
                width: 40,
                height: 40,
            },
            1.0,
        )
        .expect("layout");
        let snapshot = tree.snapshot().expect("snapshot");
        static DEFINITIONS: &[EffectDef] = &[
            EffectDef {
                name: "b",
                shader: "",
                animated: false,
                slots: 1,
                fields: &[],
            },
            EffectDef {
                name: "a",
                shader: "",
                animated: false,
                slots: 1,
                fields: &[],
            },
        ];
        let effects = Effects::new(DEFINITIONS);
        let info = FrameInfo {
            time: 1.5,
            delta: 0.016,
            frame: 7,
        };
        let (instances, ranges) = instances(snapshot, &effects, info);

        assert_eq!(instances.len(), 3);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].name, "b");
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].count, 1);
        assert_eq!(ranges[1].name, "a");
        assert_eq!(ranges[1].start, 1);
        assert_eq!(ranges[1].count, 2);
        assert_eq!(instances[0].params[0], [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(instances[0].meta[0], 1.5);
        assert!(instances.iter().all(|item| item.bounds[2] > 0.0));
    }
}
