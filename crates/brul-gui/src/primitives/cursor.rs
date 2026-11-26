#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    Grab,
    Grabbing,
    Crosshair,
    NotAllowed,
    Move,
}
