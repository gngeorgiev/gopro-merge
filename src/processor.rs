use std::path::PathBuf;
use std::thread;
use std::{io, marker::PhantomData};

use crate::merge::{self, Merger};
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
}

pub struct Processor<R, M> {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    movies: Option<MovieGroups>,

    _reporter: PhantomData<R>,
    _merger: PhantomData<M>,
}

impl<R, M> Processor<R, M>
where
    R: Reporter,
    R::Progress: Progress,
    M: Merger<Progress = R::Progress>,
{
    pub fn new(input: PathBuf, output: PathBuf, movies: MovieGroups) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            movies: Some(movies),

            _reporter: Default::default(),
            _merger: Default::default(),
        }
    }

    pub fn process(mut self) -> Result<()> {
        let reporter = R::new();

        let movies = {
            let mut m = self.movies.take().unwrap();
            m.sort();
            m
        };
        let movies_len = movies.len();
        let input = self.input.take().unwrap();
        let output = self.output.take().unwrap();

        let mergers = movies
            .into_iter()
            .enumerate()
            .map(|(index, movie)| {
                debug!("adding movie {} {:?}", index, movie);
                M::new(
                    reporter.add(&movie, index, movies_len),
                    movie,
                    input.clone(),
                    output.clone(),
                )
            })
            .collect::<Vec<_>>();

        let worker = thread::spawn(move || {
            mergers
                .into_par_iter()
                .try_for_each(|merger| merger.merge())
                .map_err(From::from)
        });

        let reporter = thread::spawn(move || reporter.wait().map_err(Error::from));

        [worker, reporter]
            .into_iter()
            .try_for_each(|handle| handle.join().unwrap())
    }
}
