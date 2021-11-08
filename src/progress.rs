use std::io;
use std::time::Duration;
use std::{io::Write, sync::Arc};

use console::style;
use crossbeam_channel::{bounded, Receiver, Sender};
use indicatif::{FormattedDuration, MultiProgress, ProgressBar, ProgressStyle};
use parking_lot::{Mutex, RwLock};
use serde_json::json;
use thiserror::Error;

use crate::group::MovieGroup;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Recv(#[from] crossbeam_channel::RecvError),

    #[error(transparent)]
    Io(#[from] io::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub trait Reporter: Clone + Sized + Send + 'static {
    type Progress;

    fn new() -> Self;

    fn add(&self, group: &MovieGroup, index: usize, movies_len: usize) -> Self::Progress;

    fn wait(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct ConsoleProgressBarReporter {
    multi: Arc<MultiProgress>,
}

impl Reporter for ConsoleProgressBarReporter {
    type Progress = TerminalProgressBar;

    fn new() -> Self {
        ConsoleProgressBarReporter {
            multi: Arc::new(MultiProgress::new()),
        }
    }

    fn add(&self, group: &MovieGroup, index: usize, movies_len: usize) -> Self::Progress {
        let pb = self.multi.add(
            ProgressBar::new(100)
                .with_style(
                    ProgressStyle::default_bar().template("📹 {prefix}  {bar:70.cyan/blue}  {msg}"),
                )
                .with_prefix(format!(
                    "{} {}",
                    style(format!("{:<9}", format!("[{}/{}]", index + 1, movies_len))).bold(),
                    style(format!(
                        "{} ({} chapters)",
                        group.name(),
                        group.chapters.len()
                    ))
                    .bold()
                    .dim()
                )),
        );
        TerminalProgressBar {
            pb,
            len: Duration::default(),
        }
    }

    fn wait(&self) -> Result<()> {
        self.multi.join().map_err(From::from)
    }
}

pub trait Progress: Clone + Send + 'static {
    fn update(&mut self, progress: Duration);
    fn set_len(&mut self, len: Duration);
    fn finish(&self, err: Option<String>);
}

#[derive(Clone, Debug)]
pub struct TerminalProgressBar {
    pb: ProgressBar,
    len: Duration,
}

impl Progress for TerminalProgressBar {
    fn set_len(&mut self, len: Duration) {
        self.len = len;
    }

    fn update(&mut self, progress: Duration) {
        self.pb
            .set_position(calculate_percentage(self.len, progress));
        self.pb.set_message(self.message_styled(format!(
            "🕒 {} / {}",
            FormattedDuration(progress),
            FormattedDuration(self.len)
        )));
    }

    fn finish(&self, err: Option<String>) {
        let message = match err {
            Some(err) => self.message_styled(format!("❌ {}", err)),
            None => self.message_styled(format!("✅ {}", FormattedDuration(self.len))),
        };

        self.pb.finish_with_message(message);
    }
}

impl TerminalProgressBar {
    fn message_styled(&self, msg: String) -> String {
        style(msg).bold().to_string()
    }
}

fn calculate_percentage(len: Duration, progress: Duration) -> u64 {
    ((progress.as_secs_f64() / len.as_secs_f64()) * 100f64).round() as u64
}

#[derive(Clone)]
pub struct JsonProgressReporter {
    progresses: Arc<Mutex<Vec<JsonProgress>>>,
}

impl Reporter for JsonProgressReporter {
    type Progress = JsonProgress;

    fn new() -> Self {
        JsonProgressReporter {
            progresses: Arc::new(Mutex::new(vec![])),
        }
    }

    fn add(&self, group: &MovieGroup, index: usize, movies_len: usize) -> Self::Progress {
        let p = JsonProgress::new(
            group.name(),
            group.chapters.len(),
            index,
            movies_len,
            io::stdout(),
            io::stderr(),
        );
        self.progresses.lock().push(p.clone());
        p
    }

    fn wait(&self) -> Result<()> {
        let progresses = self.progresses.lock();
        progresses
            .iter()
            .try_for_each(|p| p.chan.1.recv().map_err(From::from))
    }
}

type JsonProgressStream = Arc<Mutex<dyn Write + Sync + Send>>;

#[derive(Clone)]
pub struct JsonProgress {
    len: Arc<RwLock<Duration>>,

    name: String,
    chapters: usize,
    index: usize,
    movies_len: usize,

    chan: (Sender<()>, Receiver<()>),

    out_stream: JsonProgressStream,
    err_out_stream: JsonProgressStream,
}

impl Progress for JsonProgress {
    fn set_len(&mut self, len: Duration) {
        *self.len.write() = len;
    }

    fn update(&mut self, progress: Duration) {
        let len = *self.len.read();
        self.print(progress, calculate_percentage(len, progress));
    }

    fn finish(&self, err: Option<String>) {
        if let Some(err) = err {
            self.print_err(err);
        }

        self.chan.0.send(()).unwrap();
    }
}

impl JsonProgress {
    fn new<T: Write + Sync + Send + 'static, E: Write + Sync + Send + 'static>(
        name: String,
        chapters: usize,
        index: usize,
        movies_len: usize,
        out_stream: T,
        err_out_stream: E,
    ) -> Self {
        JsonProgress {
            len: Arc::new(RwLock::new(Duration::default())),
            name,
            chapters,
            index,
            movies_len,
            chan: bounded(1),
            out_stream: Arc::new(Mutex::new(out_stream)),
            err_out_stream: Arc::new(Mutex::new(err_out_stream)),
        }
    }

    fn print_err(&self, err: String) {
        let json_data = json!({
            "name": self.name,
            "chapters": self.chapters,
            "index": self.index,
            "len": FormattedDuration(*self.len.read()).to_string(),
            "movies_len": self.movies_len,
            "err": err,
        });

        // This stream is usually going to be stderr, unless in tests
        // so it's generally fine to panic if we can't print to stdout anyways
        self.err_out_stream
            .lock()
            .write_all(format!("{}\n", json_data).as_bytes())
            .expect("writing json progress to err stream");
    }

    fn print(&self, progress: Duration, progress_percentage: u64) {
        let json_data = json!({
            "name": self.name,
            "chapters": self.chapters,
            "index": self.index,
            "len": FormattedDuration(*self.len.read()).to_string(),
            "movies_len": self.movies_len,
            "progress_time": FormattedDuration(progress).to_string(),
            "progress_percentage": progress_percentage,
        });

        // This stream is usually going to be stdout, unless in tests
        // so it's generally fine to panic if we can't print to stdout anyways
        self.out_stream
            .lock()
            .write_all(format!("{}\n", json_data).as_bytes())
            .expect("writing json progress to out stream");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentage() {
        fn test_case(len: u64, progress: u64, expected: u64) -> (Duration, Duration, u64) {
            (
                Duration::from_secs(len),
                Duration::from_secs(progress),
                expected,
            )
        }

        let tests = vec![
            test_case(9, 3, 33),
            test_case(10, 3, 30),
            test_case(10, 5, 50),
            test_case(100, 5, 5),
            test_case(33, 10, 30),
        ];

        tests.into_iter().for_each(|(len, progress, expected)| {
            let result = calculate_percentage(len, progress);
            assert_eq!(result, expected);
        });
    }
}
