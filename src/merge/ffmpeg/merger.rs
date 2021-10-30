use std::env::temp_dir;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use indicatif::HumanDuration;
use log::*;

use crate::merge::command::{Command as _, FFmpegCommand, FFmpegCommandKind};
use crate::merge::ffmpeg::parser::{
    CommandStreamDurationParser as _, FFmpegDurationParser, FFprobeDurationParser,
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
            init_ffmpeg_input_file(group.fingerprint.file.to_string().as_str())?;

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

fn init_ffmpeg_input_file(filename: &str) -> Result<(impl Write, PathBuf)> {
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

    debug!(
        "setting progress len for {} to {}",
        &group,
        HumanDuration(duration)
    );
    progress.set_len(duration);
    FFmpegDurationParser::new(cmd.stdout()?, |duration| {
        debug!(
            "updating progress for {} to {}",
            &group,
            HumanDuration(duration)
        );
        progress.update(duration);
    })
    .parse()?;
    debug!("progress finish {}", &group);
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
    use test_env_log::test;

    use super::*;

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    lazy_static::lazy_static! {
        static ref TEST_FILES_PATHS: Vec<PathBuf> =
            vec!["./tests/GH010084.mp4", "./tests/GH020084.mp4"]
                .into_iter()
                .map(From::from)
                .collect();

         static ref SINGLE_FILE_DURATION: Duration = {
            Duration::from_secs(5)+Duration::from_micros(458333)
         };

         static ref TOTAL_DURATION: Duration = {
            (*SINGLE_FILE_DURATION)+(*SINGLE_FILE_DURATION)
         };

         // when encoded the duration is different than just summing the two durations
         static ref TOTAL_DURATION_ENCODED: Duration = {
             Duration::from_secs(10)+Duration::from_micros(918294)
         };
    }

    #[test]
    fn test_ffmpeg_tmp_file() {
        let (_, p) = init_ffmpeg_input_file("filename").unwrap();
        assert!(p.exists());
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), ".filename.txt");
    }

    #[test]
    fn test_calculate_total_duration() {
        let duration = calculate_total_duration(&TEST_FILES_PATHS).unwrap();
        assert_eq!(*TOTAL_DURATION, duration);
    }

    #[test]
    fn test_merger() {
        #[derive(Clone, Default)]
        struct MockProgress {
            finish_called: Arc<AtomicBool>,
        }

        impl Progress for MockProgress {
            fn set_len(&mut self, _: Duration) {}

            fn update(&mut self, _: Duration) {}

            fn finish(&self) {
                self.finish_called.store(true, Ordering::Relaxed);
            }
        }

        let tmp_path = PathBuf::from(".tmp");
        std::fs::create_dir_all(&tmp_path).unwrap();

        let merged_file_name = tmp_path.join("GH000084.mp4");

        let progress = MockProgress::default();
        let movies_path = std::fs::canonicalize(PathBuf::from("./tests")).unwrap();
        let group = crate::group::group_movies(&movies_path).unwrap()[0].clone();
        let merger = FFmpegMerger::new(progress.clone(), group, movies_path, tmp_path);
        merger.merge().unwrap();

        let duration = calculate_total_duration(&[merged_file_name]).unwrap();
        assert_eq!(*TOTAL_DURATION_ENCODED, duration);

        assert!(progress.finish_called.load(Ordering::Relaxed));
    }
}
