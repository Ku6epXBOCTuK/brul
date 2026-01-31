use thiserror::Error;

use crate::GuiControlMessage;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    WinitEventLoopError(#[from] winit::error::EventLoopError),

    #[error(transparent)]
    WinitEventLoopClosed(#[from] winit::event_loop::EventLoopClosed<GuiControlMessage>),

    #[error("{0}")]
    WinitOtherError(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
