use std::path::PathBuf;

use crate::group::MovieGroup;
pub use crate::merge::ffmpeg_merger::FfmpegMerger;
use crate::merge::Result;

use crate::progress::Progress;

pub trait Merger: Sized + Send + 'static {
    type Progress: Progress;

    fn new(
        progress: Self::Progress,
        group: MovieGroup,
        movies_path: PathBuf,
        merged_output_path: PathBuf,
    ) -> Self;
    fn merge(self) -> Result<()>;
}