mod command;
pub mod merge;
mod stream;

use std::{io, num::ParseIntError, process::ExitStatusError};

pub use merge::*;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to convert recording {0}, exit status {1}")]
    FailedToConvert(String, ExitStatusError),

    #[error("Invalid ffmpeg output line {0}")]
    InvalidOutputLine(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Parsing ffmpeg output line {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("Cannot get stdout stream for command {0}")]
    NoStdout(String),

    #[error("Command not spawned {0}")]
    CommandNotSpawned(String),
}
