use std::path::PathBuf;
use std::thread;

use crate::progress::{ConsoleProgressBar, ConsoleProgressBarReporter, Reporter};
use crate::recording::{Recording, RecordingGroup};
use crate::{concat::concatenate, recording::RecordingGroups};
use rayon::prelude::*;

use anyhow::Result;

struct GroupWithProgress {
    group: RecordingGroup,
    pb: ConsoleProgressBar,
}

pub fn process(
    input_path: PathBuf,
    output_path: PathBuf,
    recordings: RecordingGroups,
) -> Result<()> {
    let reporter = ConsoleProgressBarReporter::new();

    let data = recordings
        .into_iter()
        .map(|group| GroupWithProgress {
            group,
            pb: reporter.add(100),
        })
        .collect::<Vec<_>>();

    let worker = thread::spawn(move || {
        data.into_par_iter()
            .map(|task| {
                concatenate(
                    task.pb.clone(),
                    input_path.clone(),
                    output_path.clone(),
                    task.group.clone(),
                )?;

                Ok(())
            })
            .collect::<Result<_>>()
    });

    worker.join().expect("spawning worker thread")?;
    reporter.wait()?;

    Ok(())
}
