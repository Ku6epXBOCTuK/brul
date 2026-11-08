//! bropicker: переключатель браузеров на BRUL.
//!
//! Конфиг — TOML из `%APPDATA%\bropicker\config.toml` (или `BP_CONFIG`).
//! Браузеры, правила доменов и запомненный выбор живут в [`config`].

pub mod config;

pub use config::{
    BrowserEntry, Config, ConfigError, Resolution, ResolveSource, Settings, Theme, default_path,
    domain_of,
};
