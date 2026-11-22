//! Measurement points: `measure` gets a counter reset after the first drawn
//! frame and a report after the event loop stops; `start` and `startup` time
//! the path to the first frame.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use crate::window::Backend;

static AFTER_FIRST_FRAME: Mutex<Option<fn()>> = Mutex::new(None);
static ON_EXIT: Mutex<Option<fn()>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();
static LOGGED: AtomicBool = AtomicBool::new(false);
static RASTER: Mutex<Raster> = Mutex::new(Raster {
    backend: None,
    first: None,
    samples: Vec::new(),
});

struct Raster {
    backend: Option<Backend>,
    first: Option<f64>,
    samples: Vec<f64>,
}

pub fn measure(reset: fn(), report: fn()) {
    *lock(&AFTER_FIRST_FRAME) = Some(reset);
    *lock(&ON_EXIT) = Some(report);
}

pub(crate) fn start() {
    START.get_or_init(Instant::now);
}

pub(crate) fn startup(backend: crate::window::Backend, scale: f64, width: u32, height: u32) {
    if !wanted() || LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(start) = START.get() else {
        return;
    };
    let millis = start.elapsed().as_secs_f64() * 1000.0;
    tracing::info!(
        target: "brul",
        "brul startup: first frame in {millis:.1} ms, backend {backend}, scale {scale:.2}, surface {width}x{height}"
    );
}

pub(crate) fn first_frame() {
    if let Some(callback) = lock(&AFTER_FIRST_FRAME).take() {
        callback();
    }
}

pub(crate) fn raster(backend: Backend, duration: Duration) {
    let millis = duration.as_secs_f64() * 1000.0;
    let mut raster = lock(&RASTER);
    raster.backend = Some(backend);
    if raster.first.is_none() {
        raster.first = Some(millis);
    } else {
        raster.samples.push(millis);
    }
}

pub(crate) fn measuring() -> bool {
    wanted()
}

pub(crate) fn next_frame() -> Option<Instant> {
    wanted().then(|| Instant::now() + Duration::from_millis(16))
}

pub(crate) fn dump_path() -> Option<PathBuf> {
    std::env::var_os("BRUL_FRAME_DUMP")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn exit() {
    if let Some(callback) = lock(&ON_EXIT).take() {
        callback();
    }
    report_raster();
}

fn report_raster() {
    if !wanted() {
        return;
    }
    let raster = lock(&RASTER);
    let (Some(backend), Some(first)) = (raster.backend, raster.first) else {
        return;
    };
    if raster.samples.is_empty() {
        tracing::info!(target: "brul", "brul raster: backend {backend}, first {first:.1} ms, no steady frames");
        return;
    }
    let mut samples = raster.samples.clone();
    samples.sort_by(f64::total_cmp);
    tracing::info!(
        target: "brul",
        "brul raster: backend {backend}, first {first:.1} ms, steady median {:.1} ms, min {:.1} ms, max {:.1} ms over {} frames",
        samples[samples.len() / 2], samples[0], samples[samples.len() - 1], samples.len()
    );
}

fn wanted() -> bool {
    static WANTED: OnceLock<bool> = OnceLock::new();
    *WANTED.get_or_init(|| enabled(std::env::var_os("BRUL_STARTUP_LOG").as_deref()))
}

fn enabled(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

fn lock<T>(slot: &'static Mutex<T>) -> MutexGuard<'static, T> {
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
