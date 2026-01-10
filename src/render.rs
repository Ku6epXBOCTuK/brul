use std::{num::NonZero, sync::Arc};

use crate::{Color, Rect};
use wgpu;
use winit::window::Window;

pub struct Renderer {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let renderer = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };
        renderer.configure_surface();
        renderer
    }

    pub fn _resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        self.configure_surface();
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(&self.device, &surface_config);
    }

    pub fn _render() {}

    fn _begin_frame(&mut self) -> Option<wgpu::TextureView> {
        let output = self.surface.get_current_texture().ok()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Some(view)
    }

    fn _end_frame(&mut self) {
        self.queue.submit(None);
    }

    pub fn clear(&mut self, color: Color) {
        let surface_texture = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(_) => return,
        };

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Clear encoder"),
            });

        let operations = wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: color.r as f64,
                g: color.g as f64,
                b: color.b as f64,
                a: color.a as f64,
            }),
            store: wgpu::StoreOp::Store,
        };

        let color_atachments = [Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            resolve_target: None,
            ops: operations,
            depth_slice: None,
        })];

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &color_atachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: Some(NonZero::new(1).unwrap()),
        });

        drop(render_pass);

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }

    pub fn _draw_rect(&mut self, _rect: Rect, _color: Color) {
        // TODO:
        todo!()
    }
}
