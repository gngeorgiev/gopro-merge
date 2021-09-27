pub mod merge;
mod stream;

use std::{io, num::ParseIntError};

pub use merge::*;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to get ffmpeg info {0}")]
    FailedToGetInfo(String),

    #[error("Failed to convert recording {0}")]
    FailedToConvert(String),

    #[error("Invalid ffmpeg output line {0}")]
    InvalidOutputLine(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Parsing ffmpeg output line {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error(transparent)]
    IO(#[from] io::Error),
}
