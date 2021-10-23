use std::env::temp_dir;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use indicatif::HumanDuration;
use log::*;

use crate::merge::command::{Command as _, FFmpegCommand, FFmpegCommandKind};
use crate::merge::ffmpeg::parser::{
    CommandStreamDurationParser as _, FFmpegDurationProgressParser, FFprobeDurationParser,
};
use crate::merge::Result;
use crate::progress::Progress;
use crate::{group::MovieGroup, merge::Merger};

pub struct FFmpegMerger<P> {
    progress: P,
    group: MovieGroup,
    movies_path: PathBuf,
    merged_output_path: PathBuf,
}

impl<P> Merger for FFmpegMerger<P>
where
    P: Progress + Sized + Send + 'static,
{
    type Progress = P;

    fn new(
        progress: Self::Progress,
        group: MovieGroup,
        movies_path: PathBuf,
        merged_output_path: PathBuf,
    ) -> Self {
        FFmpegMerger {
            progress,
            group,
            movies_path,
            merged_output_path,
        }
    }

    fn merge(self) -> Result<()> {
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
            progress,
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

fn write_movies_to_input_file(mut input_file: impl Write, movies_paths: &[PathBuf]) -> Result<()> {
    movies_paths.iter().try_for_each(|path| {
        write!(
            input_file,
            "file '{}'\r\n",
            path.as_os_str().to_str().unwrap()
        )
        .map_err(From::from)
    })
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

    let mut cmd = FFmpegCommand::new(FFmpegCommandKind::FFmpeg(
        input_file_path.into(),
        output_file_path,
    ))
    .spawn()?;

    progress.set_len(duration);
    FFmpegDurationProgressParser::new(cmd.stdout()?, &mut progress).parse()?;
    progress.finish();

    cmd.wait_success()
}

fn calculate_total_duration(paths: &[PathBuf]) -> Result<Duration> {
    paths
        .iter()
        .map(|path| {
            let mut cmd = FFmpegCommand::new(FFmpegCommandKind::FFprobe(path.into())).spawn()?;
            let duration = FFprobeDurationParser::new(cmd.stdout()?).parse()?;
            cmd.wait_success().map(|_| duration)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ops::Add;

    lazy_static::lazy_static! {
        static ref TEST_FILES_PATHS: Vec<PathBuf> =
            vec!["./tests/GH010084.mp4", "./tests/GH010085.mp4"]
                .into_iter()
                .map(From::from)
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
    fn test_ffmpeg_tmp_file() {
        let (_, p) = init_ffmpeg_tmp_file("filename").unwrap();
        assert!(p.exists());
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), ".filename.txt");
    }

    #[test]
    fn test_calculate_total_duration() {
        // let duration = calculate_total_duration(TEST_FILES_PATHS.clone()).unwrap();
        // assert_eq!(*TOTAL_DURATION, duration);
    }
}
