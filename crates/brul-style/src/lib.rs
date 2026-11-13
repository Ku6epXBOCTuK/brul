//! Модель стиля BRUL: имена свойств, значения, правила и каскад.

pub mod ast;
pub mod properties;
pub mod snapshot;
pub mod ty;
pub mod validate;

pub use ast::{
    Animation, ClassRule, Color, Gradient, GradientColor, Property, State, Stylesheet, Theme,
    ThemeEntry, Value,
};
pub use properties::{
    Kind, LayoutField, SPECS, Side, Spec, StatePolicy, StyleField, Target, TextField, ThemeType,
};
pub use snapshot::{Snapshot, SnapshotError};
pub use ty::Type;
pub use validate::ValidationError;
