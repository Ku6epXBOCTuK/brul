//! Точки замера: `measure` получает сброс счётчиков после первого нарисованного
//! кадра и отчёт после остановки цикла событий, `start` и `startup` меряют путь
//! до первого кадра.

use std::ffi::OsStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Instant;

static AFTER_FIRST_FRAME: Mutex<Option<fn()>> = Mutex::new(None);
static ON_EXIT: Mutex<Option<fn()>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();
static LOGGED: AtomicBool = AtomicBool::new(false);

pub fn measure(reset: fn(), report: fn()) {
    *lock(&AFTER_FIRST_FRAME) = Some(reset);
    *lock(&ON_EXIT) = Some(report);
}

pub(crate) fn start() {
    START.get_or_init(Instant::now);
}

pub(crate) fn startup(scale: f64, width: u32, height: u32) {
    if !wanted() || LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(start) = START.get() else {
        return;
    };
    let millis = start.elapsed().as_secs_f64() * 1000.0;
    tracing::info!(
        target: "brul",
        "brul startup: first frame in {millis:.1} ms, scale {scale:.2}, surface {width}x{height}"
    );
}

pub(crate) fn first_frame() {
    if let Some(callback) = lock(&AFTER_FIRST_FRAME).take() {
        callback();
    }
}

pub(crate) fn exit() {
    if let Some(callback) = lock(&ON_EXIT).take() {
        callback();
    }
}

fn wanted() -> bool {
    static WANTED: OnceLock<bool> = OnceLock::new();
    *WANTED.get_or_init(|| enabled(std::env::var_os("BRUL_STARTUP_LOG").as_deref()))
}

fn enabled(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

fn lock(slot: &'static Mutex<Option<fn()>>) -> MutexGuard<'static, Option<fn()>> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::enabled;

    use std::ffi::OsStr;

    #[test]
    fn the_startup_line_is_on_for_a_set_variable_only() {
        assert!(enabled(Some(OsStr::new("1"))));
        assert!(enabled(Some(OsStr::new("yes"))));
        assert!(!enabled(Some(OsStr::new("0"))));
        assert!(!enabled(Some(OsStr::new(""))));
        assert!(!enabled(None));
    }
}
