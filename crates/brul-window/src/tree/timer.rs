use std::time::{Duration, Instant};

use brul_gui::{DispatchEvent, Event, UiEvent};
use brul_ident::{FunctionName, TimerId, TimerKind, TimerOwner};
use winit::event_loop::ControlFlow;

use crate::window::{TimerCommand, WindowCommand};

pub(crate) struct Timer {
    pub(super) next: Instant,
    pub(super) interval: Duration,
    pub(super) owner: TimerOwner,
    pub(super) function: FunctionName,
    pub(super) id: TimerId,
    pub(super) kind: TimerKind,
}

impl Timer {
    pub(super) fn new(
        id: TimerId,
        interval: Duration,
        owner: TimerOwner,
        function: FunctionName,
        kind: TimerKind,
    ) -> Self {
        Self {
            next: Instant::now() + interval,
            interval,
            owner,
            function,
            id,
            kind,
        }
    }
}

pub(crate) fn due(timers: &mut Vec<Timer>, now: Instant) -> Vec<DispatchEvent> {
    let mut events = Vec::new();
    timers.retain_mut(|timer| {
        if timer.next > now {
            return true;
        }
        events.push(DispatchEvent::Semantic(UiEvent::window(Event::timer(
            timer.owner,
            timer.function.clone(),
        ))));
        if timer.kind == TimerKind::Once {
            return false;
        }
        while timer.next <= now {
            timer.next += timer.interval;
        }
        true
    });
    events
}

pub(crate) fn cancel(timers: &mut Vec<Timer>, owner: TimerOwner, id: TimerId) {
    timers.retain(|timer| timer.owner != owner || timer.id != id);
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
            WindowCommand::Timer(TimerCommand::Start {
                id,
                owner,
                function,
                interval,
                kind,
            }) => {
                timers.push(Timer::new(id, interval, owner, function, kind));
            }
            WindowCommand::Timer(TimerCommand::Cancel { id, owner }) => {
                cancel(&mut timers, owner, id);
            }
            command => rest.push(command),
        }
    }
    (timers, rest)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use brul_gui::{DispatchEvent, Event, UiEvent};
    use brul_ident::{TimerId, TimerKind, TimerOwner};
    use winit::event_loop::ControlFlow;

    use super::{Timer, due, wakeup};

    #[test]
    fn a_new_timer_waits_for_its_interval() {
        let timer = Timer::new(
            TimerId::new(1),
            Duration::from_secs(5),
            TimerOwner::new(0),
            "later".into(),
            TimerKind::Every,
        );
        assert!(timer.next > Instant::now());
        assert_eq!(timer.interval, Duration::from_secs(5));
    }

    #[test]
    fn a_due_timer_fires_once_and_moves_past_now() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now,
            interval: Duration::from_millis(10),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            id: TimerId::new(1),
            kind: TimerKind::Every,
        }];

        let events = due(&mut timers, now);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            DispatchEvent::Semantic(UiEvent {
                event: Event::Timer { function, .. },
                ..
            }) if function.as_ref() == "tick"
        ));
        assert!(timers[0].next > now);
        assert!(due(&mut timers, now).is_empty());
    }

    #[test]
    fn a_due_timer_reuses_the_function_allocation() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now,
            interval: Duration::from_millis(10),
            owner: TimerOwner::new(0),
            function: "tick".into(),
            id: TimerId::new(1),
            kind: TimerKind::Every,
        }];

        let events = due(&mut timers, now);

        assert!(matches!(
            &events[0],
            DispatchEvent::Semantic(UiEvent {
                event: Event::Timer { function, .. },
                ..
            }) if function.as_str().as_ptr() == timers[0].function.as_str().as_ptr()
        ));
    }

    #[test]
    fn a_timer_in_the_future_stays_silent() {
        let now = Instant::now();
        let mut timers = vec![Timer {
            next: now + Duration::from_secs(1),
            interval: Duration::from_secs(1),
            owner: TimerOwner::new(0),
            function: "late".into(),
            id: TimerId::new(1),
            kind: TimerKind::Every,
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
                owner: TimerOwner::new(0),
                function: "far".into(),
                id: TimerId::new(1),
                kind: TimerKind::Every,
            },
            Timer {
                next: now + Duration::from_secs(1),
                interval: Duration::from_secs(1),
                owner: TimerOwner::new(0),
                function: "near".into(),
                id: TimerId::new(2),
                kind: TimerKind::Every,
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
            owner: TimerOwner::new(0),
            function: "late".into(),
            id: TimerId::new(1),
            kind: TimerKind::Every,
        }];

        assert_eq!(wakeup(&timers, Some(frame)), ControlFlow::WaitUntil(frame));
        assert_eq!(wakeup(&[], Some(frame)), ControlFlow::WaitUntil(frame));
    }
}
