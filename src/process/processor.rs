use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::channel;

use crate::process::concat::concatenate;
use crate::progress::{ConsoleProgressBar, ConsoleProgressBarReporter, Reporter};
use crate::recording::{Group, Recording};
use rayon::prelude::*;

use anyhow::{Context, Error, Result};
use std::time::Duration;

pub struct ParallelRecordingsProcessor {
    input_path: PathBuf,
    output_path: PathBuf,
    recordings: Vec<(Group, Vec<Recording>)>,
}

impl ParallelRecordingsProcessor {
    pub fn new(
        input_path: PathBuf,
        output_path: PathBuf,
        recordings: Vec<(Group, Vec<Recording>)>,
    ) -> Self {
        ParallelRecordingsProcessor {
            input_path,
            output_path,
            recordings,
        }
    }

    pub fn process(self) -> Result<()> {
        let reporter = ConsoleProgressBarReporter::new();
        let reporter_worker = reporter.clone();

        let data = self
            .recordings
            .into_iter()
            .map(|rec| (rec.0, rec.1, reporter_worker.add(100)))
            .collect::<Vec<_>>();

        let input_path = self.input_path.clone();
        let output_path = self.output_path.clone();

        let worker = std::thread::spawn(move || {
            data.into_par_iter()
                .map(|(grp, rec, pb)| {
                    concatenate(
                        pb.clone(),
                        input_path.clone(),
                        output_path.clone(),
                        grp.clone(),
                        rec.clone(),
                    )?;

                    Ok(())
                })
                .collect::<Result<_>>()
        });

        reporter.wait()?;
        worker.join().expect("spawning worker thread")?;

        Ok(())
    }
}
