use std::path::PathBuf;
use std::thread;

use crate::group::RecordingGroups;
use crate::merge::merge;
use crate::progress::{ConsoleProgressBarReporter, Reporter};

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
            .map(|(pb, group)| merge(pb, group, &input_path, &output_path).map_err(From::from))
            .collect::<Result<Vec<_>>>()?;

        Ok::<_, anyhow::Error>(())
    });

    let reporter = thread::spawn(move || reporter.wait().map_err(From::from));

    [worker, reporter]
        .into_iter()
        .map(|handle| handle.join().unwrap().map_err(From::from))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}
