use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use winit::window::Window;

use brul_gui::Frame;

use crate::window::WindowError;

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
    blit_sampler: wgpu::Sampler,
    blit: Option<BlitTarget>,
}

struct BlitTarget {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl Gpu {
    pub(crate) fn new(window: &Arc<dyn Window>) -> Result<Self, WindowError> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(window))
            .map_err(|error| WindowError::stage("surface", error))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(|error| WindowError::stage("adapter", error))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|error| WindowError::stage("device", error))?;

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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("brul-blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            blit_layout,
            blit_pipeline,
            blit_sampler,
            blit: None,
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
            &self.blit_sampler,
            width,
            height,
        ));
    }

    pub(crate) fn present_frame(&mut self, surface: &dyn crate::surface::Surface, frame: &Frame) {
        let width = frame.width().max(1);
        let height = frame.height().max(1);
        self.ensure_blit(width, height);
        let target = self.blit.as_ref().expect("blit target");

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            frame.pixels(),
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

        let surface_frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                surface.redraw();
                return;
            }
            _ => return,
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &target.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        surface.pre_present_notify();
        self.queue.present(surface_frame);
    }
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
