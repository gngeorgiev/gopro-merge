use std::{
    path::PathBuf,
    process::{Child, ChildStdout, Command as Process, Stdio},
};

use log::*;

use crate::merge::command::Command;
use crate::merge::{Error, Result};

const FFMPEG_PROCESS_NAME: &str = "ffmpeg";
const FFPROBE_PROCESS_NAME: &str = "ffprobe";

pub enum FFmpegCommandKind {
    FFmpeg(PathBuf, PathBuf),
    FFprobe(PathBuf),
}

impl FFmpegCommandKind {
    fn args(&self) -> Vec<&str> {
        match self {
            FFmpegCommandKind::FFmpeg(input, output) => {
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
            FFmpegCommandKind::FFprobe(input) => {
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
            FFmpegCommandKind::FFmpeg(..) => FFMPEG_PROCESS_NAME,
            FFmpegCommandKind::FFprobe(..) => FFPROBE_PROCESS_NAME,
        }
    }
}

pub struct FFmpegCommand {
    kind: FFmpegCommandKind,
    process: Process,
    child: Option<Child>,
}

impl FFmpegCommand {
    pub fn new(kind: FFmpegCommandKind) -> Self {
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
        let exit_status = self
            .child
            .ok_or_else(|| Error::CommandNotSpawned(self.kind.process_name().into()))?
            .wait()?;

        if exit_status.success() {
            Ok(())
        } else {
            Err(Error::FailedToConvert(
                match self.kind {
                    FFmpegCommandKind::FFmpeg(input, _) | FFmpegCommandKind::FFprobe(input) => {
                        input.as_os_str().to_str().unwrap().into()
                    }
                },
                exit_status,
            ))
        }
    }
}
