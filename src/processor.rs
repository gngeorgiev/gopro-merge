use std::io;
use std::path::PathBuf;
use std::thread;

use crate::merge;
use crate::progress::{self, Reporter};
use crate::{group::RecordingGroups, progress::Progress};

use log::*;
use rayon::prelude::*;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Merge(#[from] merge::Error),

    #[error(transparent)]
    Progress(#[from] progress::Error),

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("Processor has no reporter set")]
    NoReporter,
}

pub struct Processor<R> {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    recordings: Option<RecordingGroups>,
    reporter: Option<R>,
}

impl<R> Processor<R>
where
    R: Reporter + Sized + Send + 'static,
    R::Progress: Progress + Send + 'static,
{
    pub fn new(input: PathBuf, output: PathBuf, recordings: RecordingGroups) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            recordings: Some(recordings),
            reporter: None,
        }
    }

    pub fn with_reporter(mut self, reporter: R) -> Self {
        self.reporter = Some(reporter);
        self
    }

    pub fn process(mut self) -> Result<()> {
        let reporter = self.get_reporter()?;

        let mut recordings = self.recordings.take().unwrap();
        recordings.sort();
        let recordings_len = recordings.len();
        let input = self.input.take().unwrap();
        let output = self.output.take().unwrap();

        let recordings = recordings
            .into_iter()
            .enumerate()
            .map(|(index, recording)| {
                debug!("adding recording {} {:?}", index, recording);
                merge::Merger::new(
                    reporter.add(&recording, index, recordings_len),
                    recording,
                    input.clone().into(),
                    output.clone().into(),
                )
            })
            .collect::<Vec<_>>();

        let worker = thread::spawn(move || {
            recordings
                .into_par_iter()
                .map(|merger| merger.merge().map_err(Error::from))
                .collect::<Result<Vec<_>>>()?;

            Ok(())
        });

        let reporter = thread::spawn(move || self.get_reporter()?.wait().map_err(Error::from));

        [worker, reporter]
            .into_iter()
            .map(|handle| handle.join().unwrap().map_err(Error::from))
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn get_reporter(&self) -> Result<R> {
        self.reporter
            .as_ref()
            .ok_or_else(|| Error::NoReporter)
            .map(|r| r.clone())
    }
}
