mod decoder;
mod player;
mod view;

pub use decoder::{DecodeError, Decoder, Frame, FrameData, Frames, Gif};
pub use player::Player;
pub use view::view;
