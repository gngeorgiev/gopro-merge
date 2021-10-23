mod command;
mod ffmpeg_merger;
pub mod merger;
mod stream;

use std::{io, num::ParseIntError, process::ExitStatusError};

pub use merger::*;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to convert movie {0}, exit status {1}")]
    FailedToConvert(String, ExitStatusError),

    #[error("Invalid ffmpeg output line {0}")]
    InvalidOutputLine(String),

    #[error("Parsing ffmpeg output line {0}")]
    ParseInt(#[from] ParseIntError),

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("Cannot get stdout stream for command {0}")]
    NoStdout(String),

    #[error("Command not spawned {0}")]
    CommandNotSpawned(String),
}
