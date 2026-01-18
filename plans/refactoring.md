# План рефакторинга BRUL GUI Library

## **Текущее состояние проекта**

**Workspace структура:**

- `crates/brul-core/` - базовый крейт (пустой)
- `crates/brul-gui/` - GUI крейт (пустой)
- `crates/brul-macros/` - макросы
- `src/` - старые исходники (app.rs, app_old.rs, core.rs, events.rs, render.rs, widget.rs, brul.rs, event_loop.rs, lib.rs)

**Анализ старых исходников в `src/`:**

- `core.rs` - базовые типы: `Point`, `Size`, `Rect`, `Edges`, `Color`
- `events.rs` - `Event` enum и `EventContext` для обработки событий
- `render.rs` - `Renderer` struct (wgpu инициализация и рендеринг)
- `widget.rs` - `Widget` trait и `Rectangle` implementation
- `app.rs` / `app_old.rs` - `App` struct и event loop logic
- `brul.rs` - `Brul` builder pattern для состояния и задач
- `event_loop.rs` - закомментированный код (можно удалить)
- `lib.rs` - реэкспорт всех модулей

## **Проблема циклических зависимостей**

Point, Size, Rect, Color нужны И в brul-core (для Brul builder) И в brul-gui (для рендеринга и UI).
Если вынести их в brul-core → brul-gui зависит от brul-core → возможно, brul-core тоже понадобится GUI типы.

## **Решение: Выделить utils крейт**

### **Новая архитектура:**

```
crates/
├── brul-utils/             # Утилиты: базовые типы (без зависимостей)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── math/
│       │   ├── mod.rs
│       │   ├── geometry.rs # Point, Size, Rect, Edges
│       │   └── color.rs    # Color
│       └── events/
│           ├── mod.rs
│           └── event.rs    # Event, EventContext (без winit)
│
├── brul-core/              # Ядро: Brul builder + tokio
│   ├── Cargo.toml          # Зависит от brul-utils
│   └── src/
│       ├── lib.rs
│       └── brul/
│           ├── mod.rs
│           └── builder.rs  # Brul struct
│
├── brul-gui/               # GUI: рендеринг и UI
│   ├── Cargo.toml          # Зависит от brul-utils + wgpu/winit
│   └── src/
│       ├── lib.rs
│       ├── application/    # Приложение
│       │   ├── mod.rs
│       │   └── app.rs
│       ├── render/         # Рендеринг
│       │   ├── mod.rs
│       │   └── renderer.rs
│       └── ui/             # UI виджеты
│           ├── mod.rs
│           ├── widget.rs
│           └── rectangle.rs
│
└── brul-macros/            # Макросы
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

### **Что хранить в utils?**

**В `brul-utils`:**

- ✅ Базовые типы данных (Point, Size, Rect, Edges, Color)
- ✅ Event system (без привязки к winit)
- ✅ Математические функции
- ✅ Константы
- ✅ Утилитарные функции

**НЕ хранить в utils:**

- ❌ Platform-specific код (winit, wgpu)
- ❌ Async runtime (tokio)
- ❌ UI компоненты
- ❌ Сложная бизнес-логика

### **Зависимости после рефакторинга:**

```
brul-utils  (без внешних зависимостей)
    ↓
brul-core   (depends: brul-utils, tokio)
    ↓
brul-gui    (depends: brul-utils, wgpu, winit)
```

**Никаких циклических зависимостей!**

## **Пошаговый план рефакторинга**

### **Шаг 1: Создать brul-utils**

1. Создать `crates/brul-utils/src/math/geometry.rs` - Point, Size, Rect, Edges
2. Создать `crates/brul-utils/src/math/color.rs` - Color с константами
3. Создать `crates/brul-utils/src/math/mod.rs` - реэкспорт
4. Создать `crates/brul-utils/src/events/event.rs` - Event, EventContext (без winit)
5. Создать `crates/brul-utils/src/events/mod.rs` - реэкспорт
6. Создать `crates/brul-utils/src/lib.rs` - реэкспорт всех модулей
7. Обновить `crates/brul-utils/Cargo.toml`

### **Шаг 2: Обновить brul-core**

1. Скопировать `src/brul.rs` → `crates/brul-core/src/brul/builder.rs`
2. Создать `crates/brul-core/src/brul/mod.rs`
3. Создать `crates/brul-core/src/lib.rs`
4. Добавить зависимость `brul-utils = { path = "../brul-utils" }` в Cargo.toml
5. Обновить импорты в Brul (использовать типы из brul-utils)

### **Шаг 3: Создать brul-gui**

1. `crates/brul-gui/src/render/renderer.rs` - Renderer из src/render.rs
2. `crates/brul-gui/src/ui/widget.rs` - Widget trait
3. `crates/brul-gui/src/ui/rectangle.rs` - Rectangle struct
4. `crates/brul-gui/src/application/app.rs` - App из src/app_old.rs + логика из src/app.rs
5. Создать модульные файлы (mod.rs)
6. Обновить `crates/brul-gui/src/lib.rs`
7. Добавить зависимости: `brul-utils`, `wgpu`, `winit`

### **Шаг 4: Интеграция и очистка**

1. Обновить корневой `Cargo.toml` - проверить members
2. Проверить компиляцию workspace
3. Создать примеры в `examples/`
4. Переместить старые файлы в `examples/legacy/` или удалить
5. Обновить документацию

## **Альтернатива: facade крейт**

Если хотите упростить для пользователей, можно добавить `brul` крейт:

```
crates/
├── brul-utils/
├── brul-core/
├── brul-gui/
└── brul/                    # Facade (re-export всех публичных типов)
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

В `brul/src/lib.rs`:

```rust
pub use brul_utils::{Point, Size, Rect, Color, Event};
pub use brul_core::Brul;
pub use brul_gui::{App, Renderer, Widget, Rectangle};
```

Тогда пользователи импортируют только `brul`, а внутренняя структура чистая.

## **Рекомендация:**

**Используйте Решение 1 (brul-utils + brul-core + brul-gui)**, потому что:

- ✅ Четкое разделение ответственности
- ✅ Нет циклических зависимостей
- ✅ Можно использовать utils отдельно
- ✅ Легко тестировать
- ✅ Понятная архитектура

**Если нужен один крейт для пользователей** → добавьте facade `brul`.
