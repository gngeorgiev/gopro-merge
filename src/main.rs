#![feature(exit_status_error)]

use std::env;

use std::path::PathBuf;

use structopt::StructOpt;

mod encoding;
mod group;
mod identifier;
mod merge;
mod processor;
mod progress;
mod recording;

use crate::processor::Processor;
use crate::{group::recordings, progress::ConsoleProgressBarReporter};

#[derive(StructOpt, Debug)]
#[structopt(name = "gopro-join")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(short, long)]
    threads: Option<usize>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

impl Opt {
    fn get_input(&self) -> Result<PathBuf> {
        let wd = env::current_dir()?;
        let path = match &self.input {
            Some(path) => wd.join(path).canonicalize()?,
            None => wd,
        };

        Ok(path)
    }

    fn get_output(&self) -> Result<PathBuf> {
        match &self.output {
            Some(out) => Ok(out.clone()),
            None => self.get_input(),
        }
    }
}

fn main() -> Result<()> {
    color_backtrace::install();
    env_logger::init();

    let opt = Opt::from_args();

    if let Some(threads) = opt.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()?;
    }

    let input = opt.get_input()?;
    let output = opt.get_output()?;

    let recordings = recordings(&input)?;
    let processor =
        Processor::new(input, output, recordings).with_reporter(ConsoleProgressBarReporter::new());

    processor.process().map_err(From::from)
}

#[cfg(test)]
mod tests {
    // use super::*;
}
