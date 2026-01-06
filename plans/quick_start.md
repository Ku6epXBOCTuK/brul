# 🚀 Быстрый старт: GUI библиотека за 5 шагов

## 📋 TL;DR

**Цель**: Создать окно с цветом фона используя wgpu + winit.

**Время**: 3-5 часов  
**Сложность**: Средняя  
**Результат**: Рабочее окно, готовое к расширению

---

## Шаг 1: Обновить Cargo.toml

```toml
[package]
name = "brul-gui"
version = "0.1.0"
edition = "2024"

[dependencies]
winit = "0.30.12"
wgpu = "27.0.1"
tokio = { version = "1.49.0", features = ["full"] }
pollster = "0.3"
```

---

## Шаг 2: Создать структуру проекта

Создайте следующие файлы:

### `src/core/mod.rs`

```rust
pub mod geometry;
pub mod color;

pub use geometry::*;
pub use color::*;
```

### `src/core/geometry.rs`

```rust
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
```

### `src/core/color.rs`

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
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

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

---

## Шаг 3: Создать рендерер

### `src/render/mod.rs`

```rust
pub mod renderer;
pub use renderer::*;
```

### `src/render/renderer.rs`

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
}
```

---

## Шаг 4: Создать App

### `src/app/mod.rs`

```rust
pub mod app;
pub use app::*;
```

### `src/app/app.rs`

```rust
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use crate::render::Renderer;
use crate::core::Color;
use pollster::block_on;

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

        // Создаем renderer
        let mut renderer = block_on(Renderer::new(window));

        println!("🚀 Brul GUI running...");

        self.event_loop.run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            println!("👋 Closing...");
                            elwt.exit();
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            renderer.resize(size.width, size.height);
                        }
                        _ => {}
                    }
                }
                winit::event::Event::AboutToWait => {
                    // Рендерим кадр
                    if let Some(view) = renderer.begin_frame() {
                        // Очищаем экран белым цветом
                        renderer.clear(&view, Color::WHITE);
                        renderer.end_frame();
                    }
                }
                _ => {}
            }
        }).unwrap();

        // Для возврата ! типа
        panic!("Event loop exited unexpectedly");
    }
}
```

---

## Шаг 5: Обновить основные файлы

### `src/lib.rs`

```rust
pub mod core;
pub mod render;
pub mod app;

pub mod prelude {
    pub use crate::core::*;
    pub use crate::app::App;
}
```

### `src/main.rs`

```rust
use brul_gui::prelude::*;

fn main() {
    println!("🚀 Brul GUI - Starting...");

    let app = App::new();
    app.run();
}
```

### `src/event_loop.rs` (можно удалить или оставить пустым)

---

## 🎯 Тестирование

```bash
# Проверить компиляцию
cargo check

# Запустить
cargo run
```

**Ожидаемый результат**: Открывается окно 800x600 с белым фоном. Можно закрыть крестиком.

---

## 🎓 Что вы узнали

✅ **wgpu basics** - инициализация, рендеринг  
✅ **winit event loop** - обработка событий  
✅ **Базовая структура** - модули, трейты  
✅ **2D геометрия** - точки, размеры, цвета

---

## 🚀 Следующие шаги

После успешного запуска можно:

1. **Добавить прямоугольник** - `renderer.draw_rect()`
2. **Добавить текст** - через fontdue
3. **Создать Widget trait** - базовый интерфейс
4. **Добавить layout** - flex/stack
5. **Обработка событий** - клики, ховеры

---

## 🔧 Отладка

### Окно не открывается

```rust
// Добавьте логирование
println!("Creating window...");
let window = WindowBuilder::new().build(&event_loop)?;
println!("Window created!");
```

### Черный экран

```rust
// Проверьте clear color
renderer.clear(&view, Color::RED); // Должен быть красный
```

### Ошибки wgpu

```rust
// Добавьте больше логов
let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
    // ...
}).await.expect("Не удалось найти адаптер");
```

---

## 📚 Дополнительные ресурсы

- [wgpu tutorial](https://sotrh.github.io/learn-wgpu/)
- [winit examples](https://github.com/rust-windowing/winit/tree/master/examples)
- [Rust GUI landscape](https://www.areweguiyet.com/)

---

**Готовы?** Начинайте с `Cargo.toml` и создавайте файлы по порядку. Удачи! 🎉
