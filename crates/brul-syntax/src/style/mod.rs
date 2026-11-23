mod builtin;
mod parser;
mod rule;
mod shorthand;
mod value;

pub use brul_style::validate::{PropertyAt, RuleAt, ValidationError, overrides};
pub use brul_style::{
    Animation, AreaTemplate, ClassRule, Color, ColorExpr, ColorValue, Gradient, GridLine, Kind,
    LinePlacement, Property, RepeatCount, Side, Snapshot, SnapshotError, State, Stylesheet, Theme,
    ThemeEntry, TrackSize, Value,
};
pub use builtin::{Binding, BuiltinStyles, Target, parse_builtin};
pub use parser::{
    EffectRule, Override, Style, parse_file, parse_file_in, parse_file_in_with_effects, parse_root,
};
pub use rule::Rule;
