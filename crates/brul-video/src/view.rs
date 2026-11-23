use std::time::Duration;

use brul_codegen::runtime::{Build, Built, ComponentHandle, ViewError};
use brul_gui::{Texture, Widget};
use brul_syntax::ast::Position;

use crate::decoder::{DecodeError, Decoder, Frame, Frames, Gif};
use crate::player::Player;

const POSITION: Position = Position { line: 0, column: 0 };

fn decoder(source: &str) -> Result<(Box<dyn Decoder>, Frame), DecodeError> {
    if source.to_ascii_lowercase().ends_with(".gif") {
        let gif = Gif::open(source)?;
        let first = gif.first();
        return Ok((Box::new(gif), first));
    }
    let image = image::open(source)?.to_rgba8();
    let (width, height) = image.dimensions();
    let frame = Frame::rgba(width, height, image.into_raw(), Duration::from_millis(100));
    let first = frame.clone();
    let frames = Frames::once(vec![frame])?;
    Ok((Box::new(frames), first))
}

pub fn view(
    _theme: Option<&str>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let source = component.borrow_mut().seed::<String>("source", POSITION)?;
    let playing = component.borrow_mut().seed::<bool>("playing", POSITION)?;
    let (decoder, first) =
        decoder(&source).map_err(|error| ViewError::Message(error.to_string()))?;
    let sink = build.event_sink();
    let id = build.state_id();
    let player = build.state(id, component, || Player::new(id, decoder, first, sink));
    player.set_playing(playing);
    let width = player.width();
    let height = player.height();
    let texture = Texture::external(width, height, player.clone())
        .map_err(|error| ViewError::Message(error.to_string()))?;
    Ok(Built {
        widget: Widget::image(texture),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}
