use std::process::ChildStdout;

use crate::merge::Result;

pub use crate::merge::ffmpeg::{FFmpegCommand, FFmpegCommandKind};

pub trait Command
where
    Self: Sized,
{
    fn spawn(self) -> Result<Self>;
    fn stdout(&mut self) -> Result<&mut ChildStdout>;
    fn wait_success(self) -> Result<()>;
}
