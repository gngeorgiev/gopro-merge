use std::process::ChildStdout;

pub use crate::merge::ffmpeg::{FFmpegCommand, FFmpegCommandKind};
use crate::merge::Result;

pub trait Command
where
    Self: Sized,
{
    fn spawn(self) -> Result<Self>;

    fn stdout(&mut self) -> Result<&mut ChildStdout>;

    fn wait_success(self) -> Result<()>;
}
