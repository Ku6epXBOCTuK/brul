use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrulError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, BrulError>;
