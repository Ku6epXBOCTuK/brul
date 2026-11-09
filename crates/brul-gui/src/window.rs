use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use brul_reactive;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::layout::Size;
use crate::paint::Frame;
use crate::text::TextEngine;
use crate::widget::{WidgetEvent, WidgetTree};

const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.08,
    g: 0.09,
    b: 0.11,
    a: 0.85,
};

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

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub header_height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("BRUL"),
            width: 960,
            height: 640,
            header_height: 48,
        }
    }
}

impl WindowConfig {
    pub fn validate(&self) -> Result<(), WindowError> {
        if self.width == 0 || self.height == 0 {
            return Err(WindowError::EmptySize);
        }
        if self.header_height > self.height {
            return Err(WindowError::HeaderTooTall);
        }
        Ok(())
    }

    fn is_header(&self, y: f64, scale: f64) -> bool {
        y >= 0.0 && y < f64::from(self.header_height) * scale
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
}

#[derive(Debug)]
pub enum WindowError {
    EmptySize,
    HeaderTooTall,
    EventLoop(String),
    CreateWindow(String),
    Surface(String),
    Adapter(String),
    Device(String),
    NoSurfaceConfig,
    Widget(String),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySize => formatter.write_str("window size must be greater than zero"),
            Self::HeaderTooTall => formatter.write_str("header is taller than the window"),
            Self::EventLoop(message) => write!(formatter, "event loop: {message}"),
            Self::CreateWindow(message) => write!(formatter, "window creation: {message}"),
            Self::Surface(message) => write!(formatter, "surface: {message}"),
            Self::Adapter(message) => write!(formatter, "adapter: {message}"),
            Self::Device(message) => write!(formatter, "device: {message}"),
            Self::NoSurfaceConfig => formatter.write_str("no suitable surface configuration"),
            Self::Widget(message) => write!(formatter, "widgets: {message}"),
        }
    }
}

impl std::error::Error for WindowError {}

pub fn run(config: WindowConfig) -> Result<(), WindowError> {
    config.validate()?;
    let event_loop = EventLoop::new().map_err(|error| WindowError::EventLoop(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(RefCell::new(None));
    let app = App {
        config,
        window: None,
        gpu: None,
        error: Rc::clone(&error),
    };
    let result = event_loop.run_app(app);

    if let Some(error) = error.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::EventLoop(error.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    Exit,
    Drag,
}

pub fn run_tree(
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    handler: impl FnMut(&[WidgetEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    run_tree_themed(config, move |_| Rc::clone(&tree), handler)
}

pub fn run_tree_themed(
    config: WindowConfig,
    mut build: impl FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>> + 'static,
    handler: impl FnMut(&[WidgetEvent]) -> Vec<WindowCommand> + 'static,
) -> Result<(), WindowError> {
    config.validate()?;
    let event_loop = EventLoop::new().map_err(|error| WindowError::EventLoop(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(RefCell::new(None));
    let tree = build(ColorScheme::Dark);
    let app = TreeApp {
        config,
        tree,
        build: Box::new(build),
        handler: Box::new(handler),
        window: None,
        gpu: None,
        engine: None,
        frame: None,
        error: Rc::clone(&error),
    };
    let result = event_loop.run_app(app);

    if let Some(error) = error.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::EventLoop(error.to_string()))
}

fn create_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<dyn Window>, WindowError> {
    let size = LogicalSize::new(f64::from(config.width), f64::from(config.height));
    let attributes = WindowAttributes::default()
        .with_title(config.title.clone())
        .with_surface_size(size)
        .with_min_surface_size(size)
        .with_max_surface_size(size)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(false);

    event_loop
        .create_window(attributes)
        .map(Arc::from)
        .map_err(|error| WindowError::CreateWindow(error.to_string()))
}

struct App {
    config: WindowConfig,
    window: Option<Arc<dyn Window>>,
    gpu: Option<Gpu>,
    error: Rc<RefCell<Option<WindowError>>>,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => return self.fail(event_loop, error),
        };
        let gpu = match Gpu::new(&window) {
            Ok(gpu) => gpu,
            Err(error) => return self.fail(event_loop, error),
        };

        self.window = Some(Arc::clone(&window));
        self.gpu = Some(gpu);
        window.set_visible(true);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == window_id)
        else {
            return;
        };
        let window = Arc::clone(window);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    event_loop.exit();
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                position,
                primary: true,
                button,
                is_macos_activation_click: false,
                ..
            } => {
                if button.mouse_button() == Some(MouseButton::Left)
                    && self.config.is_header(position.y, window.scale_factor())
                {
                    let _ = window.drag_window();
                }
            }
            WindowEvent::SurfaceResized(size) => {
                self.resize(size.width, size.height);
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = window.surface_size();
                self.resize(size.width, size.height);
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.render(&window),
            WindowEvent::Occluded(false) => window.request_redraw(),
            _ => {}
        }
    }
}

type EventHandler = Box<dyn FnMut(&[WidgetEvent]) -> Vec<WindowCommand>>;
type TreeBuilder = Box<dyn FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>>>;

fn key_code(key: &Key) -> Option<String> {
    let named = match key {
        Key::Named(NamedKey::Escape) => "Escape",
        Key::Named(NamedKey::Enter) => "Enter",
        Key::Named(NamedKey::Tab) => "Tab",
        Key::Named(NamedKey::Backspace) => "Backspace",
        Key::Named(NamedKey::Delete) => "Delete",
        Key::Named(NamedKey::ArrowUp) => "ArrowUp",
        Key::Named(NamedKey::ArrowDown) => "ArrowDown",
        Key::Named(NamedKey::ArrowLeft) => "ArrowLeft",
        Key::Named(NamedKey::ArrowRight) => "ArrowRight",
        Key::Character(text) => return Some(text.to_string()),
        _ => return None,
    };
    Some(named.to_owned())
}

struct TreeApp {
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: TreeBuilder,
    handler: EventHandler,
    window: Option<Arc<dyn Window>>,
    gpu: Option<Gpu>,
    engine: Option<TextEngine>,
    frame: Option<Frame>,
    error: Rc<RefCell<Option<WindowError>>>,
}

impl ApplicationHandler for TreeApp {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => return self.fail(event_loop, error),
        };
        let gpu = match Gpu::new(&window) {
            Ok(gpu) => gpu,
            Err(error) => return self.fail(event_loop, error),
        };
        let scheme = match event_loop.system_theme() {
            Some(winit::window::Theme::Light) => ColorScheme::Light,
            _ => ColorScheme::Dark,
        };
        self.tree = (self.build)(scheme);
        let mut engine = TextEngine::new();
        let size = window.surface_size();
        let viewport = Size {
            width: size.width,
            height: size.height,
        };
        let layout =
            self.tree
                .borrow_mut()
                .layout(&mut engine, viewport, window.scale_factor() as f32);
        if let Err(error) = layout {
            return self.fail(event_loop, WindowError::Widget(error.to_string()));
        }

        self.window = Some(Arc::clone(&window));
        self.gpu = Some(gpu);
        self.engine = Some(engine);
        self.frame = Some(Frame::new(size.width.max(1), size.height.max(1)));
        window.set_visible(true);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == window_id)
        else {
            return;
        };
        let window = Arc::clone(window);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(code) = key_code(&event.logical_key) {
                        self.dispatch_events(event_loop, &window, vec![WidgetEvent::Key(code)]);
                    }
                    if event.logical_key == Key::Named(NamedKey::Escape) {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::PointerMoved { position, .. } => {
                let events = self
                    .tree
                    .borrow_mut()
                    .pointer_moved(position.x.floor() as i32, position.y.floor() as i32);
                self.dispatch(event_loop, &window, events);
            }
            WindowEvent::PointerLeft { .. } => {
                let events = self.tree.borrow_mut().pointer_leave();
                self.dispatch_events(event_loop, &window, events);
            }
            WindowEvent::PointerButton {
                state,
                position,
                primary: true,
                button,
                is_macos_activation_click: false,
                ..
            } => {
                if button.mouse_button() != Some(MouseButton::Left) {
                    return;
                }
                if state == ElementState::Pressed
                    && self.config.is_header(position.y, window.scale_factor())
                {
                    let _ = window.drag_window();
                    return;
                }

                let x = position.x.floor() as i32;
                let y = position.y.floor() as i32;
                let events = if state == ElementState::Pressed {
                    self.tree.borrow_mut().pointer_down(x, y)
                } else {
                    self.tree.borrow_mut().pointer_up(x, y)
                };
                self.dispatch(event_loop, &window, events);
            }
            WindowEvent::ThemeChanged(theme) => {
                let scheme = match theme {
                    winit::window::Theme::Light => ColorScheme::Light,
                    _ => ColorScheme::Dark,
                };
                self.tree = (self.build)(scheme);
                let size = window.surface_size();
                self.resize(event_loop, &window, size.width, size.height);
            }
            WindowEvent::SurfaceResized(size) => {
                self.resize(event_loop, &window, size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = window.surface_size();
                self.resize(event_loop, &window, size.width, size.height);
            }
            WindowEvent::RedrawRequested => self.render(event_loop, &window),
            WindowEvent::Occluded(false) => window.request_redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &dyn ActiveEventLoop) {
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

impl TreeApp {
    fn fail(&mut self, event_loop: &dyn ActiveEventLoop, error: WindowError) {
        *self.error.borrow_mut() = Some(error);
        event_loop.exit();
    }

    fn dispatch(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window: &Arc<dyn Window>,
        events: Result<Vec<WidgetEvent>, crate::WidgetError>,
    ) {
        match events {
            Ok(events) => self.dispatch_events(event_loop, window, events),
            Err(error) => self.fail(event_loop, WindowError::Widget(error.to_string())),
        }
    }

    fn dispatch_events(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window: &Arc<dyn Window>,
        events: Vec<WidgetEvent>,
    ) {
        if events.is_empty() {
            return;
        }
        if events
            .iter()
            .any(|event| matches!(event, WidgetEvent::Drag(_)))
        {
            let _ = window.drag_window();
        }
        let commands = (self.handler)(&events);
        for command in commands {
            match command {
                WindowCommand::Exit => event_loop.exit(),
                WindowCommand::Drag => {
                    let _ = window.drag_window();
                }
            }
        }
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty() {
            window.request_redraw();
        }
    }

    fn resize(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window: &Arc<dyn Window>,
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
        let layout = match self.engine.as_mut() {
            Some(engine) => self.tree.borrow_mut().layout(
                engine,
                Size { width, height },
                window.scale_factor() as f32,
            ),
            None => Ok(()),
        };
        if let Err(error) = layout {
            return self.fail(event_loop, WindowError::Widget(error.to_string()));
        }
        if let Some(frame) = self.frame.as_mut() {
            frame.resize(width, height);
        } else {
            self.frame = Some(Frame::new(width, height));
        }
        window.request_redraw();
    }

    fn render(&mut self, event_loop: &dyn ActiveEventLoop, window: &Arc<dyn Window>) {
        brul_reactive::flush();
        if let Err(error) = self.render_frame(window) {
            self.fail(event_loop, WindowError::Widget(error.to_string()));
        }
    }

    fn render_frame(&mut self, window: &Arc<dyn Window>) -> Result<(), crate::WidgetError> {
        if self.tree.borrow().layout_dirty()
            && let Some(engine) = self.engine.as_mut()
        {
            self.tree.borrow_mut().relayout(engine)?;
        }
        let (dirty, rebuild) = {
            let mut tree = self.tree.borrow_mut();
            (tree.take_dirty(), tree.snapshot_dirty())
        };
        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        let Some(frame) = self.frame.as_mut() else {
            return Ok(());
        };
        let Some(engine) = self.engine.as_ref() else {
            return Ok(());
        };

        if rebuild || !dirty.is_empty() {
            let mut tree = self.tree.borrow_mut();
            let snapshot = tree.snapshot()?;
            frame.paint(snapshot, engine.atlas(), &dirty);
        }

        gpu.present_frame(window, frame);
        Ok(())
    }
}

impl App {
    fn fail(&mut self, event_loop: &dyn ActiveEventLoop, error: WindowError) {
        *self.error.borrow_mut() = Some(error);
        event_loop.exit();
    }

    fn resize(&mut self, width: u32, height: u32) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        gpu.resize(width, height);
    }

    fn render(&mut self, window: &Arc<dyn Window>) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                window.request_redraw();
                return;
            }
            _ => return,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
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
                        load: wgpu::LoadOp::Clear(CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        gpu.queue.submit(Some(encoder.finish()));
        window.pre_present_notify();
        gpu.queue.present(frame);
    }
}

struct Gpu {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
    fn new(window: &Arc<dyn Window>) -> Result<Self, WindowError> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(window))
            .map_err(|error| WindowError::Surface(error.to_string()))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(|error| WindowError::Adapter(error.to_string()))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|error| WindowError::Device(error.to_string()))?;

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

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.config.width == width && self.config.height == height {
            return;
        }

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

    fn present_frame(&mut self, window: &Arc<dyn Window>, frame: &Frame) {
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
                window.request_redraw();
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
        window.pre_present_notify();
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
    use super::{WindowConfig, WindowError};

    #[test]
    fn default_config_is_valid() {
        assert!(WindowConfig::default().validate().is_ok());
    }

    #[test]
    fn zero_size_is_rejected() {
        let config = WindowConfig {
            width: 0,
            ..WindowConfig::default()
        };
        assert!(matches!(config.validate(), Err(WindowError::EmptySize)));
    }

    #[test]
    fn header_taller_than_window_is_rejected() {
        let config = WindowConfig {
            height: 40,
            header_height: 41,
            ..WindowConfig::default()
        };
        assert!(matches!(config.validate(), Err(WindowError::HeaderTooTall)));
    }

    #[test]
    fn header_hit_test_uses_physical_pixels() {
        let config = WindowConfig {
            header_height: 40,
            ..WindowConfig::default()
        };
        assert!(config.is_header(0.0, 1.0));
        assert!(config.is_header(39.9, 1.0));
        assert!(!config.is_header(40.0, 1.0));
        assert!(!config.is_header(-1.0, 1.0));
        assert!(config.is_header(79.0, 2.0));
        assert!(!config.is_header(80.0, 2.0));
    }
}
