//! Модель стиля BRUL: имена свойств, значения, правила и каскад.

#[macro_export]
macro_rules! states {
    ($callback:ident) => {
        $callback! {
            (Selected, Selected, selected, "selected"),
            (Checked, Checked, checked, "checked"),
            (Hover, Hovered, hovered, "hover"),
            (Focus, Focused, focused, "focus"),
            (Pressed, Pressed, pressed, "pressed"),
            (Disabled, Disabled, disabled, "disabled"),
        }
    };
}

pub mod ast;
mod color;
pub mod grid;
pub mod properties;
pub mod snapshot;
pub mod ty;
pub mod validate;

pub use ast::{
    Animation, ClassRule, Color, ColorExpr, ColorValue, Gradient, Property, State, Stylesheet,
    Theme, ThemeEntry, Value,
};
pub use color::mix;
pub use grid::{
    Area, AreaTemplate, GridLine, LinePlacement, ParseGridError, RepeatCount, TrackSize,
};
pub use properties::{
    Kind, LayoutField, SPECS, Side, Spec, StatePolicy, StyleField, Target, TextField, ThemeType,
};
pub use snapshot::{Snapshot, SnapshotError};
pub use ty::Type;
pub use validate::ValidationError;
