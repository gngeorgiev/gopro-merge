use std::{
    path::Path,
    process::{Child, ChildStdout, Command as Process, Stdio},
};

use log::*;

use super::{Error, Result};

const FFMPEG_PROCESS_NAME: &str = "ffmpeg";

pub trait Command
where
    Self: Sized,
{
    fn spawn(self) -> Result<Self>;
    fn stdout(&mut self) -> Result<&mut ChildStdout>;
    fn wait_success(self) -> Result<()>;
}

pub struct FFmpegCommand {
    process: Process,
    child: Option<Child>,
    input_file_path: Option<String>,
}

impl FFmpegCommand {
    pub fn new<T: AsRef<Path>, E: AsRef<Path>>(input_file_path: T, output_file_path: E) -> Self {
        let input_file_path = input_file_path.as_ref();
        let output_file_path = output_file_path.as_ref();

        let args = [
            "-f",
            "concat",
            "-safe",
            "0",
            "-y",
            "-i",
            input_file_path.as_os_str().to_str().unwrap(),
            "-c",
            "copy",
            output_file_path.as_os_str().to_str().unwrap(),
            "-loglevel",
            "error",
            "-progress",
            "pipe:1",
        ];

        debug!(
            "Creating {} command with args {:?}",
            FFMPEG_PROCESS_NAME, &args
        );
        let mut process = Process::new(FFMPEG_PROCESS_NAME);
        process
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null()); //TODO: async reading of stdout/stderr for less threads

        FFmpegCommand {
            process,
            child: None,
            input_file_path: Some(input_file_path.as_os_str().to_str().unwrap().into()),
        }
    }
}

impl Command for FFmpegCommand {
    fn spawn(mut self) -> Result<Self> {
        self.child = Some(self.process.spawn()?);
        Ok(self)
    }

    fn stdout(&mut self) -> Result<&mut ChildStdout> {
        stdout(&mut self.child, FFMPEG_PROCESS_NAME)
    }

    fn wait_success(self) -> Result<()> {
        wait_success(self.child, self.input_file_path)
    }
}

fn stdout<'a>(
    child: &'a mut Option<Child>,
    process_name: &'static str,
) -> Result<&'a mut ChildStdout> {
    let stdout = child
        .as_mut()
        .expect("command not spawned, can not get stdout")
        .stdout
        .as_mut()
        .ok_or_else(|| Error::NoStdout(process_name.into()))?;

    Ok(stdout)
}

fn wait_success(child: Option<Child>, mut file: Option<String>) -> Result<()> {
    match child
        .expect("command not spawned, can not wait")
        .wait()?
        .success()
    {
        true => Ok(()),
        false => Err(Error::FailedToConvert(file.take().unwrap())),
    }
}
