use std::env;

use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

mod encoding;
mod group;
mod identifier;
mod merge;
mod processor;
mod progress;
mod recording;

use crate::{group::recordings, processor::process};

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

    let groups = recordings(&input)?;
    process(input, output, groups)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // use super::*;
}
