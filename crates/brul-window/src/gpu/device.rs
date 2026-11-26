use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use brul_gpu::effect::{EFFECT_FRAME_SIZE, EFFECT_INSTANCE_SIZE};
use winit::window::Window;

use crate::gpu_frame::FrameCache;
use crate::window::WindowError;

use super::effects;
use super::{Gpu, Textures};

fn clear_color(transparent: bool) -> wgpu::Color {
    if transparent {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    } else {
        crate::window::CLEAR
    }
}

pub(crate) fn premultiplied_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

fn size_change(current: (u32, u32), width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    (current != (width, height)).then_some((width, height))
}

pub(crate) fn unorm_format(format: wgpu::TextureFormat) -> wgpu::TextureFormat {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8Unorm,
        other => other,
    }
}

fn alpha_mode(capabilities: &wgpu::SurfaceCapabilities) -> wgpu::CompositeAlphaMode {
    for mode in [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::PostMultiplied,
        wgpu::CompositeAlphaMode::Inherit,
        wgpu::CompositeAlphaMode::Auto,
    ] {
        if capabilities.alpha_modes.contains(&mode) {
            return mode;
        }
    }
    capabilities
        .alpha_modes
        .first()
        .copied()
        .unwrap_or(wgpu::CompositeAlphaMode::Auto)
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

impl Gpu {
    pub(crate) fn new(window: &Arc<dyn Window>, transparent: bool) -> Result<Self, WindowError> {
        // NOTE: the GL backend parks a hidden window on a thread that pumps no messages, and a
        // keyboard layout switch then waits for it forever (wgpu#10504)
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(Arc::clone(window))
            .map_err(WindowError::gpu)?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(WindowError::gpu)?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(WindowError::gpu)?;

        let size = window.surface_size();
        let capabilities = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(WindowError::NoSurfaceConfig)?;
        config.alpha_mode = alpha_mode(&capabilities);
        let unorm = unorm_format(config.format);
        if capabilities.formats.contains(&unorm) {
            config.format = unorm;
        }
        config.desired_maximum_frame_latency = 1;
        let surface_copy = capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let wants_capture = crate::perf::dump_path().is_some();
        let capture = wants_capture && surface_copy;
        if wants_capture && !surface_copy {
            tracing::warn!("surface cannot be read back, frame dump is disabled");
        }
        let effects_enabled = surface_copy;
        if !effects_enabled {
            tracing::warn!("surface cannot be copied, GPU effects are disabled");
        }
        if surface_copy {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        surface.configure(&device, &config);
        if capture {
            tracing::info!("brul capture: surface format {:?}", config.format);
        }

        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("brul-nearest-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let shape_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brul-shape-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shape_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("brul-shape-pipeline-layout"),
                bind_group_layouts: &[Some(&shape_layout)],
                immediate_size: 0,
            });
        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("brul-shape-shader"),
            source: wgpu::ShaderSource::Wgsl(brul_gpu::SHADER.into()),
        });
        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("brul-shape-pipeline"),
            layout: Some(&shape_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
        let quad_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brul-quad-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("brul-quad-pipeline-layout"),
            bind_group_layouts: &[Some(&quad_layout)],
            immediate_size: 0,
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("brul-quad-pipeline"),
            layout: Some(&quad_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: Some("vs_quad"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: Some("fs_quad"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
        let viewport_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-shape-viewport"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-shape-instances"),
            size: brul_gpu::INSTANCE_SIZE,
            usage: brul_gpu::instance_usage(),
            mapped_at_creation: false,
        });
        let clip_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-clips"),
            size: brul_gpu::CLIP_INSTANCE_SIZE,
            usage: brul_gpu::instance_usage(),
            mapped_at_creation: false,
        });
        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-quad-instances"),
            size: brul_gpu::TEXTURE_INSTANCE_SIZE,
            usage: brul_gpu::instance_usage(),
            mapped_at_creation: false,
        });
        let effect_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brul-effect-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let effect_empty_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("brul-effect-empty-layout"),
                entries: &[],
            });
        let effect_empty_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brul-effect-empty-group"),
            layout: &effect_empty_layout,
            entries: &[],
        });
        let effect_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-effect-frame"),
            size: EFFECT_FRAME_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let effect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-effect-instances"),
            size: EFFECT_INSTANCE_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let effect_targets = effects::targets(&device, config.width, config.height, config.format);

        Ok(Self {
            clear_color: clear_color(transparent),
            _instance: instance,
            surface,
            device,
            queue,
            config,
            shape_layout,
            shape_pipeline,
            viewport_buffer,
            viewport: vec![0; 8],
            instance_buffer,
            instance_capacity: 0,
            clip_buffer,
            clip_capacity: brul_gpu::CLIP_INSTANCE_SIZE as usize,
            quad_pipeline,
            quad_buffer,
            quad_capacity: brul_gpu::TEXTURE_INSTANCE_SIZE as usize,
            textures: Textures::new(quad_layout, nearest_sampler.clone()),
            frame: FrameCache::default(),
            effects_enabled,
            effect_pipelines: HashMap::new(),
            effect_layout,
            effect_empty_layout,
            effect_empty_group,
            effect_sampler: nearest_sampler,
            effect_uniform_buffer,
            effect_uniform: vec![0; EFFECT_FRAME_SIZE as usize],
            effect_buffer,
            effect_capacity: EFFECT_INSTANCE_SIZE as usize,
            effect_backdrop: effect_targets.backdrop,
            effect_backdrop_view: effect_targets.backdrop_view,
            effect_history: effect_targets.history,
            effect_history_views: effect_targets.history_views,
            effect_history_write: 0,
            capture,
            captured: None,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let Some((width, height)) =
            size_change((self.config.width, self.config.height), width, height)
        else {
            return;
        };

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let targets = effects::targets(&self.device, width, height, self.config.format);
        self.effect_backdrop = targets.backdrop;
        self.effect_backdrop_view = targets.backdrop_view;
        self.effect_history = targets.history;
        self.effect_history_views = targets.history_views;
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
