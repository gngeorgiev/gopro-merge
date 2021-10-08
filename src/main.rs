#![feature(exit_status_error)]

use std::{env, path::Path};

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

#[derive(StructOpt, Debug, Default)]
#[structopt(name = "gopro-join")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(short, long)]
    parallel: Option<usize>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

impl Opt {
    // Only the first calls of get_input and get_output produce expected results, no intended to be called twice
    fn get_input(&mut self, parent: &Path) -> Result<PathBuf> {
        self.input
            .take()
            .map_or_else(
                || parent.to_path_buf().canonicalize(),
                |path| parent.join(path).canonicalize(),
            )
            .map_err(From::from)
    }

    fn get_output(&mut self, parent: &Path) -> Result<PathBuf> {
        self.output.take().map_or_else(
            || self.get_input(parent),
            |out| out.canonicalize().map_err(From::from),
        )
    }

    fn get_parallel(&self) -> usize {
        self.parallel.unwrap_or_default()
    }
}

fn main() -> Result<()> {
    color_backtrace::install();
    env_logger::init();

    let mut opt = Opt::from_args();

    rayon::ThreadPoolBuilder::new()
        .num_threads(opt.get_parallel())
        .build_global()?;

    let wd = env::current_dir()?;
    let input = opt.get_input(wd.as_path())?;
    let output = opt.get_output(wd.as_path())?;

    let recordings = recordings(&input)?;
    let processor =
        Processor::new(input, output, recordings).with_reporter(ConsoleProgressBarReporter::new());

    processor.process().map_err(From::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_input_output() {
        let mut opt = Opt::default();

        let canonicalized_root = if cfg!(target_os = "macos") {
            // path::canonicalize addds /private to /tmp on macos
            PathBuf::from("/private/")
        } else {
            PathBuf::from("/")
        };

        let root: PathBuf = "/".into();

        opt.input = Some("tmp".into());
        assert_eq!(
            canonicalized_root.join("tmp"),
            opt.get_input(root.as_path()).unwrap(),
        );

        opt.input = None;
        assert_eq!(
            canonicalized_root.join("tmp"),
            opt.get_input(root.join("tmp").as_path()).unwrap(),
        );

        assert_eq!(root, opt.get_input(root.as_path()).unwrap());

        opt.output = Some("/tmp".into());
        assert_eq!(
            canonicalized_root.join("tmp"),
            opt.get_output(root.as_path()).unwrap()
        );

        opt.input = Some("/tmp".into());
        opt.output = None;
        assert_eq!(
            canonicalized_root.join("tmp"),
            opt.get_output(root.as_path()).unwrap()
        );

        opt.input = None;
        opt.output = None;
        assert_eq!(root, opt.get_output(root.as_path()).unwrap());
    }

    #[test]
    fn test_opt_parallel() {
        let mut opt = Opt::default();

        opt.parallel = Some(5);
        assert_eq!(5, opt.get_parallel());

        opt.parallel = Some(0);
        assert_eq!(0, opt.get_parallel());

        opt.parallel = None;
        assert_eq!(0, opt.get_parallel());
    }
}
