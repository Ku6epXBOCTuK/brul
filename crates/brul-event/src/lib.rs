//! `brul-event`: the shared vocabulary of raw input events and semantic UI events.

use std::fmt;
use std::str::FromStr;

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

macro_rules! event_family {
    (
        $event:ident, $kind:ident, $set:ident,
        [$($from:ident => $to:ident),* $(,)?],
        $((
            $variant:ident,
            $name:literal,
            $field:literal,
            $constructor:ident,
            ($($part:ident : $ty:ty),*),
            $arguments:literal
        )),+ $(,)?
    ) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum $event {
            $($variant { $($part: $ty),* }),+
        }

        impl $event {
            pub fn kind(&self) -> $kind {
                match self {
                    $(Self::$variant { .. } => $kind::$variant),+
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
        pub enum $kind {
            $($variant),+
        }

        impl $kind {
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

            pub const fn dispatch_kind(self) -> Self {
                match self {
                    $(Self::$from => Self::$to,)*
                    other => other,
                }
            }
        }

        impl fmt::Display for $kind {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.name())
            }
        }

        impl FromStr for $kind {
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
        pub struct $set(u64);

        impl $set {
            pub const fn empty() -> Self {
                Self(0)
            }

            pub const fn contains(self, kind: $kind) -> bool {
                self.0 & (1u64 << (kind.dispatch_kind() as u8)) != 0
            }

            pub fn insert(&mut self, kind: $kind) {
                self.0 |= 1u64 << (kind.dispatch_kind() as u8);
            }

            pub fn iter(self) -> impl Iterator<Item = $kind> {
                $kind::ALL
                    .iter()
                    .copied()
                    .filter(move |kind| kind.dispatch_kind() == *kind && self.contains(*kind))
            }
        }

        impl<const N: usize> From<[$kind; N]> for $set {
            fn from(kinds: [$kind; N]) -> Self {
                kinds.into_iter().collect()
            }
        }

        impl FromIterator<$kind> for $set {
            fn from_iter<T: IntoIterator<Item = $kind>>(kinds: T) -> Self {
                let mut set = Self::empty();
                for kind in kinds {
                    set.insert(kind);
                }
                set
            }
        }
    };
}

event_family! {
    RawEvent, RawKind, RawEventSet,
    [KeyNoop => Key],
    (PointerEnter, "enter", "PointerEnter", pointer_enter, (), 0),
    (PointerLeave, "leave", "PointerLeave", pointer_leave, (), 0),
    (
        PointerCancel,
        "pointer_cancel",
        "PointerCancel",
        pointer_cancel,
        (),
        0
    ),
    (
        PointerMove,
        "pointer_move",
        "PointerMove",
        pointer_move,
        (x: i32, y: i32, modifiers: KeyModifiers),
        0
    ),
    (
        Press,
        "press",
        "Press",
        press,
        (x: i32, y: i32, modifiers: KeyModifiers),
        0
    ),
    (
        Release,
        "release",
        "Release",
        release,
        (x: i32, y: i32, modifiers: KeyModifiers),
        0
    ),
    (Drag, "drag", "Drag", drag, (x: i32, y: i32), 0),
    (Key, "key", "Key", key, (key: KeyPress, modifiers: KeyModifiers), 0),
    (
        KeyNoop,
        "key_noop",
        "KeyNoop",
        key_noop,
        (key: KeyPress, modifiers: KeyModifiers),
        0
    ),
    (Ime, "ime", "Ime", ime, (kind: ImeKind), 0),
    (Paste, "paste", "Paste", paste, (text: String), 0),
    (Wheel, "wheel", "Wheel", wheel, (delta: i32), 0),
}

event_family! {
    Event, EventKind, EventSet,
    [],
    (Key, "key", "Key", key, (key: KeyPress, modifiers: KeyModifiers), 1),
    (Click, "click", "Click", click, (), 0),
    (Close, "close", "Close", close, (), 0),
    (
        Timer,
        "timer",
        "Timer",
        timer,
        (owner: brul_ident::TimerOwner, function: brul_ident::FunctionName),
        0
    ),
    (Focus, "focus", "Focus", focus, (), 0),
    (Blur, "blur", "Blur", blur, (), 0),
    (Input, "input", "Input", input, (value: String, caret: usize), 1),
    (Submit, "submit", "Submit", submit, (), 0),
    (Dismiss, "dismiss", "Dismiss", dismiss, (), 0),
    (Copy, "copy", "Copy", copy, (text: String), 0),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_fields_share_one_table() {
        for kind in RawKind::ALL {
            let name = kind.name();
            assert_eq!(name.parse::<RawKind>().unwrap(), *kind);
            assert_eq!(kind.field().parse::<RawKind>().unwrap(), *kind);
        }
        for kind in EventKind::ALL {
            let name = kind.name();
            assert_eq!(name.parse::<EventKind>().unwrap(), *kind);
            assert_eq!(kind.field().parse::<EventKind>().unwrap(), *kind);
        }
    }

    #[test]
    fn an_event_set_keeps_the_kinds_it_was_given() {
        let raw = RawEventSet::from([RawKind::Key, RawKind::Press]);
        assert_eq!(
            raw.iter().collect::<Vec<_>>(),
            vec![RawKind::Press, RawKind::Key]
        );
        let semantic = EventSet::from([EventKind::Input, EventKind::Click]);
        assert_eq!(
            semantic.iter().collect::<Vec<_>>(),
            vec![EventKind::Click, EventKind::Input]
        );
    }

    #[test]
    fn a_noop_raw_key_uses_the_key_subscription() {
        let set = RawEventSet::from([RawKind::Key]);
        assert!(set.contains(RawKind::KeyNoop));
        assert_eq!(RawKind::KeyNoop.dispatch_kind(), RawKind::Key);
    }
}
