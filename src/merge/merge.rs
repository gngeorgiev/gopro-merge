use std::time::Duration;
use std::{env::temp_dir, fs};
use std::{io::Write, path::Path};
use std::{ops::Add, path::PathBuf};

use indicatif::HumanDuration;
use log::*;

use crate::merge::command::{Command as _, FFmpegCommand, FFprobeOpts, Kind};
use crate::merge::stream::{
    CommandStreamDurationParser as _, FfmpegDurationProgressParser, FfprobeDurationParser,
};
use crate::merge::Result;
use crate::progress::Progress;
use crate::{group::MovieGroup, merge::command::FFmpegOpts};

pub struct Merger<P> {
    progress: P,
    group: MovieGroup,
    movies_path: PathBuf,
    merged_output_path: PathBuf,
}

impl<P> Merger<P>
where
    P: Progress,
{
    pub fn new(
        progress: P,
        group: MovieGroup,
        movies_path: PathBuf,
        merged_output_path: PathBuf,
    ) -> Self {
        Merger {
            progress,
            group,
            movies_path,
            merged_output_path,
        }
    }

    pub fn merge(&self) -> Result<()> {
        let Self {
            progress,
            group,
            movies_path,
            merged_output_path,
        } = self;

        let (ffmpeg_input_file, ffmpeg_input_file_path) =
            init_ffmpeg_tmp_file(group.fingerprint.file.to_string().as_str())?;

        let movies_full_paths = group
            .chapters
            .iter()
            .map(|chapter| movies_path.join(&group.chapter_file_name(chapter)))
            .collect::<Vec<_>>();

        debug!(
            "Writing movies to ffmpeg input file {}",
            &ffmpeg_input_file_path.as_os_str().to_str().unwrap(),
        );
        write_movies_to_input_file(ffmpeg_input_file, &movies_full_paths)?;

        debug!("Calculating total duration for group {}", group.name());
        let duration = calculate_total_duration(&movies_full_paths)?;
        debug!(
            "Total duration for group {} is {:?} ({})",
            group.name(),
            duration,
            HumanDuration(duration)
        );

        convert(
            progress.clone(),
            &ffmpeg_input_file_path,
            &merged_output_path,
            duration,
            &group,
        )?;

        fs::remove_file(ffmpeg_input_file_path)?;

        Ok(())
    }
}

fn init_ffmpeg_tmp_file(filename: &str) -> Result<(impl Write, PathBuf)> {
    let tmp_file_path = temp_dir().join(&format!(".{}.txt", filename));
    let tmp_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&tmp_file_path)?;

    Ok((tmp_file, tmp_file_path))
}

fn write_movies_to_input_file(
    mut input_file: impl Write,
    movies_paths: &Vec<PathBuf>,
) -> Result<()> {
    movies_paths
        .iter()
        .map(|path| {
            write!(
                input_file,
                "file '{}'\r\n",
                path.as_os_str().to_str().unwrap()
            )
            .map_err(From::from)
        })
        .collect()
}

fn convert(
    mut progress: impl Progress,
    input_file_path: &Path,
    output_path: &Path,
    duration: Duration,
    group: &MovieGroup,
) -> Result<()> {
    // https://trac.ffmpeg.org/wiki/Concatenate
    let output_file_path = output_path.join(&group.name());

    let mut cmd = FFmpegCommand::new(Kind::FFmpeg(FFmpegOpts {
        input: input_file_path.into(),
        output: output_file_path,
    }))
    .spawn()?;

    progress.set_len(duration);
    FfmpegDurationProgressParser::new(cmd.stdout()?, &mut progress).parse()?;
    progress.finish();

    cmd.wait_success()
}

fn calculate_total_duration(paths: &Vec<PathBuf>) -> Result<Duration> {
    let durations: Vec<Duration> = paths
        .into_iter()
        .map(|path| {
            let mut cmd =
                FFmpegCommand::new(Kind::FFprobe(FFprobeOpts { input: path.into() })).spawn()?;
            let duration = FfprobeDurationParser::new(cmd.stdout()?).parse()?;
            cmd.wait_success().map(|_| duration)
        })
        .collect::<Result<_>>()?;

    let duration_total = durations
        .into_iter()
        .fold(Duration::default(), |acc, add| acc.add(add));

    Ok(duration_total)
}

#[cfg(test)]
mod tests {
    // use vfs::MemoryFS;

    // use super::*;

    // lazy_static::lazy_static! {
    //     static ref TEST_FILES_PATHS: Vec<PathBuf> =
    //         vec!["./tests/GH010084.mp4", "./tests/GH010085.mp4"]
    //             .into_iter()
    //             .map(|p| PathBuf::from(p))
    //             .collect();

    //      static ref SINGLE_FILE_DURATION: Duration = {
    //         Duration::default()
    //             .add(Duration::from_secs(5))
    //             .add(Duration::from_micros(449002))
    //      };

    //      static ref TOTAL_DURATION: Duration = {
    //         Duration::default()
    //             .add(*SINGLE_FILE_DURATION)
    //             .add(*SINGLE_FILE_DURATION)
    //      };
    // }

    // #[test]
    // fn test_ffmpeg_tmp_file() {
    //     let root: VfsPath = MemoryFS::new().into();
    //     let input_path = root.join("dir").unwrap();
    //     input_path.create_dir().unwrap();
    //     let (_, p) = init_ffmpeg_tmp_file(&input_path, "filename").unwrap();
    //     assert_eq!(format!("/dir/{}/filename.txt", TEMP_DIR_NAME), p.as_str());
    // }

    // #[test]
    // fn test_get_duration_for_input() {
    //     //     fn get_input(input: &str) -> String {
    //     //         format!(
    //     //             r#"
    //     //               firmware        : HD8.01.01.60.00
    //     // Duration: {}.43, start: 0.000000, bitrate: 78267 kb/s
    //     //   Stream #0:0(eng): Video: h264 (High) (avc1 / 0x31637661), yuvj420p(pc, bt709), 1920x1440 [SAR 1:1 DAR 4:3], 77999 kb/s, 59.94 fps, 59.94 tbr, 60k tbn, 119.88 tbc (default)
    //     //   "#,
    //     //             input
    //     //         )
    //     //     }

    //     //     let input = get_input("00:06:49.00");
    //     //     let duration = parse_timestamp_match(&input).unwrap();
    //     //     assert_eq!(duration, Duration::from_secs(409));

    //     //     let invalid_input = get_input("fdafdfdafad");
    //     //     assert!(parse_timestamp_match(&invalid_input).is_err());
    // }

    // #[test]
    // fn test_calculate_total_duration() {
    //     // let duration = calculate_total_duration(TEST_FILES_PATHS.clone()).unwrap();
    //     // assert_eq!(*TOTAL_DURATION, duration);
    // }
}
