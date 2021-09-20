use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::mem;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::progress::Progress;
use crate::recording::RecordingGroup;
use crate::{
    concat::Error::{FailedToConvert, FailedToGetInfo},
    identifier::Identifier,
};

use anyhow::Context;
use std::num::ParseIntError;
use std::ops::Add;

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

pub fn concatenate(
    pb: impl Progress,
    input_path: PathBuf,
    output_path: PathBuf,
    mut group: RecordingGroup,
) -> Result<()> {
    pb.init(group.name().as_str());

    let (input_file, input_file_path) = ffmpeg_input_file(&group)?;
    let recordings_files_paths = write_recordings_to_input_file(
        input_file,
        input_path,
        mem::replace(&mut group.chapters, vec![]),
    )?;
    let duration = calculate_total_duration(recordings_files_paths)?;

    convert(pb, input_file_path, output_path, duration, &group)?;

    Ok(())
}

fn ffmpeg_input_file(group: &RecordingGroup) -> Result<(File, PathBuf)> {
    let file_path = env::temp_dir().join(format!("{}.txt", group.fingerprint.file.representation));
    if file_path.exists() {
        fs::remove_file(&file_path)?;
    }

    let tmp_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&file_path)?;

    Ok((tmp_file, file_path))
}

fn write_recordings_to_input_file(
    mut input_file: File,
    input_path: PathBuf,
    recordings: Vec<Identifier>,
) -> Result<Vec<PathBuf>> {
    recordings
        .iter()
        .map(|rec| {
            let rec_path = input_path.join(rec.to_string());
            write!(input_file, "file '{}'\r\n", rec_path.to_str().unwrap())
                .with_context(|| "writing to ffmpeg input file")?;
            Ok(rec_path)
        })
        .collect()
}

fn convert(
    mut pb: impl Progress,
    input_file_path: PathBuf,
    output_path: PathBuf,
    duration: Duration,
    group: &RecordingGroup,
) -> Result<()> {
    // https://trac.ffmpeg.org/wiki/Concatenate
    let mut cmd = Command::new("ffmpeg")
        .args(&[
            "-f",
            "concat",
            "-safe",
            "0",
            "-y",
            "-i",
            input_file_path.as_os_str().to_str().unwrap_or_default(),
            "-c",
            "copy",
            output_path
                .join(group.name())
                .as_os_str()
                .to_str()
                .unwrap_or_default(),
            "-loglevel",
            "error",
            "-progress",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| "starting ffmpeg convert process")?;

    get_duration_from_command_stream(&mut cmd, |name, value| {
        if name != "out_time" {
            return Ok(None);
        }

        let progress_duration = parse_timestamp_match(value)?;
        pb.update(duration, progress_duration);

        Ok(None)
    })?;
    pb.finish();
    if !cmd.wait()?.success() {
        return Err(FailedToConvert(group.name()));
    }

    Ok(())
}

fn parse_timestamp_match(input: &str) -> Result<Duration> {
    macro_rules! parse {
        ($iter:expr) => {
            $iter.next().unwrap_or("0").parse::<u64>()?
        };
    }

    let mut millis_split = input.split('.');
    let mut secs_split = millis_split
        .next()
        .ok_or_else(|| Error::InvalidOutputLine(input.into()))?
        .split(':');
    let hours_duration = Duration::from_secs(parse!(secs_split) * 60 * 60);
    let minutes_duration = Duration::from_secs(parse!(secs_split) * 60);
    let seconds_duration = Duration::from_secs(parse!(secs_split));
    let millis_duration = Duration::from_micros(millis_split.next().unwrap_or("0").parse()?);

    Ok(Duration::default()
        .add(hours_duration)
        .add(minutes_duration)
        .add(seconds_duration)
        .add(millis_duration))
}

fn calculate_total_duration(paths: Vec<PathBuf>) -> Result<Duration> {
    let durations: Vec<Duration> = paths
        .into_iter()
        .map(|path| {
            let mut cmd = Command::new("ffprobe")
                .args(&[
                    "-i",
                    path.to_str().unwrap(),
                    "-show_streams",
                    "-loglevel",
                    "error",
                ])
                .stdout(Stdio::piped())
                .spawn()
                .with_context(|| "spawing ffmpeg info")?;

            let duration = get_duration_from_command_stream(&mut cmd, |name, value| {
                if name != "duration" {
                    return Ok(None);
                }

                let mut split = value.split('.');
                let seconds = Duration::from_secs(
                    split
                        .next()
                        .ok_or_else(|| Error::InvalidOutputLine(value.into()))?
                        .parse()?,
                );
                let micros = Duration::from_micros(
                    split
                        .next()
                        .ok_or_else(|| Error::InvalidOutputLine(value.into()))?
                        .parse()?,
                );

                Ok(Some(Duration::default().add(seconds).add(micros)))
            })?;
            if !cmd.wait()?.success() {
                return Err(FailedToGetInfo(path.to_str().unwrap().to_string()));
            }

            Ok::<_, Error>(duration)
        })
        .collect::<Result<_>>()?;

    let duration_total = durations
        .into_iter()
        .fold(Duration::default(), |acc, add| acc.add(add));

    Ok(duration_total)
}

fn get_duration_from_command_stream(
    cmd: &mut Child,
    mut parse: impl FnMut(&str, &str) -> Result<Option<Duration>>,
) -> Result<Duration> {
    let stdout = cmd
        .stdout
        .as_mut()
        .with_context(|| "getting ffmpeg stdout")?;
    let stdout_reader = BufReader::new(stdout);
    let lines = stdout_reader.lines();

    for line in lines {
        let line = line.with_context(|| "reading ffmpeg output line")?;
        let mut split = line.split("=");

        let output_field_name = match split.next() {
            Some(name) => name,
            None => continue,
        };

        let output_field_value = match split.next() {
            Some(value) => value,
            None => continue,
        };

        if let Some(duration) = parse(output_field_name, output_field_value)? {
            return Ok(duration);
        }
    }

    Ok(Duration::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static::lazy_static! {
        static ref TEST_FILES_PATHS: Vec<PathBuf> =
            vec!["./tests/GH010084.mp4", "./tests/GH010085.mp4"]
                .into_iter()
                .map(|p| PathBuf::from(p))
                .collect();

         static ref SINGLE_FILE_DURATION: Duration = {
            Duration::default()
                .add(Duration::from_secs(5))
                .add(Duration::from_micros(449002))
         };

         static ref TOTAL_DURATION: Duration = {
            Duration::default()
                .add(*SINGLE_FILE_DURATION)
                .add(*SINGLE_FILE_DURATION)
         };
    }

    #[test]
    fn test_get_duration_for_input() {
        fn get_input(input: &str) -> String {
            format!(
                r#"
                  firmware        : HD8.01.01.60.00
    Duration: {}.43, start: 0.000000, bitrate: 78267 kb/s
      Stream #0:0(eng): Video: h264 (High) (avc1 / 0x31637661), yuvj420p(pc, bt709), 1920x1440 [SAR 1:1 DAR 4:3], 77999 kb/s, 59.94 fps, 59.94 tbr, 60k tbn, 119.88 tbc (default)
      "#,
                input
            )
        }

        let input = get_input("00:06:49.00");
        let duration = parse_timestamp_match(&input).unwrap();
        assert_eq!(duration, Duration::from_secs(409));

        let invalid_input = get_input("fdafdfdafad");
        assert!(parse_timestamp_match(&invalid_input).is_err());
    }

    #[test]
    fn test_calculate_total_duration() {
        let duration = calculate_total_duration(TEST_FILES_PATHS.clone()).unwrap();
        assert_eq!(*TOTAL_DURATION, duration);
    }
}
