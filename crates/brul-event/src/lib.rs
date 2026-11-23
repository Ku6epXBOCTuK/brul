//! `brul-event`: the shared vocabulary of `Event` payloads, `EventKind` names and `EventSet` subscriptions.

use std::fmt;
use std::str::FromStr;

use brul_ident::{FunctionName, TimerOwner};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyPress {
    pub code: String,
    pub physical: Option<String>,
}

impl From<&str> for KeyPress {
    fn from(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            physical: None,
        }
    }
}

impl From<String> for KeyPress {
    fn from(code: String) -> Self {
        Self {
            code,
            physical: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CaretArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImeKind {
    Enabled,
    Preedit(String),
    Commit(String),
    DeleteSurrounding {
        before_bytes: usize,
        after_bytes: usize,
    },
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownEvent(pub String);

impl fmt::Display for UnknownEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown event '{}'", self.0)
    }
}

impl std::error::Error for UnknownEvent {}

macro_rules! events {
    ($((
        $variant:ident,
        $name:literal,
        $field:literal,
        $constructor:ident,
        ($($part:ident : $ty:ty),*),
        $arguments:literal
    )),+ $(,)?) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum Event {
            $($variant { $($part: $ty),* }),+
        }

        impl Event {
            pub fn kind(&self) -> EventKind {
                match self {
                    $(Self::$variant { .. } => EventKind::$variant),+
                }
            }

            $(
                pub fn $constructor($($part: $ty),*) -> Self {
                    Self::$variant { $($part),* }
                }
            )+
        }

        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(u8)]
        pub enum EventKind {
            $($variant),+
        }

        impl EventKind {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }

            pub const fn field(self) -> &'static str {
                match self {
                    $(Self::$variant => $field),+
                }
            }

            pub const fn arguments_count(self) -> usize {
                match self {
                    $(Self::$variant => $arguments),+
                }
            }
        }

        impl fmt::Display for EventKind {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.name())
            }
        }

        impl FromStr for EventKind {
            type Err = UnknownEvent;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                $(
                    if value.eq_ignore_ascii_case($name) || value.eq_ignore_ascii_case($field) {
                        return Ok(Self::$variant);
                    }
                )+
                Err(UnknownEvent(value.to_owned()))
            }
        }

        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub struct EventSet(u64);

        impl EventSet {
            pub const fn empty() -> Self {
                Self(0)
            }

            pub const fn contains(self, kind: EventKind) -> bool {
                self.0 & (1u64 << (kind as u8)) != 0
            }

            pub fn insert(&mut self, kind: EventKind) {
                self.0 |= 1u64 << (kind as u8);
            }

            pub fn iter(self) -> impl Iterator<Item = EventKind> {
                EventKind::ALL
                    .iter()
                    .copied()
                    .filter(move |kind| self.contains(*kind))
            }
        }

        impl<const N: usize> From<[EventKind; N]> for EventSet {
            fn from(kinds: [EventKind; N]) -> Self {
                kinds.into_iter().collect()
            }
        }

        impl FromIterator<EventKind> for EventSet {
            fn from_iter<T: IntoIterator<Item = EventKind>>(kinds: T) -> Self {
                let mut set = Self::empty();
                for kind in kinds {
                    set.insert(kind);
                }
                set
            }
        }
    };
}

events! {
    (PointerEnter, "enter", "PointerEnter", pointer_enter, (), 0),
    (PointerLeave, "leave", "PointerLeave", pointer_leave, (), 0),
    (Press, "press", "Press", press, (), 0),
    (Release, "release", "Release", release, (), 0),
    (Click, "click", "Click", click, (), 0),
    (Drag, "drag", "Drag", drag, (), 0),
    (Key, "key", "Key", key, (key: KeyPress, modifiers: KeyModifiers), 1),
    (Close, "close", "Close", close, (), 0),
    (Timer, "timer", "Timer", timer, (owner: TimerOwner, function: FunctionName), 0),
    (Focus, "focus", "Focus", focus, (), 0),
    (Blur, "blur", "Blur", blur, (), 0),
    (Input, "input", "Input", input, (value: String, caret: usize), 1),
    (Ime, "ime", "Ime", ime, (kind: ImeKind), 0),
    (Submit, "submit", "Submit", submit, (), 0),
    (Dismiss, "dismiss", "Dismiss", dismiss, (), 0),
    (Copy, "copy", "Copy", copy, (text: String), 0),
    (Paste, "paste", "Paste", paste, (), 0),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_fields_share_one_table() {
        for kind in EventKind::ALL {
            assert_eq!(kind.to_string().parse::<EventKind>().unwrap(), *kind);
            assert_eq!(kind.field().parse::<EventKind>().unwrap(), *kind);
        }
        assert_eq!(EventKind::PointerEnter.name(), "enter");
        assert_eq!(
            Event::key(KeyPress::from("a"), KeyModifiers::default()).kind(),
            EventKind::Key
        );
    }

    #[test]
    fn an_event_set_keeps_the_kinds_it_was_given() {
        let set = EventSet::from([EventKind::Key, EventKind::Click]);
        assert!(set.contains(EventKind::Key));
        assert!(set.contains(EventKind::Click));
        assert!(!set.contains(EventKind::Ime));
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [EventKind::Click, EventKind::Key]
        );
    }
}
