use std::io;
use std::path::PathBuf;
use std::thread;

use crate::merge;
use crate::progress::{self, Reporter};
use crate::{group::MovieGroups, progress::Progress};

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
    movies: Option<MovieGroups>,
    reporter: Option<R>,
}

impl<R> Processor<R>
where
    R: Reporter + Sized + Send + 'static,
    R::Progress: Progress + Send + 'static,
{
    pub fn new(input: PathBuf, output: PathBuf, movies: MovieGroups) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            movies: Some(movies),
            reporter: None,
        }
    }

    pub fn with_reporter(mut self, reporter: R) -> Self {
        self.reporter = Some(reporter);
        self
    }

    pub fn process(mut self) -> Result<()> {
        let reporter = self.get_reporter()?;

        let mut movies = self.movies.take().unwrap();
        movies.sort();
        let movies_len = movies.len();
        let input = self.input.take().unwrap();
        let output = self.output.take().unwrap();

        let movies = movies
            .into_iter()
            .enumerate()
            .map(|(index, movie)| {
                debug!("adding movie {} {:?}", index, movie);
                merge::Merger::new(
                    reporter.add(&movie, index, movies_len),
                    movie,
                    input.clone().into(),
                    output.clone().into(),
                )
            })
            .collect::<Vec<_>>();

        let worker = thread::spawn(move || {
            movies
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
