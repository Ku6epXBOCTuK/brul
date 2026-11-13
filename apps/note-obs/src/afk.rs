//! Простой: сколько человек не трогал клавиатуру и мышь.

use std::time::{Duration, Instant};

const RETRY: Duration = Duration::from_secs(15);

#[cfg(windows)]
pub fn idle_secs() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return None;
    }
    let now = unsafe { GetTickCount() };
    Some(u64::from(now.wrapping_sub(info.dwTime)) / 1000)
}

#[cfg(not(windows))]
pub fn idle_secs() -> Option<u64> {
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Away,
    Back,
}

pub struct Afk {
    enabled: bool,
    text: String,
    after: Duration,
    live: bool,
    active: bool,
    written: Option<Instant>,
}

impl Afk {
    pub fn new(enabled: bool, text: String, after_secs: u64) -> Self {
        Self {
            enabled,
            text,
            after: Duration::from_secs(after_secs),
            live: false,
            active: false,
            written: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set(&mut self, enabled: bool, text: String) {
        self.enabled = enabled;
        self.text = text;
        self.active = false;
        self.written = None;
    }

    pub fn busy(&mut self) {
        self.live = true;
    }

    pub fn free(&mut self) {
        self.live = false;
    }

    pub fn poll(&mut self, idle: Option<u64>, now: Instant) -> Option<Action> {
        if self.live {
            self.active = false;
            self.written = None;
            return None;
        }
        if !self.enabled || self.text.is_empty() {
            return self.back();
        }
        let away = idle.is_some_and(|idle| Duration::from_secs(idle) >= self.after);
        if !away {
            return self.back();
        }
        let retry = self
            .written
            .is_some_and(|written| now.saturating_duration_since(written) >= RETRY);
        if self.active && !retry {
            return None;
        }
        self.active = true;
        self.written = Some(now);
        Some(Action::Away)
    }

    fn back(&mut self) -> Option<Action> {
        if !std::mem::take(&mut self.active) {
            return None;
        }
        self.written = None;
        Some(Action::Back)
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Afk};
    use std::time::{Duration, Instant};

    fn afk() -> Afk {
        Afk::new(true, String::from("away"), 5)
    }

    #[test]
    fn a_short_pause_stays_idle() {
        let mut afk = afk();
        assert_eq!(afk.poll(Some(4), Instant::now()), None);
    }

    #[test]
    fn a_long_pause_goes_away_once() {
        let mut afk = afk();
        let now = Instant::now();
        assert_eq!(afk.poll(Some(5), now), Some(Action::Away));
        assert_eq!(afk.poll(Some(9), now + Duration::from_secs(1)), None);
    }

    #[test]
    fn an_away_text_is_rewritten_after_a_retry() {
        let mut afk = afk();
        let now = Instant::now();
        assert_eq!(afk.poll(Some(5), now), Some(Action::Away));
        assert_eq!(
            afk.poll(Some(60), now + Duration::from_secs(20)),
            Some(Action::Away)
        );
    }

    #[test]
    fn coming_back_clears_the_text_once() {
        let mut afk = afk();
        let now = Instant::now();
        afk.poll(Some(5), now);
        assert_eq!(afk.poll(Some(0), now), Some(Action::Back));
        assert_eq!(afk.poll(Some(0), now), None);
    }

    #[test]
    fn a_manual_note_keeps_afk_off() {
        let mut afk = afk();
        afk.busy();
        assert_eq!(afk.poll(Some(600), Instant::now()), None);
        afk.free();
        assert_eq!(afk.poll(Some(600), Instant::now()), Some(Action::Away));
    }

    #[test]
    fn a_disabled_or_empty_afk_does_nothing() {
        let mut off = Afk::new(false, String::from("away"), 5);
        assert_eq!(off.poll(Some(600), Instant::now()), None);
        let mut empty = Afk::new(true, String::new(), 5);
        assert_eq!(empty.poll(Some(600), Instant::now()), None);
    }
}
