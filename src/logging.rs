//! Управляемое логирование движка.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};

use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use tracing::{debug, error, info, trace, warn};

static INIT: Once = Once::new();

/// Включает лог в файл; уровень задаёт `BRUL_LOG`, путь — `BRUL_LOG_FILE`.
///
/// По умолчанию уровень `info`, файл лежит в `~/.config/brul/<exe>.log` и
/// дописывается, поэтому зависание остаётся в нём после перезапуска.
pub fn init() {
    INIT.call_once(|| {
        let level = std::env::var("BRUL_LOG")
            .ok()
            .as_deref()
            .and_then(parse_level)
            .unwrap_or(LevelFilter::INFO);
        let stem = binary_stem();
        let path = std::env::var_os("BRUL_LOG_FILE")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| default_path(&stem));
        let Ok(file) = open(&path) else {
            return;
        };
        let file = fmt::layer()
            .with_writer(Mutex::new(file))
            .with_ansi(false)
            .with_filter(targets(level, &stem));
        let _ = tracing_subscriber::registry().with(file).try_init();
    });
}

fn parse_level(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "error" => Some(LevelFilter::ERROR),
        "warn" => Some(LevelFilter::WARN),
        "info" => Some(LevelFilter::INFO),
        "debug" => Some(LevelFilter::DEBUG),
        "trace" => Some(LevelFilter::TRACE),
        _ => None,
    }
}

fn binary_stem() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "brul".to_owned())
}

fn default_path(stem: &str) -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    log_path_for(home, stem)
}

fn log_path_for(home: Option<PathBuf>, stem: &str) -> PathBuf {
    match home {
        Some(home) => home
            .join(".config")
            .join("brul")
            .join(format!("{stem}.log")),
        None => PathBuf::from(format!("{stem}.log")),
    }
}

fn open(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::OpenOptions::new().create(true).append(true).open(path)
}

fn targets(level: LevelFilter, stem: &str) -> Targets {
    let mut targets = Targets::new()
        .with_target("brul", level)
        .with_target("brul_window", level)
        .with_target("brul_gui", level)
        .with_target("brul_script", level)
        .with_target("brul_syntax", level)
        .with_target("brul_codegen", level)
        .with_target("brul_reactive", level);
    if !stem.is_empty() {
        targets = targets.with_target(stem.to_owned(), level);
    }
    targets
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after the epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("brul-log-{}-{stamp}.log", std::process::id()))
    }

    fn file_subscriber(level: LevelFilter, file: File) -> impl tracing::Subscriber + Send + Sync {
        tracing_subscriber::registry().with(
            fmt::layer()
                .with_writer(Mutex::new(file))
                .with_ansi(false)
                .with_filter(level),
        )
    }

    #[test]
    fn levels_are_parsed_by_name() {
        assert_eq!(parse_level("DEBUG"), Some(LevelFilter::DEBUG));
        assert_eq!(parse_level(" warn "), Some(LevelFilter::WARN));
        assert_eq!(parse_level("verbose"), None);
    }

    #[test]
    fn the_log_lives_in_the_engine_directory() {
        let home = PathBuf::from("C:\\Users\\tester");
        assert_eq!(
            log_path_for(Some(home.clone()), "bropicker"),
            home.join(".config").join("brul").join("bropicker.log")
        );
        assert_eq!(
            log_path_for(None, "bropicker"),
            PathBuf::from("bropicker.log")
        );
    }

    #[test]
    fn a_file_layer_writes_events_at_its_level() {
        let path = temp_path();
        let subscriber =
            file_subscriber(LevelFilter::DEBUG, File::create(&path).expect("log file"));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("a debug line");
            tracing::info!("an info line");
        });

        let text = fs::read_to_string(&path).expect("written log");
        let _ = fs::remove_file(&path);
        assert!(text.contains("a debug line"));
        assert!(text.contains("an info line"));
    }

    #[test]
    fn a_file_layer_drops_events_below_its_level() {
        let path = temp_path();
        let subscriber = file_subscriber(LevelFilter::WARN, File::create(&path).expect("log file"));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("a debug line");
            tracing::warn!("a warning line");
        });

        let text = fs::read_to_string(&path).expect("written log");
        let _ = fs::remove_file(&path);
        assert!(!text.contains("a debug line"));
        assert!(text.contains("a warning line"));
    }
}
