use std::sync::Arc;
use std::time::Duration;

use indicatif::{FormattedDuration, MultiProgress, ProgressBar, ProgressStyle};

use crate::group::RecordingGroup;

pub trait Reporter<T> {
    fn add(&self, len: u64, group: &RecordingGroup) -> T;
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

impl Reporter<TerminalProgressBar> for ConsoleProgressBarReporter {
    fn add(&self, len: u64, group: &RecordingGroup) -> TerminalProgressBar {
        let pb = self.multi.add(
            ProgressBar::new(len)
                .with_style(
                    ProgressStyle::default_bar()
                        .template("📹 {prefix:5} ⌛ {bar:70.cyan/blue} {msg}"),
                )
                .with_prefix(format!(
                    "{} ({} chapters)",
                    group.name(),
                    group.chapters.len()
                )),
        );
        TerminalProgressBar { pb, len: None }
    }

    fn wait(&self) -> std::io::Result<()> {
        self.multi.join()
    }
}

pub trait Progress {
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
        self.pb.set_message(format!(
            "🕒 {} / {}",
            FormattedDuration(progress),
            FormattedDuration(len)
        ));
    }

    fn finish(&self) {
        self.pb
            .set_message(format!("✅ {}", FormattedDuration(self.len.unwrap())));
        self.pb.finish()
    }
}
