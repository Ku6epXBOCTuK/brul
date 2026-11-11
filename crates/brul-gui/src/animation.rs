use std::time::Duration;

use crate::Color;

pub fn ease_out(progress: f32) -> f32 {
    let rest = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - rest * rest * rest
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track {
    elapsed: Duration,
    duration: Duration,
}

impl Track {
    pub fn new(duration: Duration) -> Self {
        Self {
            elapsed: Duration::ZERO,
            duration,
        }
    }

    pub fn value(&self) -> f32 {
        ease_out(progress(self.elapsed, self.duration))
    }

    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn advance(&mut self, delta: Duration) -> bool {
        self.elapsed = (self.elapsed + delta).min(self.duration);
        !self.done()
    }
}

pub trait Tween: Copy {
    fn tween(self, to: Self, progress: f32) -> Self;
}

impl Tween for f32 {
    fn tween(self, to: Self, progress: f32) -> Self {
        self + (to - self) * progress
    }
}

impl Tween for Color {
    fn tween(self, to: Self, progress: f32) -> Self {
        self.lerp(to, progress)
    }
}

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{Track, Tween, ease_out};
    use crate::Color;
    use std::time::Duration;

    #[test]
    fn a_track_starts_at_zero_and_lands_exactly_on_one() {
        let mut track = Track::new(Duration::from_millis(300));
        assert_eq!(track.value(), 0.0);
        assert!(track.advance(Duration::from_millis(150)));
        assert!((track.value() - 0.875).abs() < 0.01);
        assert!(!track.advance(Duration::from_millis(150)));
        assert_eq!(track.value(), 1.0);
        assert!(track.done());
    }

    #[test]
    fn a_zero_length_track_is_done_at_once() {
        let mut track = Track::new(Duration::ZERO);
        assert!(track.done());
        assert_eq!(track.value(), 1.0);
        assert!(!track.advance(Duration::from_millis(16)));
    }

    #[test]
    fn ease_out_moves_faster_at_the_start() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.25) > 0.25);
        assert!(ease_out(0.75) > 0.75);
    }

    #[test]
    fn colors_and_numbers_tween_between_endpoints() {
        assert_eq!(0.0_f32.tween(10.0, 0.5), 5.0);
        assert_eq!(
            Color::BLACK.tween(Color::WHITE, 0.5),
            Color::rgb(128, 128, 128)
        );
    }
}
