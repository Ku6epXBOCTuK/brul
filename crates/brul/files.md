```
src/
├── lib.rs             # Реэкспорт публичного API
├── app.rs             # Публичные структуры App, AppBuilder, AppHandle
├── app/
│   ├── builder.rs     # Реализация AppBuilder
│   ├── core.rs        # Реализация App
│   ├── handle.rs      # Реализация AppHandle
│   └── manager.rs     # AppManager trait
├── runtime.rs         # Runtime, RuntimeHandle, платформенная абстракция
├── state.rs           # StateManager, StateHandle
├── window.rs          # WindowManager, WindowHandle, Window
├── config.rs          # Config структура
└── error.rs           # Error enum и Result type (опционально)
```

**Пояснения:**

### `lib.rs`

```rust
pub mod app;
pub mod config;
pub mod runtime;
pub mod state;
pub mod window;
pub mod error;

pub use app::{App, AppBuilder, AppHandle, AppManager};
pub use config::Config;
pub use error::{Error, Result};
```

### `app.rs`

- `App` - основная структура приложения
- `AppBuilder` - билдер с setup хуками
- `AppHandle` - легковесный дескриптор
- `AppManager` - общий трейт для App и AppHandle
- `AppCommand` - enum команд управления

### `runtime.rs`

- `Runtime` - платформенный рантайм (event loop)
- `RuntimeHandle` - дескриптор для доступа к runtime
- `PlatformEvent` - системные события

### `state.rs`

- `StateManager` - управление состоянием
- `StateHandle` - read-only доступ к состоянию

### `window.rs`

- `WindowManager` - менеджер окон
- `WindowHandle` - дескриптор окна
- `Window` - структура окна
- `WindowConfig` - конфигурация окна

### `config.rs`

- `Config` - основная конфигурация приложения
- Включает дефолтные настройки окон

### `error.rs` (опционально, можно и в app.rs)

- `Error` - enum ошибок
- `Result<T> = std::result::Result<T, Error>`
