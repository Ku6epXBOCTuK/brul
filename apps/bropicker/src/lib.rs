//! bropicker: переключатель браузеров на BRUL.
//!
//! Конфиг — TOML из `%APPDATA%\bropicker\config.toml` (или `BP_CONFIG`).
//! Браузеры, правила доменов и запомненный выбор живут в [`config`]; запуск
//! процесса без shell — в [`launcher`].

pub mod config;
pub mod generated;
pub mod launcher;

pub use config::{
    BrowserEntry, Config, ConfigError, Resolution, ResolveSource, Settings, Theme, default_path,
    demo_browsers, domain_of,
};
pub use launcher::{
    CommandSpec, LaunchError, Launcher, Runner, SystemRunner, command_for, split_flags,
};
