#[derive(Clone, Copy)]
pub enum Type {
    ThemeKey,
    Number,
    Px,
    SignedPx,
    Length,
    Color,
    Paint,
    String,
    Bool,
    Keywords(&'static [&'static str]),
    TrackList,
    GridLine,
    NumberRange(f64, f64),
    NumberIn(&'static [f64]),
}
