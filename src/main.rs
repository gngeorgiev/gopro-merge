use std::path::PathBuf;
use std::{env, path::Path, str::FromStr};

use log::*;
use structopt::StructOpt;

use crate::group::group_movies;
use crate::merge::FFmpegMerger;
use crate::processor::Processor;
use crate::progress::{ConsoleProgressBarReporter, JsonProgressReporter, Reporter};
use derive_more::Display;

mod encoding;
mod group;
mod identifier;
mod merge;
mod movie;
mod processor;
mod progress;

type Error = Box<dyn std::error::Error + 'static>;
type Result<T> = std::result::Result<T, Error>;

#[derive(StructOpt, Debug, Default)]
#[structopt(name = "gopro-merge")]
struct Opt {
    /// Directory where to read movies from. [default: current directory]
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    /// Directory where to write merged movies. [default: <input>]
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,

    /// The amount of parallel movies to be merged. [default: amount of cores]
    #[structopt(short, long)]
    parallel: Option<usize>,

    /// The reporter to be used for progress one of "json" | "progressbar".
    #[structopt(default_value = "progressbar", short, long)]
    reporter: OptReporter,
}

#[derive(Debug, PartialEq, Eq, Display)]
enum OptReporter {
    #[display(fmt = "json")]
    Json,
    #[display(fmt = "progressbar")]
    ProgressBar,
}

impl FromStr for OptReporter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "json" => OptReporter::Json,
            "progressbar" => OptReporter::ProgressBar,
            _ => Default::default(),
        })
    }
}

impl Default for OptReporter {
    fn default() -> Self {
        OptReporter::ProgressBar
    }
}

impl Opt {
    // Only the first calls of get_input and get_output produce expected results, not intended to be called twice
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

    let movies = group_movies(&input)?;
    debug!("collected movies: {:?}", movies);

    debug!("starting processor with {} reporter", opt.reporter);
    match opt.reporter {
        OptReporter::ProgressBar => Processor::<
            ConsoleProgressBarReporter,
            FFmpegMerger<<ConsoleProgressBarReporter as Reporter>::Progress>,
        >::new(input, output, movies)
        .process(),
        OptReporter::Json => Processor::<
            JsonProgressReporter,
            FFmpegMerger<<JsonProgressReporter as Reporter>::Progress>,
        >::new(input, output, movies)
        .process(),
    }
    .map_err(From::from)
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
        let mut opt = Opt {
            parallel: Some(5),
            ..Default::default()
        };

        assert_eq!(5, opt.get_parallel());

        opt.parallel = Some(0);
        assert_eq!(0, opt.get_parallel());

        opt.parallel = None;
        assert_eq!(0, opt.get_parallel());
    }

    #[test]
    fn test_opt_reporter() {
        let tests = vec![
            ("json", OptReporter::Json),
            ("progressbar", OptReporter::ProgressBar),
            ("0r3938413", OptReporter::ProgressBar),
        ];

        tests.into_iter().for_each(|(input, expected)| {
            assert_eq!(expected, OptReporter::from_str(input).unwrap());
        })
    }
}
