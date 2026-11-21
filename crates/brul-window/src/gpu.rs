mod capture;
mod device;
mod frame;
mod textures;

use crate::gpu_frame::FrameCache;

use textures::Textures;

pub(crate) struct Gpu {
    clear_color: wgpu::Color,
    _instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    shape_layout: wgpu::BindGroupLayout,
    shape_pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport: Vec<u8>,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    clip_buffer: wgpu::Buffer,
    clip_capacity: usize,
    quad_pipeline: wgpu::RenderPipeline,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    textures: Textures,
    frame: FrameCache,
    capture: bool,
    captured: Option<Vec<u8>>,
}
