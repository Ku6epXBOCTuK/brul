use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    WinitEventLoopError(#[from] winit::error::EventLoopError),
}

pub type Result<T> = std::result::Result<T, Error>;
