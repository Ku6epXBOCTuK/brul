# Технические детали реализации GUI библиотеки

## 🎯 Приоритетная реализация

Рекомендую начать с **минимальной рабочей версии** (MVP), чтобы быстро получить результат.

### MVP: "Hello Rectangle"

Цель: Отобразить статичный прямоугольник на экране с использованием wgpu.

## 📋 Детальный план по файлам

### 1. `src/core/geometry.rs`

```rust
use std::ops::{Add, Sub, Mul, Div};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size { width: 0.0, height: 0.0 };
    pub fn new(width: f32, height: f32) -> Self { Self { width, height } }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self { Self { x, y, width, height } }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Edges {
    pub const ZERO: Edges = Edges { left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 };
    pub fn new(all: f32) -> Self { Self { left: all, right: all, top: all, bottom: all } }
}
```

### 2. `src/core/color.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_rgba8(&self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }
}
```

### 3. `src/render/renderer.rs`

```rust
use wgpu;
use winit::window::Window;
use crate::core::{Rect, Color};

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Window,
}

impl Renderer {
    pub async fn new(window: Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12ShaderCompiler::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let window_size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            window,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn begin_frame(&mut self) -> Option<wgpu::TextureView> {
        let output = self.surface.get_current_texture().ok()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some(view)
    }

    pub fn end_frame(&mut self) {
        self.queue.submit(None);
    }

    pub fn clear(&mut self, view: &wgpu::TextureView, color: Color) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color.r as f64,
                            g: color.g as f64,
                            b: color.b as f64,
                            a: color.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
    }

    pub fn draw_rect(&mut self, view: &wgpu::TextureView, rect: Rect, color: Color) {
        // TODO: Реализовать рисование прямоугольника через вершинные буферы
        // Для MVP можно использовать wgpu::PrimitiveTopology::TriangleList
    }
}
```

### 4. `src/app/app.rs`

```rust
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use crate::render::Renderer;
use crate::core::Color;

pub struct App {
    event_loop: EventLoop<()>,
}

impl App {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        Self { event_loop }
    }

    pub fn run(self) -> ! {
        let window = WindowBuilder::new()
            .with_title("Brul GUI")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&self.event_loop)
            .unwrap();

        // Создаем renderer (в будущем будет асинхронно)
        // let renderer = pollster::block_on(Renderer::new(window));

        println!("App running...");

        self.event_loop.run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            println!("Resized: {:?}", size);
                        }
                        _ => {}
                    }
                }
                winit::event::Event::AboutToWait => {
                    // Render frame here
                }
                _ => {}
            }
        }).unwrap();
    }
}
```

### 5. Обновление `src/lib.rs`

```rust
pub mod core;
pub mod render;
pub mod app;

pub mod prelude {
    pub use crate::core::*;
    pub use crate::app::App;
}
```

### 6. Обновление `src/main.rs`

```rust
use brul_gui::prelude::*;

fn main() {
    println!("Brul GUI - Starting...");

    let app = App::new();
    app.run();
}
```

## 🎯 Пошаговый план MVP

### Шаг 1: Базовая структура (сделано)

- [x] Cargo.toml с зависимостями
- [x] Базовая структура модулей

### Шаг 2: Core примитивы

- [ ] Создать `src/core/geometry.rs`
- [ ] Создать `src/core/color.rs`
- [ ] Обновить `src/core/mod.rs`

### Шаг 3: Базовый рендерер

- [ ] Создать `src/render/renderer.rs`
- [ ] Добавить `src/render/mod.rs`
- [ ] Настроить wgpu инициализацию

### Шаг 4: App и event loop

- [ ] Создать `src/app/app.rs`
- [ ] Добавить `src/app/mod.rs`
- [ ] Интегрировать с winit

### Шаг 5: Тестовый запуск

- [ ] Обновить `src/lib.rs`
- [ ] Обновить `src/main.rs`
- [ ] Запустить и проверить окно

## 🔧 Дополнительные зависимости

Добавить в `Cargo.toml`:

```toml
[dependencies]
winit = "0.30.12"
wgpu = "27.0.1"
tokio = { version = "1.49.0", features = ["full"] }
pollster = "0.3"  # Для блокирующего async в main
glam = "0.29"     # Математика (опционально)
```

## 📝 Примеры кода для будущих этапов

### Пример: Widget trait

```rust
pub trait Widget {
    fn layout(&mut self, constraints: BoxConstraints) -> Size;
    fn paint(&self, renderer: &mut Renderer, bounds: Rect);
    fn handle_event(&mut self, event: &Event) -> EventResult;
}
```

### Пример: Container widget

```rust
pub struct Container {
    child: Option<Box<dyn Widget>>,
    padding: Edges,
    background: Color,
}

impl Widget for Container {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        // Логика layout для контейнера
        Size::new(100.0, 100.0) // TODO
    }

    fn paint(&self, renderer: &mut Renderer, bounds: Rect) {
        // Рисуем фон
        renderer.draw_rect(bounds, self.background);

        // Рисуем child
        if let Some(child) = &self.child {
            let child_bounds = bounds.shrink(self.padding.left, self.padding.top);
            child.paint(renderer, child_bounds);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // Обработка событий
        EventResult::Continue
    }
}
```

## 🎯 Следующие действия

1. **Создать структуру директорий** в `src/`
2. **Реализовать core примитивы** (geometry, color)
3. **Настроить базовый рендерер** на wgpu
4. **Создать App** с event loop
5. **Тестовый запуск** - отобразить окно с цветом фона

После этого можно добавлять виджеты и систему layout.
