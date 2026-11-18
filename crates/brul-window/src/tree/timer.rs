use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_gui::WidgetEvent;
use winit::event_loop::ControlFlow;

use crate::window::WindowCommand;

pub(crate) struct Timer {
    pub(super) next: Instant,
    pub(super) interval: Duration,
    pub(super) function: Arc<str>,
}

impl Timer {
    pub(super) fn new(interval: Duration, function: Arc<str>) -> Self {
        Self {
            next: Instant::now() + interval,
            interval,
            function,
        }
    }
}

pub(crate) fn due(timers: &mut [Timer], now: Instant) -> Vec<WidgetEvent> {
    let mut events = Vec::new();
    for timer in timers {
        if timer.next > now {
            continue;
        }
        events.push(WidgetEvent::Timer(Arc::clone(&timer.function)));
        while timer.next <= now {
            timer.next += timer.interval;
        }
    }
    events
}

#[cfg(test)]
fn wakeup(timers: &[Timer], frame: Option<Instant>) -> ControlFlow {
    let timer = timers.iter().map(|timer| timer.next).min();
    wakeup_at(earliest(timer, frame))
}

pub(crate) fn wakeup_at(next: Option<Instant>) -> ControlFlow {
    match next {
        Some(next) => ControlFlow::WaitUntil(next),
        None => ControlFlow::Wait,
    }
}

pub(super) fn earliest(first: Option<Instant>, second: Option<Instant>) -> Option<Instant> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (first, second) => first.or(second),
    }
}

pub(super) fn split_initial(commands: Vec<WindowCommand>) -> (Vec<Timer>, Vec<WindowCommand>) {
    let mut timers = Vec::new();
    let mut rest = Vec::new();
    for command in commands {
        match command {
            WindowCommand::Every { interval, function } => {
                timers.push(Timer::new(interval, function.into()));
            }
            command => rest.push(command),
        }
    }
    (timers, rest)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use brul_gui::WidgetEvent;
    use winit::event_loop::ControlFlow;

    use super::{Timer, due, wakeup};

    #[test]
    fn a_new_timer_waits_for_its_interval() {
        let timer = Timer::new(Duration::from_secs(5), "later".into());
        assert!(timer.next > Instant::now());
        assert_eq!(timer.interval, Duration::from_secs(5));
    }

    #[test]
    fn a_due_timer_fires_once_and_moves_past_now() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now,
            interval: Duration::from_millis(10),
            function: "tick".into(),
        }];

        let events = due(&mut timers, now);

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], WidgetEvent::Timer(name) if name.as_ref() == "tick"));
        assert!(timers[0].next > now);
        assert!(due(&mut timers, now).is_empty());
    }

    #[test]
    fn a_due_timer_reuses_the_function_allocation() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now,
            interval: Duration::from_millis(10),
            function: "tick".into(),
        }];

        let events = due(&mut timers, now);

        assert!(
            matches!(&events[0], WidgetEvent::Timer(name) if Arc::ptr_eq(name, &timers[0].function))
        );
    }

    #[test]
    fn a_timer_in_the_future_stays_silent() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now + Duration::from_secs(1),
            interval: Duration::from_secs(1),
            function: "late".into(),
        }];
        assert!(due(&mut timers, now).is_empty());
    }

    #[test]
    fn the_loop_waits_for_the_earliest_timer() {
        let now = Instant::now();
        assert_eq!(wakeup(&[], None), ControlFlow::Wait);

        let mut timers = vec![
            Timer {
                next: now + Duration::from_secs(2),
                interval: Duration::from_secs(1),
                function: "far".into(),
            },
            Timer {
                next: now + Duration::from_secs(1),
                interval: Duration::from_secs(1),
                function: "near".into(),
            },
        ];
        assert_eq!(
            wakeup(&timers, None),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );

        timers[0].next = now + Duration::from_secs(1);
        assert_eq!(
            wakeup(&timers, None),
            ControlFlow::WaitUntil(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn a_frame_deadline_wakes_the_loop_before_a_later_timer() {
        let now = Instant::now();
        let frame = now + Duration::from_millis(16);
        let timers = vec![Timer {
            next: now + Duration::from_secs(1),
            interval: Duration::from_secs(1),
            function: "late".into(),
        }];

        assert_eq!(wakeup(&timers, Some(frame)), ControlFlow::WaitUntil(frame));
        assert_eq!(wakeup(&[], Some(frame)), ControlFlow::WaitUntil(frame));
    }
}
