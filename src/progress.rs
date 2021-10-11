use std::sync::Arc;
use std::time::Duration;

use console::style;
use indicatif::{FormattedDuration, MultiProgress, ProgressBar, ProgressStyle};

use crate::group::RecordingGroup;

pub trait Reporter: Clone {
    type Progress;

    fn add(&self, group: &RecordingGroup, index: usize, recordings_len: usize) -> Self::Progress;
    fn wait(&self) -> std::io::Result<()>;
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

    fn add(&self, group: &RecordingGroup, index: usize, recordings_len: usize) -> Self::Progress {
        let pb = self.multi.add(
            ProgressBar::new(100)
                .with_style(
                    ProgressStyle::default_bar().template("📹 {prefix}  {bar:70.cyan/blue}  {msg}"),
                )
                .with_prefix(format!(
                    "{} {}",
                    style(format!(
                        "{:<9}",
                        format!("[{}/{}]", index + 1, recordings_len)
                    ))
                    .bold(),
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

    fn wait(&self) -> std::io::Result<()> {
        self.multi.join()
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
        let len = self.len.unwrap();
        let percentage = ((progress.as_secs_f64() / len.as_secs_f64()) * 100f64).round() as u64;
        self.pb.set_position(percentage);
        self.pb.set_message(self.message_styled(format!(
            "🕒 {} / {}",
            FormattedDuration(progress),
            FormattedDuration(len)
        )));
    }

    fn finish(&self) {
        self.pb.set_message(
            self.message_styled(format!("✅ {}", FormattedDuration(self.len.unwrap()))),
        );
        self.pb.finish()
    }
}

impl TerminalProgressBar {
    fn message_styled(&self, msg: String) -> String {
        style(msg).bold().to_string()
    }
}
