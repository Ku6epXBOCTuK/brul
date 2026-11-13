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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Snapshot {
    pub enabled: bool,
    pub live: bool,
    pub active: bool,
    pub idle: Option<u64>,
    pub after_secs: u64,
    pub silent: bool,
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
        self.written = None;
    }

    pub fn busy(&mut self) {
        self.live = true;
    }

    pub fn free(&mut self) {
        self.live = false;
    }

    pub fn snapshot(&self, idle: Option<u64>) -> Snapshot {
        Snapshot {
            enabled: self.enabled,
            live: self.live,
            active: self.active,
            idle,
            after_secs: self.after.as_secs(),
            silent: self.text.trim().is_empty(),
        }
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
        if self.active {
            let retry = self
                .written
                .is_none_or(|written| now.saturating_duration_since(written) >= RETRY);
            if !retry {
                return None;
            }
        } else if !idle.is_some_and(|idle| Duration::from_secs(idle) >= self.after) {
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

pub fn describe(snapshot: Snapshot) -> String {
    if !snapshot.enabled || snapshot.silent {
        return String::from("AFK выключен");
    }
    let Some(idle) = snapshot.idle else {
        return String::from("простой недоступен");
    };
    if snapshot.active {
        return format!("AFK на экране · простой {}", clock(idle));
    }
    if snapshot.live {
        return format!("AFK на паузе: на экране заметка · простой {}", clock(idle));
    }
    format!(
        "простой {} · AFK через {}",
        clock(idle),
        clock(snapshot.after_secs.saturating_sub(idle))
    )
}

fn clock(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::{Action, Afk, Snapshot, describe};
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
    fn coming_back_keeps_the_text_until_cleared() {
        let mut afk = afk();
        let now = Instant::now();
        afk.poll(Some(5), now);
        assert_eq!(afk.poll(Some(0), now), None);
        assert_eq!(afk.poll(Some(0), now + Duration::from_secs(1)), None);
    }

    #[test]
    fn switching_afk_off_clears_the_text_once() {
        let mut afk = afk();
        afk.poll(Some(5), Instant::now());
        afk.set(false, String::from("away"));
        assert_eq!(afk.poll(Some(5), Instant::now()), Some(Action::Back));
        assert_eq!(afk.poll(Some(5), Instant::now()), None);
    }

    #[test]
    fn a_reset_idle_keeps_the_away_text_on_screen() {
        let mut afk = afk();
        let now = Instant::now();
        afk.poll(Some(5), now);
        assert_eq!(afk.poll(Some(0), now + Duration::from_secs(1)), None);
        assert_eq!(
            afk.poll(Some(0), now + Duration::from_secs(16)),
            Some(Action::Away)
        );
    }

    #[test]
    fn a_new_away_text_is_written_at_once() {
        let mut afk = afk();
        let now = Instant::now();
        afk.poll(Some(5), now);
        afk.set(true, String::from("back soon"));
        assert_eq!(afk.poll(Some(5), now), Some(Action::Away));
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

    fn snapshot() -> Snapshot {
        Snapshot {
            enabled: true,
            live: false,
            active: false,
            idle: Some(83),
            after_secs: 300,
            silent: false,
        }
    }

    #[test]
    fn a_countdown_names_the_idle_time_and_the_rest() {
        assert_eq!(describe(snapshot()), "простой 01:23 · AFK через 03:37");
    }

    #[test]
    fn a_live_note_pauses_the_countdown() {
        let live = Snapshot {
            live: true,
            ..snapshot()
        };
        assert_eq!(
            describe(live),
            "AFK на паузе: на экране заметка · простой 01:23"
        );
    }

    #[test]
    fn an_away_text_is_named_on_screen() {
        let active = Snapshot {
            active: true,
            ..snapshot()
        };
        assert_eq!(describe(active), "AFK на экране · простой 01:23");
    }

    #[test]
    fn a_disabled_or_silent_afk_is_off() {
        let off = Snapshot {
            enabled: false,
            ..snapshot()
        };
        assert_eq!(describe(off), "AFK выключен");
        let silent = Snapshot {
            silent: true,
            ..snapshot()
        };
        assert_eq!(describe(silent), "AFK выключен");
    }

    #[test]
    fn an_unknown_idle_time_is_named() {
        let unknown = Snapshot {
            idle: None,
            ..snapshot()
        };
        assert_eq!(describe(unknown), "простой недоступен");
    }

    #[test]
    fn a_pause_past_the_threshold_shows_no_time_left() {
        let away = Snapshot {
            idle: Some(400),
            ..snapshot()
        };
        assert_eq!(describe(away), "простой 06:40 · AFK через 00:00");
    }
}
