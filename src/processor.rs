use std::path::PathBuf;
use std::thread;

use crate::group::{RecordingGroup, RecordingGroups};
use crate::merge::merge;
use crate::progress::{ConsoleProgressBarReporter, Reporter, TerminalProgressBar};

use anyhow::Result;
use rayon::prelude::*;

pub fn process(
    input_path: PathBuf,
    output_path: PathBuf,
    recordings: RecordingGroups,
) -> Result<()> {
    let reporter = ConsoleProgressBarReporter::new();

    let data = recordings
        .into_iter()
        .map(|group| (reporter.add(100, group.name()), group))
        .collect::<Vec<_>>();

    let worker = thread::spawn(move || {
        data.into_par_iter()
            .map(|(pb, group)| {
                merge(pb, &input_path, &output_path, group)?;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok::<_, anyhow::Error>(())
    });

    let reporter = thread::spawn(move || {
        reporter.wait()?;
        Ok::<_, anyhow::Error>(())
    });

    [worker, reporter]
        .into_iter()
        .map(|handle| match handle.join() {
            Err(err) => Err(anyhow::anyhow!("ffmpeg concatenation worker {:?}", err)),
            _ => Ok(()),
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}
