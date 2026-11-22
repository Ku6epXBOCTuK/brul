mod capture;
mod device;
mod effects;
mod frame;
mod textures;

use std::collections::HashMap;

use crate::gpu_frame::FrameCache;

use effects::EffectPipeline;
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
    effects_enabled: bool,
    effect_pipelines: HashMap<&'static str, EffectPipeline>,
    effect_layout: wgpu::BindGroupLayout,
    effect_empty_layout: wgpu::BindGroupLayout,
    effect_empty_group: wgpu::BindGroup,
    effect_sampler: wgpu::Sampler,
    effect_uniform_buffer: wgpu::Buffer,
    effect_uniform: Vec<u8>,
    effect_buffer: wgpu::Buffer,
    effect_capacity: usize,
    effect_backdrop: wgpu::Texture,
    effect_backdrop_view: wgpu::TextureView,
    effect_history: [wgpu::Texture; 2],
    effect_history_views: [wgpu::TextureView; 2],
    effect_history_write: usize,
    capture: bool,
    captured: Option<Vec<u8>>,
}
