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
pub use parser::{Override, Style, parse_file, parse_file_in, parse_root};
pub use rule::Rule;
