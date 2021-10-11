use std::io;
use std::sync::Arc;
use std::time::Duration;

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

pub trait Reporter: Clone {
    type Progress;

    fn add(&self, group: &MovieGroup, index: usize, movies_len: usize) -> Self::Progress;
    fn wait(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct ConsoleProgressBarReporter {
    multi: Arc<MultiProgress>,
}

impl ConsoleProgressBarReporter {
    pub fn new() -> Self {
        ConsoleProgressBarReporter {
            multi: Arc::new(MultiProgress::new()),
        }
    }
}

impl Reporter for ConsoleProgressBarReporter {
    type Progress = TerminalProgressBar;

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
        TerminalProgressBar { pb, len: None }
    }

    fn wait(&self) -> Result<()> {
        self.multi.join().map_err(From::from)
    }
}

pub trait Progress: Clone {
    fn update(&mut self, progress: Duration);
    fn set_len(&mut self, len: Duration);
    fn finish(&self);
}

#[derive(Clone, Debug)]
pub struct TerminalProgressBar {
    pb: ProgressBar,
    len: Option<Duration>,
}

impl Progress for TerminalProgressBar {
    fn set_len(&mut self, len: Duration) {
        self.len = Some(len);
    }

    fn update(&mut self, progress: Duration) {
        self.pb
            .set_position(calculate_percentage(self.len(), progress));
        self.pb.set_message(self.message_styled(format!(
            "🕒 {} / {}",
            FormattedDuration(progress),
            FormattedDuration(self.len())
        )));
    }

    fn finish(&self) {
        self.pb.finish_with_message(
            self.message_styled(format!("✅ {}", FormattedDuration(self.len()))),
        );
    }
}

impl TerminalProgressBar {
    fn message_styled(&self, msg: String) -> String {
        style(msg).bold().to_string()
    }

    fn len(&self) -> Duration {
        self.len.expect("progress len not set")
    }
}

fn calculate_percentage(len: Duration, progress: Duration) -> u64 {
    ((progress.as_secs_f64() / len.as_secs_f64()) * 100f64).round() as u64
}

#[derive(Clone)]
pub struct JsonProgressReporter {
    progresses: Arc<Mutex<Vec<JsonProgress>>>,
}

impl JsonProgressReporter {
    pub fn new() -> Self {
        JsonProgressReporter {
            progresses: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl Reporter for JsonProgressReporter {
    type Progress = JsonProgress;

    fn add(&self, group: &MovieGroup, index: usize, movies_len: usize) -> Self::Progress {
        let p = JsonProgress::new(group.name(), group.chapters.len(), index, movies_len);
        self.progresses.lock().push(p.clone());
        p
    }

    fn wait(&self) -> Result<()> {
        let progresses = self.progresses.lock();
        progresses
            .iter()
            .map(|p| p.chan.1.recv().map_err(From::from))
            .collect::<Result<Vec<_>>>()
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct JsonProgress {
    inner: Arc<RwLock<JsonProgressInner>>,
    chan: (Sender<()>, Receiver<()>),
}

impl Progress for JsonProgress {
    fn set_len(&mut self, len: Duration) {
        self.inner.write().len = len.into();
    }

    fn update(&mut self, progress: Duration) {
        self.print(
            progress,
            calculate_percentage(self.inner.read().len.expect("len not set"), progress),
        );
    }

    fn finish(&self) {
        self.chan.0.send(()).unwrap();
    }
}

impl JsonProgress {
    fn new(name: String, chapters: usize, index: usize, all_movies: usize) -> Self {
        JsonProgress {
            inner: Arc::new(RwLock::new(JsonProgressInner {
                name,
                chapters,
                index,
                all_movies,
                len: None,
            })),
            chan: bounded(1),
        }
    }

    fn print(&self, progress: Duration, progress_percentage: u64) {
        let inner = self.inner.read();

        let json_data = json!({
            "name": inner.name,
            "chapters": inner.chapters,
            "index": inner.index,
            "len": FormattedDuration(inner.len.expect("len not set: print")).to_string(),
            "all_movies": inner.all_movies,
            "progress_time": FormattedDuration(progress).to_string(),
            "progress_percentage": progress_percentage,
        });

        //TODO: This should probabaly write to a selected stream in the future and be fallible
        println!("{}", json_data);
    }
}

struct JsonProgressInner {
    name: String,
    chapters: usize,
    index: usize,
    all_movies: usize,
    len: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentage() {
        fn test_case<T: Into<f64>>(
            len: T,
            progress: T,
            expected: u64,
        ) -> (Duration, Duration, u64) {
            (
                Duration::from_secs_f64(len.into()),
                Duration::from_secs_f64(progress.into()),
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
