use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use winit::window::Window;

use brul_gui::{Atlas, Dirty, Frame, Texture, WidgetSnapshot};

use crate::window::WindowError;

fn premultiplied_blend() -> wgpu::BlendState {
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

const BLIT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var coordinates = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = coordinates[index];
    return output;
}

@group(0) @binding(0) var frame_texture: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame_texture, frame_sampler, input.uv);
}
"#;

pub(crate) struct Gpu {
    _instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    blit_layout: wgpu::BindGroupLayout,
    blit_pipeline: wgpu::RenderPipeline,
    nearest_sampler: wgpu::Sampler,
    blit: Option<BlitTarget>,
    shape_layout: wgpu::BindGroupLayout,
    shape_pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    quad_layout: wgpu::BindGroupLayout,
    quad_pipeline: wgpu::RenderPipeline,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    atlas: Option<AtlasTexture>,
    images: HashMap<usize, ImageTexture>,
    frame_index: u64,
    frame: Frame,
}

struct BlitTarget {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

struct AtlasTexture {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    revision: u64,
    bind_group: Option<wgpu::BindGroup>,
}

struct ImageTexture {
    source: Rc<Texture>,
    texture: wgpu::Texture,
    bind_group: Option<wgpu::BindGroup>,
    uploaded: bool,
    last_frame: u64,
}

impl Gpu {
    pub(crate) fn new(window: &Arc<dyn Window>) -> Result<Self, WindowError> {
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
        config.desired_maximum_frame_latency = 1;
        surface.configure(&device, &config);

        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brul-blit-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("brul-blit-pipeline-layout"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("brul-blit-shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("brul-blit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
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
        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-quad-instances"),
            size: brul_gpu::TEXTURE_INSTANCE_SIZE,
            usage: brul_gpu::instance_usage(),
            mapped_at_creation: false,
        });

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            blit_layout,
            blit_pipeline,
            nearest_sampler,
            blit: None,
            shape_layout,
            shape_pipeline,
            viewport_buffer,
            instance_buffer,
            instance_capacity: 0,
            quad_layout,
            quad_pipeline,
            quad_buffer,
            quad_capacity: brul_gpu::TEXTURE_INSTANCE_SIZE as usize,
            atlas: None,
            images: HashMap::new(),
            frame_index: 0,
            frame: Frame::new(1, 1),
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
        self.blit = None;
    }

    pub(crate) fn clear_surface(&self, surface: &dyn crate::surface::Surface) -> bool {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                surface.redraw();
                return false;
            }
            _ => return false,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-window"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brul-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(crate::window::CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        surface.pre_present_notify();
        self.queue.present(frame);
        true
    }

    fn ensure_blit(&mut self, width: u32, height: u32) {
        if self
            .blit
            .as_ref()
            .is_some_and(|target| target.width == width && target.height == height)
        {
            return;
        }
        self.blit = Some(BlitTarget::new(
            &self.device,
            &self.blit_layout,
            &self.nearest_sampler,
            width,
            height,
        ));
    }

    pub(crate) fn render(
        &mut self,
        surface: &dyn crate::surface::Surface,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
    ) -> bool {
        let (width, height) = surface.size();
        self.frame.resize(width.max(1), height.max(1));
        self.frame.paint_content(snapshot, atlas, dirty);
        let shapes = brul_gpu::shape_instances(snapshot);
        let textures = brul_gpu::texture_instances(snapshot, atlas);
        self.present_frame(surface, atlas, &shapes, &textures)
    }

    fn present_frame(
        &mut self,
        surface: &dyn crate::surface::Surface,
        atlas: &Atlas,
        shapes: &[brul_gpu::ShapeInstance],
        textures: &brul_gpu::TextureBatches,
    ) -> bool {
        let width = self.frame.width().max(1);
        let height = self.frame.height().max(1);
        self.ensure_blit(width, height);
        if shapes.len() > self.instance_capacity {
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brul-shape-instances"),
                size: shapes.len() as u64 * brul_gpu::INSTANCE_SIZE,
                usage: brul_gpu::instance_usage(),
                mapped_at_creation: false,
            });
            self.instance_capacity = shapes.len();
        }
        let mut quads = Vec::with_capacity(
            textures.glyphs.len()
                + textures
                    .images
                    .iter()
                    .map(|batch| batch.instances.len())
                    .sum::<usize>(),
        );
        quads.extend_from_slice(&textures.glyphs);
        let mut image_ranges = Vec::with_capacity(textures.images.len());
        for batch in &textures.images {
            let start = quads.len() as u32;
            quads.extend_from_slice(&batch.instances);
            image_ranges.push((
                start,
                batch.instances.len() as u32,
                Rc::clone(&batch.texture),
            ));
        }
        if !quads.is_empty() {
            let bytes = brul_gpu::texture_instance_bytes(&quads);
            if bytes.len() > self.quad_capacity {
                self.quad_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("brul-quad-instances"),
                    size: bytes.len() as u64,
                    usage: brul_gpu::instance_usage(),
                    mapped_at_creation: false,
                });
                self.quad_capacity = bytes.len();
                if let Some(cache) = &mut self.atlas {
                    cache.bind_group = None;
                }
                for cache in self.images.values_mut() {
                    cache.bind_group = None;
                }
            }
            self.queue.write_buffer(&self.quad_buffer, 0, &bytes);
        }
        self.frame_index += 1;
        self.images
            .retain(|_, cache| cache.last_frame + 120 >= self.frame_index);
        if self.images.len() > 256 {
            let mut oldest: Vec<_> = self
                .images
                .iter()
                .map(|(&key, cache)| (cache.last_frame, key))
                .collect();
            oldest.sort_unstable();
            for (_, key) in oldest.into_iter().take(self.images.len() - 256) {
                self.images.remove(&key);
            }
        }
        let glyph_bind_group =
            (!textures.glyphs.is_empty()).then(|| self.ensure_atlas(atlas).clone());
        let mut image_bind_groups = Vec::with_capacity(image_ranges.len());
        for (_, _, texture) in &image_ranges {
            image_bind_groups.push(self.ensure_image(texture).clone());
        }

        let target = self.blit.as_ref().expect("blit target");
        if !shapes.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, &brul_gpu::instance_bytes(shapes));
        }
        let viewport = [
            f32::to_ne_bytes(width as f32),
            f32::to_ne_bytes(height as f32),
        ]
        .concat();
        self.queue.write_buffer(&self.viewport_buffer, 0, &viewport);
        let shape_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brul-shape-bind-group"),
            layout: &self.shape_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.viewport_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.instance_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.frame.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let blit_bind_group = target.bind_group.clone();

        let surface_frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                surface.redraw();
                return false;
            }
            _ => return false,
        };
        let view = surface_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-present"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("brul-present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(crate::window::CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !shapes.is_empty() {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &shape_bind_group, &[]);
                pass.draw(0..6, 0..shapes.len() as u32);
            }
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &blit_bind_group, &[]);
            pass.draw(0..3, 0..1);
            if !quads.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                if let Some(group) = &glyph_bind_group {
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..6, 0..textures.glyphs.len() as u32);
                }
                for ((start, count, _), group) in image_ranges.iter().zip(&image_bind_groups) {
                    if *count == 0 {
                        continue;
                    }
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..6, *start..*start + *count);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        surface.pre_present_notify();
        self.queue.present(surface_frame);
        true
    }

    fn ensure_atlas(&mut self, atlas: &Atlas) -> &wgpu::BindGroup {
        if self
            .atlas
            .as_ref()
            .is_none_or(|cache| cache.width != atlas.width() || cache.height != atlas.height())
        {
            self.atlas = Some(AtlasTexture {
                texture: create_texture(&self.device, atlas.width(), atlas.height()),
                width: atlas.width(),
                height: atlas.height(),
                revision: u64::MAX,
                bind_group: None,
            });
        }
        let cache = self.atlas.as_mut().expect("atlas texture");
        if cache.revision != atlas.revision() {
            upload_texture(
                &self.queue,
                &cache.texture,
                cache.width,
                cache.height,
                atlas.pixels(),
            );
            cache.revision = atlas.revision();
        }
        if cache.bind_group.is_none() {
            cache.bind_group = Some(quad_bind_group(
                &self.device,
                &self.quad_layout,
                &self.viewport_buffer,
                &self.quad_buffer,
                &self.nearest_sampler,
                &cache.texture,
            ));
        }
        cache.bind_group.as_ref().expect("atlas bind group")
    }

    fn ensure_image(&mut self, source: &Rc<Texture>) -> &wgpu::BindGroup {
        let key = Rc::as_ptr(source) as usize;
        let frame = self.frame_index;
        if !self.images.contains_key(&key) {
            let texture = create_texture(&self.device, source.width(), source.height());
            self.images.insert(
                key,
                ImageTexture {
                    source: Rc::clone(source),
                    texture,
                    bind_group: None,
                    uploaded: false,
                    last_frame: frame,
                },
            );
        }
        let cache = self.images.get_mut(&key).expect("image texture");
        if !cache.uploaded {
            let source = &cache.source;
            upload_texture(
                &self.queue,
                &cache.texture,
                source.width(),
                source.height(),
                source.pixels(),
            );
            cache.uploaded = true;
        }
        cache.last_frame = frame;
        if cache.bind_group.is_none() {
            cache.bind_group = Some(quad_bind_group(
                &self.device,
                &self.quad_layout,
                &self.viewport_buffer,
                &self.quad_buffer,
                &self.nearest_sampler,
                &cache.texture,
            ));
        }
        cache.bind_group.as_ref().expect("image bind group")
    }
}

fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("brul-quad-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn quad_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    viewport: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    texture: &wgpu::Texture,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("brul-quad-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: viewport,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: instances,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

impl BlitTarget {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("brul-frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brul-frame-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Self {
            texture,
            bind_group,
            width,
            height,
        }
    }
}

fn size_change(current: (u32, u32), width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    (current != (width, height)).then_some((width, height))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::ready;
    use std::task::Poll;
    use wgpu::{CompositeAlphaMode, TextureFormat, TextureUsages};

    fn capabilities(alpha_modes: Vec<CompositeAlphaMode>) -> wgpu::SurfaceCapabilities {
        wgpu::SurfaceCapabilities {
            formats: vec![TextureFormat::Rgba8UnormSrgb],
            format_capabilities: Vec::new(),
            present_modes: Vec::new(),
            alpha_modes,
            usages: TextureUsages::RENDER_ATTACHMENT,
        }
    }

    #[test]
    fn a_size_change_needs_a_size_that_differs() {
        assert_eq!(size_change((8, 6), 8, 6), None);
        assert_eq!(size_change((8, 6), 9, 6), Some((9, 6)));
        assert_eq!(size_change((8, 6), 8, 7), Some((8, 7)));
    }

    #[test]
    fn an_empty_size_is_not_a_size_change() {
        assert_eq!(size_change((8, 6), 0, 6), None);
        assert_eq!(size_change((8, 6), 8, 0), None);
    }

    #[test]
    fn the_first_preferred_alpha_mode_wins() {
        let modes = vec![
            CompositeAlphaMode::Inherit,
            CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::PreMultiplied,
        ];
        assert_eq!(
            alpha_mode(&capabilities(modes)),
            CompositeAlphaMode::PreMultiplied
        );
        assert_eq!(
            alpha_mode(&capabilities(vec![
                CompositeAlphaMode::Inherit,
                CompositeAlphaMode::PostMultiplied,
            ])),
            CompositeAlphaMode::PostMultiplied
        );
    }

    #[test]
    fn an_unexpected_set_of_alpha_modes_falls_back_to_the_first() {
        assert_eq!(
            alpha_mode(&capabilities(vec![CompositeAlphaMode::Opaque])),
            CompositeAlphaMode::Opaque
        );
        assert_eq!(
            alpha_mode(&capabilities(Vec::new())),
            CompositeAlphaMode::Auto
        );
    }

    struct PendingOnce(bool);

    impl Future for PendingOnce {
        type Output = u8;

        fn poll(mut self: std::pin::Pin<&mut Self>, context: &mut Context<'_>) -> Poll<u8> {
            if self.0 {
                return Poll::Ready(7);
            }
            self.0 = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }

    #[test]
    fn block_on_waits_for_a_future_that_is_ready_at_once() {
        assert_eq!(block_on(ready(7)), 7);
    }

    #[test]
    fn block_on_polls_again_after_pending() {
        assert_eq!(block_on(PendingOnce(false)), 7);
    }
}
