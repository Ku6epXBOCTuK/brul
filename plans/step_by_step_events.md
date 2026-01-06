# Пошаговая реализация системы событий

## 🎯 Ваш выбор: Создать свой Event сразу

Это отличное решение! Вот пошаговый план:

---

## Шаг 1: Создайте модуль событий

**Создайте файл `src/events.rs`** и добавьте этот код:

```rust
use winit::event::{MouseButton, VirtualKeyCode};

/// Базовые события GUI
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    MouseMove { x: f32, y: f32 },
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    KeyDown { key: VirtualKeyCode },
    KeyUp { key: VirtualKeyCode },
    WindowResize { width: u32, height: u32 },
    WindowClose,
}

/// Результат обработки события
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    Continue,
    Stop,
    Redraw,
    Close,
}

/// Трейт для обработки событий
pub trait EventHandler {
    fn handle_event(&mut self, event: &Event) -> EventResult;
}
```

---

## Шаг 2: Добавьте конвертацию из winit

**В тот же файл `src/events.rs`** добавьте:

```rust
pub mod conversions {
    use super::*;
    use winit::event::{WindowEvent, ElementState};

    pub fn from_winit_event(event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                Some(Event::MouseMove {
                    x: position.x as f32,
                    y: position.y as f32
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // Пока используем (0.0, 0.0) - позже добавим отслеживание
                let (x, y) = (0.0, 0.0);
                match state {
                    ElementState::Pressed => Some(Event::MouseDown { button: *button, x, y }),
                    ElementState::Released => Some(Event::MouseUp { button: *button, x, y }),
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                match event.state {
                    ElementState::Pressed => event.virtual_keycode.map(|key| Event::KeyDown { key }),
                    ElementState::Released => event.virtual_keycode.map(|key| Event::KeyUp { key }),
                }
            }
            WindowEvent::Resized(size) => {
                Some(Event::WindowResize { width: size.width, height: size.height })
            }
            WindowEvent::CloseRequested => {
                Some(Event::WindowClose)
            }
            _ => None,
        }
    }
}
```

---

## Шаг 3: Обновите src/lib.rs

```rust
pub mod events;
pub mod app;

pub use events::{Event, EventResult, EventHandler};
pub use app::App;

pub fn start_app() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
```

---

## Шаг 4: Обновите src/app.rs

```rust
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};
use crate::events::{Event, conversions};

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    // Позже добавим: mouse_position: Option<(f32, f32)>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = Window::default_attributes()
                .with_title("BRUL GUI")
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));

            let window = event_loop
                .create_window(win_attr)
                .expect("Create window error");

            self.window = Some(window);
            println!("✅ Window created");
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Конвертируем и обрабатываем
        if let Some(gui_event) = conversions::from_winit_event(&event) {
            println!("📨 Event: {:?}", gui_event);

            match gui_event {
                Event::WindowClose => {
                    println!("👋 Closing...");
                    event_loop.exit();
                }
                Event::WindowResize { width, height } => {
                    println!("📐 Resized: {}x{}", width, height);
                }
                Event::MouseMove { x, y } => {
                    // Для отладки можно раскомментировать:
                    // println!("🖱️ Mouse: ({:.1}, {:.1})", x, y);
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Здесь будет рендеринг в будущем
    }

    fn redraw_requested(&mut self, _window_id: winit::window::WindowId) {
        println!("🎨 Redraw requested");
    }
}
```

---

## Шаг 5: Проверьте что работает

```bash
cargo run
```

**Ожидаемый результат:**

- ✅ Открывается окно 800x600
- ✅ В консоли: "✅ Window created"
- ✅ При движении мыши (если раскомментировать) видно координаты
- ✅ При закрытии: "👋 Closing..."

---

## 🎓 Что вы узнаете на этом шаге

1. **Как winit вызывает ваши методы** - `resumed`, `window_event`, `about_to_wait`
2. **Как конвертировать события** - из winit в ваш формат
3. **Как структурировать код** - модуль events, трейты

---

## 🚀 Следующий шаг (после проверки)

**Добавим отслеживание позиции мыши:**

```rust
// В src/app.rs
pub struct App {
    window: Option<Window>,
    mouse_position: Option<(f32, f32)>, // Добавляем
}

// В window_event:
WindowEvent::CursorMoved { position, .. } => {
    let pos = (position.x as f32, position.y as f32);
    self.mouse_position = Some(pos);

    let gui_event = Event::MouseMove { x: pos.0, y: pos.1 };
    // ... обработка
}
```

---

## 🤔 Вопросы перед началом

1. **Нужна ли вам помощь с установкой зависимостей?** (winit уже есть)
2. **Хотите ли вы сразу добавить отслеживание позиции мыши?**
3. **Какие события важны для вас в первую очередь?**

Начинайте с Шага 1-3, проверяйте, что компилируется, и двигайтесь дальше! Удачи! 🎉
