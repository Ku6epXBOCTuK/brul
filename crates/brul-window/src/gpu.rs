use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};

use winit::window::Window;

use brul_gui::{Atlas, Dirty, Texture, WidgetSnapshot};

use crate::gpu_frame::{CLIP, DECOR, FrameCache, GLYPH, IMAGE, SHAPE};
use crate::window::WindowError;

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

struct FrameData {
    clip: Vec<u8>,
    instances: Vec<u8>,
    quads: Vec<u8>,
    shape_count: usize,
    decor_count: usize,
    glyph_count: usize,
    image_ranges: Vec<(u32, u32, Rc<Texture>)>,
}

pub(crate) struct Gpu {
    clear_color: wgpu::Color,
    _instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    nearest_sampler: wgpu::Sampler,
    shape_layout: wgpu::BindGroupLayout,
    shape_pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    clip_buffer: wgpu::Buffer,
    clip_capacity: usize,
    quad_layout: wgpu::BindGroupLayout,
    quad_pipeline: wgpu::RenderPipeline,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    atlas: Option<AtlasTexture>,
    images: HashMap<usize, ImageTexture>,
    frame: FrameCache,
    frame_index: u64,
    capture: bool,
    captured: Option<Vec<u8>>,
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
    pub(crate) fn capture(&mut self) -> Option<Vec<u8>> {
        self.captured.take()
    }

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
        let mut capture = crate::perf::dump_path().is_some();
        if capture {
            if capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC) {
                config.usage |= wgpu::TextureUsages::COPY_SRC;
            } else {
                tracing::warn!("surface cannot be read back, frame dump is disabled");
                capture = false;
            }
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

        Ok(Self {
            clear_color: clear_color(transparent),
            _instance: instance,
            surface,
            device,
            queue,
            config,
            nearest_sampler,
            shape_layout,
            shape_pipeline,
            viewport_buffer,
            instance_buffer,
            instance_capacity: 0,
            clip_buffer,
            clip_capacity: brul_gpu::CLIP_INSTANCE_SIZE as usize,
            quad_layout,
            quad_pipeline,
            quad_buffer,
            quad_capacity: brul_gpu::TEXTURE_INSTANCE_SIZE as usize,
            atlas: None,
            images: HashMap::new(),
            frame: FrameCache::default(),
            frame_index: 0,
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
                        load: wgpu::LoadOp::Clear(self.clear_color),
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

    pub(crate) fn render(
        &mut self,
        surface: &dyn crate::surface::Surface,
        snapshot: &WidgetSnapshot,
        atlas: &Atlas,
        dirty: &Dirty,
    ) -> bool {
        let (width, height) = surface.size();
        self.frame.update(snapshot, atlas, dirty);
        let frame = &self.frame;
        let clip = frame.buffer(CLIP).to_vec();
        let mut instances = frame.buffer(SHAPE).to_vec();
        instances.extend_from_slice(frame.buffer(DECOR));
        let glyph_bytes = frame.buffer(GLYPH).len();
        let mut quads = frame.buffer(GLYPH).to_vec();
        quads.extend_from_slice(frame.buffer(IMAGE));
        let glyph_count = glyph_bytes / brul_gpu::TEXTURE_INSTANCE_SIZE as usize;
        let image_ranges: Vec<_> = frame
            .images()
            .iter()
            .map(|(start, count, texture)| {
                (*start + glyph_count as u32, *count, Rc::clone(texture))
            })
            .collect();
        let shape_count = frame.buffer(SHAPE).len() / brul_gpu::INSTANCE_SIZE as usize;
        let decor_count = frame.buffer(DECOR).len() / brul_gpu::INSTANCE_SIZE as usize;
        self.present_frame(
            surface,
            (width.max(1), height.max(1)),
            atlas,
            FrameData {
                clip,
                instances,
                quads,
                shape_count,
                decor_count,
                glyph_count,
                image_ranges,
            },
        )
    }

    fn present_frame(
        &mut self,
        surface: &dyn crate::surface::Surface,
        (width, height): (u32, u32),
        atlas: &Atlas,
        data: FrameData,
    ) -> bool {
        let FrameData {
            clip,
            instances,
            quads,
            shape_count,
            decor_count,
            glyph_count,
            image_ranges,
        } = data;
        if instances.len() > self.instance_capacity {
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brul-shape-instances"),
                size: instances.len() as u64,
                usage: brul_gpu::instance_usage(),
                mapped_at_creation: false,
            });
            self.instance_capacity = instances.len();
        }
        if !clip.is_empty() {
            if clip.len() > self.clip_capacity {
                self.clip_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("brul-clips"),
                    size: clip.len() as u64,
                    usage: brul_gpu::instance_usage(),
                    mapped_at_creation: false,
                });
                self.clip_capacity = clip.len();
                if let Some(cache) = &mut self.atlas {
                    cache.bind_group = None;
                }
                for cache in self.images.values_mut() {
                    cache.bind_group = None;
                }
            }
            self.queue.write_buffer(&self.clip_buffer, 0, &clip);
        }
        if !quads.is_empty() {
            if quads.len() > self.quad_capacity {
                self.quad_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("brul-quad-instances"),
                    size: quads.len() as u64,
                    usage: brul_gpu::instance_usage(),
                    mapped_at_creation: false,
                });
                self.quad_capacity = quads.len();
                if let Some(cache) = &mut self.atlas {
                    cache.bind_group = None;
                }
                for cache in self.images.values_mut() {
                    cache.bind_group = None;
                }
            }
            self.queue.write_buffer(&self.quad_buffer, 0, &quads);
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
        let glyph_bind_group = (glyph_count > 0).then(|| self.ensure_atlas(atlas).clone());
        let mut image_bind_groups = Vec::with_capacity(image_ranges.len());
        for (_, _, texture) in &image_ranges {
            image_bind_groups.push(self.ensure_image(texture).clone());
        }

        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, &instances);
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.clip_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
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
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if shape_count > 0 {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &shape_bind_group, &[]);
                pass.draw(0..6, 0..shape_count as u32);
            }
            if decor_count > 0 {
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &shape_bind_group, &[]);
                pass.draw(0..6, shape_count as u32..(shape_count + decor_count) as u32);
            }
            if !quads.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                if let Some(group) = &glyph_bind_group {
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..6, 0..glyph_count as u32);
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
        if self.capture && self.captured.is_none() {
            self.captured = self.readback(&surface_frame.texture, width, height);
        }
        surface.pre_present_notify();
        self.queue.present(surface_frame);
        true
    }

    fn readback(&self, texture: &wgpu::Texture, width: u32, height: u32) -> Option<Vec<u8>> {
        let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brul-frame-readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("brul-frame-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let done = Arc::new(AtomicBool::new(false));
        buffer.map_async(wgpu::MapMode::Read, 0..buffer.size(), {
            let done = Arc::clone(&done);
            move |_| done.store(true, Ordering::SeqCst)
        });
        while !done.load(Ordering::SeqCst) {
            let _ = self.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
        }
        let view = buffer.get_mapped_range(..).ok()?;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for row in view.chunks(stride as usize).take(height as usize) {
            pixels.extend_from_slice(&row[..width as usize * 4]);
        }
        drop(view);
        buffer.unmap();
        Some(pixels)
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
                &self.clip_buffer,
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
                &self.clip_buffer,
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
    clips: &wgpu::Buffer,
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
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: clips,
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

fn size_change(current: (u32, u32), width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    (current != (width, height)).then_some((width, height))
}

fn unorm_format(format: wgpu::TextureFormat) -> wgpu::TextureFormat {
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
    fn a_transparent_window_clears_to_transparent_black() {
        assert_eq!(
            clear_color(true),
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }
        );
    }

    #[test]
    fn an_opaque_window_keeps_the_regular_clear_color() {
        assert_eq!(clear_color(false), crate::window::CLEAR);
    }

    #[test]
    fn an_srgb_surface_format_becomes_its_unorm_pair() {
        assert_eq!(
            unorm_format(wgpu::TextureFormat::Bgra8UnormSrgb),
            wgpu::TextureFormat::Bgra8Unorm
        );
        assert_eq!(
            unorm_format(wgpu::TextureFormat::Rgba8UnormSrgb),
            wgpu::TextureFormat::Rgba8Unorm
        );
        assert_eq!(
            unorm_format(wgpu::TextureFormat::Rgba16Float),
            wgpu::TextureFormat::Rgba16Float
        );
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
