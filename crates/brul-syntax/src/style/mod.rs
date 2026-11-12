mod parser;
mod rule;
mod shorthand;
mod value;

pub use brul_style::{
    Animation, ClassRule, Color, Gradient, GradientColor, Kind, Property, Side, Snapshot,
    SnapshotError, State, Stylesheet, Theme, ThemeEntry, Value,
};
pub use parser::parse_file;
pub use rule::Rule;
