use std::sync::Arc;
use std::time::Duration;

use indicatif::{FormattedDuration, MultiProgress, ProgressBar, ProgressStyle};

pub trait Reporter<T> {
    fn add(&self, len: u64, prefix: String) -> T;
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
    fn add(&self, len: u64, prefix: String) -> TerminalProgressBar {
        let pb = self.multi.add(
            ProgressBar::new(len)
                .with_style(
                    ProgressStyle::default_bar()
                        .template("📹 {prefix:5} ⌛ {bar:70.cyan/blue} {msg}"),
                )
                .with_prefix(prefix),
        );
        TerminalProgressBar { pb }
    }

    fn wait(&self) -> std::io::Result<()> {
        self.multi.join()
    }
}

pub trait Progress {
    fn update(&mut self, len: Duration, progress: Duration);
    fn finish(&self);
}

#[derive(Clone, Debug)]
pub struct TerminalProgressBar {
    pb: ProgressBar,
}

impl Progress for TerminalProgressBar {
    fn update(&mut self, len: Duration, progress: Duration) {
        let percentage = ((progress.as_secs_f64() / len.as_secs_f64()) * 100f64).round() as u64;
        self.pb.set_position(percentage);
        let message = match percentage < 100 {
            true => format!(
                "🕒 {} / {}",
                FormattedDuration(progress),
                FormattedDuration(len)
            ),
            false => format!("✅ {}", FormattedDuration(len)),
        };

        self.pb.set_message(message);
    }

    fn finish(&self) {
        self.pb.finish()
    }
}
