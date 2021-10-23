mod command;
mod ffmpeg;
pub mod merger;

use std::io;
use std::num::ParseIntError;
use std::process::ExitStatus;

pub use ffmpeg::*;
pub use merger::*;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to convert movie {0}, exit status {1}")]
    FailedToConvert(String, ExitStatus),

    #[error("Parsing ffmpeg output line {0}")]
    ParseInt(#[from] ParseIntError),

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("Cannot get stdout stream for command {0}")]
    NoStdout(String),

    #[error("Command not spawned {0}")]
    CommandNotSpawned(String),
}
