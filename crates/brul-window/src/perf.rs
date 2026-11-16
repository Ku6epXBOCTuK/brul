//! Точки замера: `measure` получает сброс счётчиков после первого нарисованного
//! кадра и отчёт после остановки цикла событий.

use std::sync::{Mutex, MutexGuard, PoisonError};

static AFTER_FIRST_FRAME: Mutex<Option<fn()>> = Mutex::new(None);
static ON_EXIT: Mutex<Option<fn()>> = Mutex::new(None);

pub fn measure(reset: fn(), report: fn()) {
    *lock(&AFTER_FIRST_FRAME) = Some(reset);
    *lock(&ON_EXIT) = Some(report);
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

fn lock(slot: &'static Mutex<Option<fn()>>) -> MutexGuard<'static, Option<fn()>> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}
