use std::{
    path::PathBuf,
    process::{Child, ChildStdout, Command as Process, Stdio},
};

use log::*;

use super::{Error, Result};

const FFMPEG_PROCESS_NAME: &str = "ffmpeg";
const FFPROBE_PROCESS_NAME: &str = "ffprobe";

pub trait Command
where
    Self: Sized,
{
    fn spawn(self) -> Result<Self>;
    fn stdout(&mut self) -> Result<&mut ChildStdout>;
    fn wait_success(self) -> Result<()>;
}

pub enum Kind {
    FFmpeg(PathBuf, PathBuf),
    FFprobe(PathBuf),
}

impl Kind {
    fn args(&self) -> Vec<&str> {
        match self {
            Kind::FFmpeg(input, output) => {
                vec![
                    "-f",
                    "concat",
                    "-safe",
                    "0",
                    "-y",
                    "-i",
                    input.as_os_str().to_str().unwrap(),
                    "-c",
                    "copy",
                    output.as_os_str().to_str().unwrap(),
                    "-loglevel",
                    "error",
                    "-progress",
                    "pipe:1",
                ]
            }
            Kind::FFprobe(input) => {
                vec![
                    "-i",
                    input.as_os_str().to_str().unwrap(),
                    "-show_streams",
                    "-loglevel",
                    "error",
                ]
            }
        }
    }

    fn process_name(&self) -> &'static str {
        match self {
            Kind::FFmpeg(_, _) => FFMPEG_PROCESS_NAME,
            Kind::FFprobe(_) => FFPROBE_PROCESS_NAME,
        }
    }

    fn file(&self) -> &str {
        match self {
            Kind::FFmpeg(input, _) => input.as_os_str().to_str().unwrap(),
            Kind::FFprobe(input) => input.as_os_str().to_str().unwrap(),
        }
    }
}

pub struct FFmpegCommand {
    kind: Kind,
    process: Process,
    child: Option<Child>,
}

impl FFmpegCommand {
    pub fn new(kind: Kind) -> Self {
        let args = kind.args();

        debug!(
            "Creating {} command with args {:?}",
            kind.process_name(),
            &args[..]
        );

        let mut process = Process::new(kind.process_name());
        process
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()); //TODO: write stdout with a cli flag

        FFmpegCommand {
            kind,
            process,
            child: None,
        }
    }
}

impl Command for FFmpegCommand {
    fn spawn(mut self) -> Result<Self> {
        self.child = Some(self.process.spawn()?);
        Ok(self)
    }

    fn stdout(&mut self) -> Result<&mut ChildStdout> {
        let stdout = self
            .child
            .as_mut()
            .ok_or_else(|| Error::CommandNotSpawned(self.kind.process_name().into()))?
            .stdout
            .as_mut()
            .ok_or_else(|| Error::NoStdout(self.kind.process_name().into()))?;

        Ok(stdout)
    }

    fn wait_success(self) -> Result<()> {
        self.child
            .ok_or_else(|| Error::CommandNotSpawned(self.kind.process_name().into()))?
            .wait()?
            .exit_ok()
            .map_err(|err| Error::FailedToConvert(self.kind.file().into(), err))
    }
}
