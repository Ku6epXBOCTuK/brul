Для GUI-библиотеки на winit + wgpu структура будет особенной. Вот пошаговый план рефакторинга с учётом специфики графических библиотек.

## **Этап 1: Проанализируйте текущий "монолит"**

Сначала определите, какие части у вас уже есть:

- Инициализация wgpu/winit
- Шейдеры/пайплайны
- Рендер-граф или система отрисовки
- UI-компоненты (кнопки, панели)
- Система событий
- Обработка входных данных
- Управление состоянием

## **Этап 2: Целевая архитектура (типичная для GUI-библиотеки)**

Вот рекомендованная структура для графической библиотеки:

```
src/
├── lib.rs              # Основной реэкспорт
├── core/               # Ядро библиотеки
│   ├── mod.rs
│   ├── application.rs  # Главный цикл, App/Window
│   ├── context.rs      # WgpuContext, Surface, Device, Queue
│   ├── config.rs       # Настройки графики
│   └── state.rs        # Состояние приложения
├── render/             # Рендеринг
│   ├── mod.rs
│   ├── renderer.rs     # Основной рендерер
│   ├── pipeline.rs     # Пайплайны wgpu
│   ├── shaders.rs      # Шейдеры (или shaders/ директория)
│   ├── texture.rs      # Текстуры, загрузка изображений
│   ├── font.rs         # Работа со шрифтами
│   └── passes/         # Рендер-пассы
│       ├── ui_pass.rs
│       ├── clear_pass.rs
│       └── ...
├── ui/                 # UI компоненты
│   ├── mod.rs
│   ├── widget.rs       # Базовый trait Widget
│   ├── layout.rs       # Компоновка (Flex, Grid)
│   ├── theme.rs        # Темы, стили
│   ├── components/     # Конкретные виджеты
│   │   ├── button.rs
│   │   ├── label.rs
│   │   ├── panel.rs
│   │   └── ...
│   └── builder.rs      # Builder-паттерн для виджетов
├── input/              # Обработка ввода
│   ├── mod.rs
│   ├── events.rs       # Система событий
│   ├── mouse.rs        # Обработка мыши
│   └── keyboard.rs     # Обработка клавиатуры
├── math/               # Математика для графики
│   ├── mod.rs
│   ├── transform.rs    # Матрицы преобразований
│   ├── geometry.rs     # Примитивы (Rect, Vec2, Color)
│   └── anim.rs         # Анимации, интерполяция
├── platform/           # Платформозависимые вещи
│   ├── mod.rs
│   ├── window.rs       # Обёртка над winit
│   └── events.rs       # Конвертация winit событий
└── utils/              # Утилиты
    ├── mod.rs
    ├── logging.rs
    ├── profiling.rs    # Профилирование (tracing/puffin)
    └── resources.rs    # Менеджер ресурсов
```

## **Этап 3: Конкретный план рефакторинга**

### **Шаг 1: Создайте модуль `core/`**

Выделите самое ядро:

```rust
// src/core/context.rs
pub struct GraphicsContext {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

// src/core/application.rs
pub struct Application {
    context: GraphicsContext,
    event_loop: Option<EventLoop<()>>,
    // ...
}

impl Application {
    pub fn new() -> Result<Self> {
        // Инициализация winit + wgpu
    }

    pub fn run(self) {
        // Главный цикл
    }
}
```

### **Шаг 2: Выделите рендеринг**

```rust
// src/render/renderer.rs
pub struct Renderer {
    context: Arc<GraphicsContext>,
    pipelines: HashMap<PipelineType, RenderPipeline>,
    ui_pass: UIPass,
}

impl Renderer {
    pub fn new(context: &GraphicsContext) -> Self {
        // Создание пайплайнов
    }

    pub fn render(&mut self, view: &TextureView, ui_components: &[UiComponent]) {
        // Рендер-граф
    }
}
```

### **Шаг 3: Постройте UI систему**

```rust
// src/ui/widget.rs
pub trait Widget {
    fn draw(&self, renderer: &mut Renderer, position: Rect);
    fn handle_event(&mut self, event: &Event) -> EventResult;
    fn layout(&mut self, constraints: Constraints) -> Size;
}

// src/ui/components/button.rs
pub struct Button {
    text: String,
    bounds: Rect,
    state: ButtonState,
}

impl Widget for Button { ... }
```

### **Шаг 4: Рефакторинг шаг за шагом**

1. **Сначала создайте структуру папок и пустые модули**

2. **Выделите GraphicsContext** - это проще всего:
   - Найдите всю инициализацию wgpu
   - Вынесите в отдельный модуль
   - Убедитесь, что всё ещё компилируется

3. **Создайте Application struct**:
   - Оберните текущий main loop
   - Сделайте публичное API: `Application::new()`, `app.run()`

4. **Постепенно выносите UI компоненты**:
   - Начните с простых (Label, Panel)
   - Определите общий trait Widget
   - Создайте систему компоновки

5. **Реорганизуйте шейдеры и пайплайны**:
   ```rust
   // Вынести в отдельные файлы
   const VERTEX_SHADER: &str = include_str!("shaders/ui.vert.wgsl");
   const FRAGMENT_SHADER: &str = include_str!("shaders/ui.frag.wgsl");
   ```

## **Этап 4: Ключевые моменты для winit/wgpu**

### **Обработка событий:**

```rust
// src/platform/events.rs
pub fn convert_winit_event(event: &winit::event::WindowEvent) -> Option<crate::Event> {
    match event {
        winit::event::WindowEvent::MouseInput { state, button, .. } => {
            Some(Event::MouseButton {
                button: convert_mouse_button(button),
                pressed: *state == winit::event::ElementState::Pressed,
            })
        }
        // ...
    }
}
```

### **Рендер-граф:**

```rust
// src/render/passes/ui_pass.rs
pub struct UIPass {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
}

impl UIPass {
    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, widgets: &[&dyn Widget]) {
        // Рендеринг UI
    }
}
```

## **Этап 5: Публичное API**

После рефакторинга API может выглядеть так:

```rust
// Основное использование
fn main() {
    let app = gui::Application::builder()
        .with_title("My App")
        .with_size(800, 600)
        .build()
        .unwrap();

    let mut window = app.create_window();

    window.add_widget(Button::new("Click me")
        .on_click(|| println!("Clicked!")));

    window.add_widget(Label::new("Hello, world!")
        .with_color(Color::RED));

    app.run();
}
```

## **Этап 6: Инструменты для помощи**

1. **Профилирование**: Добавьте `tracing` или `puffin` для отслеживания производительности
2. **Тестирование**: GUI сложно тестировать, но добавьте unit-тесты для математики, логики компоновки
3. **Документация**: Обязательно примеры в `examples/`
   ```bash
   examples/
   ├── basic_window/
   ├── multiple_widgets/
   └── custom_widget/
   ```

## **Советы по процессу:**

1. **Начните снизу вверх**:
   - Сначала GraphicsContext (работает с wgpu напрямую)
   - Потом Renderer (использует Context)
   - Потом UI система (использует Renderer)

2. **Используйте `Arc<Mutex<...>>` аккуратно**:
   - wgpu ресурсы не всегда можно разделять
   - Рассмотрите ECS (specs, bevy_ecs) для сложных UI

3. **Сохраняйте компиляцию на каждом шаге**:
   - Рефакторьте небольшими порциями
   - Используйте `todo!()` для нереализованных частей

4. **Пример первого шага**:

   ```rust
   // Было: один файл main.rs со всем кодом
   // Стало:
   // main.rs
   fn main() {
       let app = MyGuiApp::new();
       app.run();
   }

   // src/core/application.rs
   pub struct MyGuiApp {
       // Пока просто обёртка над старым кодом
       inner: LegacyMonolith,
   }

   impl MyGuiApp {
       pub fn new() -> Self {
           Self { inner: LegacyMonolith::new() }
       }

       pub fn run(self) {
           self.inner.run();
       }
   }
   ```

Начните с выделения GraphicsContext - это даст немедленную выгоду и сделает код чище. Затем двигайтесь к рендереру, а потом к UI системе.
