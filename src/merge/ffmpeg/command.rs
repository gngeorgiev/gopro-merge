use derive_more::Display;
use std::{
    fs::OpenOptions,
    path::PathBuf,
    process::{Child, ChildStdout, Command as Process, Stdio},
};

use log::*;

use crate::merge::command::Command;
use crate::merge::{Error, Result};

const FFMPEG_PROCESS_NAME: &str = "ffmpeg";
const FFPROBE_PROCESS_NAME: &str = "ffprobe";

#[derive(Display)]
pub enum FFmpegCommandKind {
    #[display(fmt = "ffmpeg")]
    FFmpeg(PathBuf, PathBuf, PathBuf),
    #[display(fmt = "ffprobe")]
    FFprobe(PathBuf),
}

impl FFmpegCommandKind {
    fn args(&self) -> Vec<&str> {
        match self {
            FFmpegCommandKind::FFmpeg(input, output, _) => {
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

    fn stderr_path(&self) -> Option<&PathBuf> {
        match self {
            FFmpegCommandKind::FFmpeg(_, _, stderr) => Some(stderr),
            FFmpegCommandKind::FFprobe(..) => None,
        }
    }
}

pub struct FFmpegCommand {
    kind: FFmpegCommandKind,
    process: Process,
    child: Option<Child>,
}

impl FFmpegCommand {
    pub fn new(kind: FFmpegCommandKind) -> Result<Self> {
        let args = kind.args();

        debug!(
            "Creating {} command with args {:?}",
            kind.process_name(),
            &args[..]
        );

        let stderr = kind
            .stderr_path()
            .map(|path| OpenOptions::new().create(true).write(true).open(path))
            .transpose()?
            .map_or_else(Stdio::null, Stdio::from);

        let mut process = Process::new(kind.process_name());
        process.args(&args).stdout(Stdio::piped()).stderr(stderr);

        Ok(FFmpegCommand {
            kind,
            process,
            child: None,
        })
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
                match &self.kind {
                    kind @ FFmpegCommandKind::FFmpeg(input, _, _)
                    | kind @ FFmpegCommandKind::FFprobe(input) => {
                        format!(
                            "{} {}",
                            kind,
                            input.as_os_str().to_str().unwrap().to_owned(),
                        )
                    }
                },
                exit_status,
            ))
        }
    }
}
