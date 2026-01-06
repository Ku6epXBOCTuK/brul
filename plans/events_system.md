# Система событий для GUI библиотеки

## 🎯 Цель

Создать систему обработки событий, которая будет работать поверх winit event loop.

## 📋 Концепция

### Что такое Event Loop в winit?

```rust
// winit предоставляет event loop, который вызывает методы ApplicationHandler:
event_loop.run_app(&mut app);
```

**Как это работает:**

1. `resumed()` - приложение запущено, создаем окно
2. `window_event()` - приходят события окна (мышь, клавиатура, ресайз)
3. `about_to_wait()` - перед тем, как цикл пойдет ждать следующее событие
4. Повторять до закрытия

## 🏗️ План реализации

### Шаг 1: Создайте модуль событий

**Файл: `src/events.rs`**

```rust
//! Система событий для GUI библиотеки

use winit::event::{MouseButton, ElementState, VirtualKeyCode};

/// Базовые события GUI
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// События мыши
    MouseMove { x: f32, y: f32 },
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    MouseEnter,
    MouseLeave,

    /// События клавиатуры
    KeyDown { key: VirtualKeyCode },
    KeyUp { key: VirtualKeyCode },

    /// События окна
    WindowResize { width: u32, height: u32 },
    WindowClose,

    /// Специальные события
    RequestRedraw,
}

/// Результат обработки события
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    Continue,  // Продолжить обработку
    Stop,      // Остановить обработку
    Redraw,    // Запросить перерисовку
    Close,     // Запросить закрытие
}

/// Трейт для обработки событий
pub trait EventHandler {
    fn handle_event(&mut self, event: &Event) -> EventResult;
}

/// Вспомогательные функции для конвертации winit событий
pub mod conversions {
    use super::*;
    use winit::event::WindowEvent;

    /// Конвертирует winit WindowEvent в наш Event
    pub fn from_winit_event(event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                Some(Event::MouseMove {
                    x: position.x as f32,
                    y: position.y as f32
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // TODO: Получить текущую позицию мыши
                let x = 0.0;
                let y = 0.0;
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
            WindowEvent::CursorEntered { .. } => Some(Event::MouseEnter),
            WindowEvent::CursorLeft { .. } => Some(Event::MouseLeave),
            _ => None,
        }
    }
}
```

### Шаг 2: Обновите App для поддержки событий

**Файл: `src/app.rs`**

```rust
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};
use crate::events::{Event, EventResult, conversions};

pub struct App {
    window: Option<Window>,
    // Добавим обработчик событий в будущем
    // event_handler: Option<Box<dyn EventHandler>>,
}

impl Default for App {
    fn default() -> Self {
        Self { window: None }
    }
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
        // Конвертируем winit событие в наше
        if let Some(gui_event) = conversions::from_winit_event(&event) {
            println!("📨 Event: {:?}", gui_event);

            // TODO: Здесь будет обработка событий
            match gui_event {
                Event::WindowClose => {
                    println!("👋 Closing window...");
                    event_loop.exit();
                }
                Event::WindowResize { width, height } => {
                    println!("📐 Resized: {}x{}", width, height);
                }
                Event::MouseMove { x, y } => {
                    // Можно добавить логирование для отладки
                    // println!("🖱️ Mouse at: ({:.1}, {:.1})", x, y);
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Этот метод вызывается перед тем, как event loop пойдет ждать следующее событие
        // Идеальное место для рендеринга!

        if let Some(window) = &self.window {
            // TODO: Здесь будет рендеринг
            // window.request_redraw(); // Запросить перерисовку
        }
    }

    fn redraw_requested(&mut self, _window_id: winit::window::WindowId) {
        // Этот метод вызывается, когда окно нужно перерисовать
        // TODO: Здесь будет код рендеринга
        println!("🎨 Redraw requested");
    }
}
```

### Шаг 3: Обновите lib.rs

**Файл: `src/lib.rs`**

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

## 🧪 Тестирование

После реализации запустите:

```bash
cargo run
```

**Ожидаемое поведение:**

- Открывается окно 800x600 с заголовком "BRUL GUI"
- В консоли появляются сообщения о создании окна
- При движении мыши можно добавить логирование (закомментировано)
- При закрытии окна появляется сообщение "Closing..."

## 🎓 Ключевые концепции

### 1. ApplicationHandler

Это трейт winit, который определяет, как приложение реагирует на события.

### 2. Event Conversion

Мы конвертируем winit события в наши собственные типы. Это дает:

- Контроль над API
- Возможность добавить дополнительную логику
- Легкую замену winit в будущем

### 3. Event Flow

```
winit event → conversions::from_winit_event() → наш Event → обработка → результат
```

## 🚀 Следующие шаги

После того как это заработает:

1. **Добавить рендеринг** - создать `Renderer` и вызывать его в `redraw_requested`
2. **Добавить обработчик событий** - trait `EventHandler` для виджетов
3. **Создать систему координат** - отслеживать позицию мыши
4. **Добавить виджеты** - которые могут обрабатывать события

## 🤔 Вопросы для вас

1. **Как вы хотите хранить текущую позицию мыши?**
   - В `App` как поле?
   - Передавать в событиях?
   - Другой способ?

2. **Нужна ли вам система "захвата" событий?**
   - Например, когда элемент захватывает мышь и получает все события до отпускания

3. **Как вы хотите передавать события в виджеты?**
   - Простой callback?
   - Иерархическая система (родитель → дети)?
   - Другой подход?

Начните с реализации шагов 1-3 и проверьте, что события логируются правильно. После этого добавим рендеринг! 🎨
