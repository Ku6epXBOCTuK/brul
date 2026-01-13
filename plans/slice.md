## **Уровень 0: Макросы (отдельный крейт)**

```
my_ui_macros/
  src/lib.rs
```

**Только процедурные макросы:**

- `#[component]` — превращает функцию в компонент
- `html! { ... }` — парсит разметку в AST (абстрактное дерево)
- `css! { ... }` — парсит CSS в структуры Rust

**На выходе:** Генерирует вызовы конструкторов виджетов и стилей для основного крейта.

---

## **Уровень 1: Public API (основной крейт)**

```
my_ui/
  src/lib.rs        <- реэкспорт всего публичного
  src/public/
    mod.rs
    widget.rs       <- типажи Widget, IntoWidget
    component.rs    <- типаж Component
    style.rs        <- публичные структуры стилей (Color, Size)
    event.rs        <- MouseEvent, KeyEvent
    app.rs          <- App::new().run()
```

**Что видит пользователь:**

```rust
use my_ui::prelude::*;

#[component]
fn MyButton() -> impl Widget {
    html! { <button>"Click"</button> }
}
```

---

## **Уровень 2: Ядро (Core) — абстракции без реализации**

```
src/core/
  mod.rs
  node/             <- виртуальное дерево
    mod.rs
    id.rs           <- WidgetId, generational индекс
    vnode.rs        <- enum VNode { Element, Text, Component }
    vtree.rs        <- дерево с parent/children ссылками

  widget/           <- абстрактные виджеты
    mod.rs
    context.rs      <- WidgetContext (для state, children)
    builder.rs      <- WidgetBuilder (конструктор)

  style/            <- абстрактные стили
    mod.rs
    selector.rs     <- CSS селектор (класс, тег, id)
    rule.rs         <- StyleRule (селектор + свойства)

  layout/           <- абстрактный лейаут
    mod.rs
    constraint.rs   <- Constraints (min/max размер)
    geometry.rs     <- Rect, Size, Point
```

**Принцип:** Здесь только данные и типажи. Никакого `wgpu`, `winit`. Можно тестировать изолированно.

---

## **Уровень 3: Стилизация (Styling) — конкретная реализация**

```
src/styling/
  mod.rs
  engine.rs         <- StyleEngine (хранит правила, кэш)
  cascade.rs        <- каскадирование стилей
  compute.rs        <- вычисление значений (px -> f32)
  properties.rs     <- все CSS свойства как Rust enum
```

**Зависит от:** `core::style`, `core::node`

**Не зависит от:** рендера, лейаута, оконной системы

---

## **Уровень 4: Лейаут (Layout) — конкретная реализация**

```
src/layout/
  mod.rs
  engine.rs         <- LayoutEngine (использует taffy)
  node.rs           <- LayoutNode (taffy нода + наши данные)
  algorithm/        <- разные алгоритмы
    mod.rs
    flex.rs
    block.rs
    grid.rs
```

**Зависит от:** `core::layout`, `core::geometry`, `styling::properties`

**Не зависит от:** рендера, GPU

---

## **Уровень 5: Рендер (Render) — абстракция рисования**

```
src/render/
  mod.rs
  painter.rs        <- типаж Painter: draw_rect(), draw_text()
  commands.rs       <- RenderCommand (примитивы для отрисовки)
  texture.rs        <- типаж TextureAtlas
  font.rs          <- типаж Font
```

**Важно:** Это типажи! Конкретной реализации `wgpu` здесь нет.

---

## **Уровень 6: Backend (реализация для wgpu+winit)**

```
src/backend/
  mod.rs
  wgpu/
    mod.rs
    painter.rs      <- impl Painter для wgpu
    texture.rs      <- impl TextureAtlas для wgpu
    shaders/        <- шейдеры на WGSL
    pipelines.rs    <- кэш пайплайнов

  winit/
    mod.rs
    window.rs       <- обертка над winit::Window
    event_loop.rs   <- преобразование событий winit

  integration.rs    <- связывает wgpu + winit + наш рендер
```

**Зависит от:** `render::*` (реализует его типажи), `wgpu`, `winit`

---

## **Уровень 7: Система (System) — координация всего**

```
src/system/
  mod.rs
  app_runner.rs     <- AppRunner (главный цикл)
  render_context.rs <- RenderContext (стили+лейаут+рендер)
  update_loop.rs    <- обработка сообщений, диффинг
  scheduler.rs      <- планировщик кадров (vsync, dirty rects)
```

**Сердце системы:** `RenderContext` знает о:

- `Styling::StyleEngine`
- `Layout::LayoutEngine`
- `Backend::Painter`

Но! Через абстракции (`render::Painter`), не конкретные имплементации.

---

## **Уровень 8: Внутренние утилиты (Internal)**

```
src/internal/
  mod.rs
  tree_diff.rs      <- алгоритм сравнения деревьев
  arena.rs          <- Arena аллокатор для нод
  cache.rs          <- LRU кэш
  profiling.rs      <- замеры производительности
```

---

## **Связи между модулями (строго однонаправленные):**

```
macros (0)
    ↓ (генерирует код для)
public (1)
    ↓ (использует)
core (2)
    ↑ (зависит от)
styling (3)    layout (4)    render (5)
    ↑              ↑             ↑
system (7) → использует все три через типажи
    ↑
backend (6) ← реализует render::Painter
```

**Правило:** Модуль может зависеть только от модулей с **меньшим номером**. Исключение: `backend` реализует типажи из `render`.

---

## **Как это собирается в `lib.rs`:**

```rust
// src/lib.rs
pub mod public {
    pub use widget::*;
    pub use component::*;
    pub use app::*;
}

pub mod prelude {
    pub use public::*;
    pub use my_ui_macros::*; // макросы
}

// Внутренние модули - не публичные
mod core;
mod styling;
mod layout;
mod render;
mod backend;
mod system;
mod internal;

// Публичная функция запуска
pub fn run_app<C: Component>(component: C) {
    let backend = backend::initialize();
    let system = system::AppRunner::new(backend);
    system.run(component);
}
```

---

## **Конкретный пример потока для кнопки:**

1. **Пользователь пишет:**

   ```rust
   html! { <button class="primary">"OK"</button> }
   ```

2. **Макрос генерирует:**

   ```rust
   Element::new("button")
       .class("primary")
       .child(Text::new("OK"))
   ```

3. **В системе:**
   - `StyleEngine` получает ноду, находит CSS правило `.primary { color: blue }`
   - `LayoutEngine` вычисляет размер кнопки (по содержимому + padding)
   - `RenderContext` создает `RenderCommand::Rect` и `RenderCommand::Text`
   - `Backend::Painter` конвертит команды в `wgpu` вызовы

---

## **Что тестировать отдельно:**

1. **Core + Styling** — без GPU и окон
   - Правильно ли применяются стили
   - Правильно ли работает селектор `.button.primary`

2. **Layout** — без GPU
   - Правильно ли рассчитываются размеры flex-контейнера

3. **Render commands** — без GPU
   - Правильно ли генерируются команды из styled+laidout ноды

4. **Backend** — интеграционные тесты
   - Шейдеры компилируются
   - Текстуры загружаются

---

## **Если хочешь минимизировать:**

Начни с трех модулей:

1. `core` — виджеты, дерево, стили (данные)
2. `render` — типажи для рисования + простая реализация (может выводить в SVG для отладки)
3. `system` — все в одном, но с четкими функциями: `style()`, `layout()`, `paint()`

Потом режь на более мелкие, когда заработает базовая версия.

Главное — сразу провести границу: **данные** (core), **логика** (system), **реализация** (backend). И между ними — только типажи.
