use indicatif::{
    FormattedDuration, MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle,
};

use crate::recording::Group;

use std::sync::{Arc};
use std::time::Duration;

pub trait Reporter<T> {
    fn add(&self, len: u64) -> T;
    fn wait(&self) -> std::io::Result<()>;
}

#[derive(Clone)]
pub struct ConsoleProgressBarReporter {
    pb: Arc<MultiProgress>,
}

impl ConsoleProgressBarReporter {
    pub fn new() -> Self {
        let mpb = MultiProgress::new();
        mpb.set_draw_target(ProgressDrawTarget::stdout_with_hz(60));
        ConsoleProgressBarReporter { pb: Arc::new(mpb) }
    }
}

impl Reporter<ConsoleProgressBar> for ConsoleProgressBarReporter {
    fn add(&self, len: u64) -> ConsoleProgressBar {
        let pb = self.pb.add(ProgressBar::new(len));
        pb.set_style(ProgressStyle::default_bar().template("{prefix:15} {bar:40.cyan/blue} {msg}"));
        pb.set_position(0);
        pb.enable_steady_tick(100);
        pb.set_message("00:00:00 / Unknown");

        ConsoleProgressBar { updated: false, pb }
    }

    fn wait(&self) -> std::io::Result<()> {
        self.pb.join()
    }
}

pub trait Progress {
    fn init(&self, group: &Group);
    fn update(&mut self, len: Duration, progress: Duration);
    fn finish(&self);
}

#[derive(Clone)]
pub struct ConsoleProgressBar {
    updated: bool,
    pb: ProgressBar,
}

impl Progress for ConsoleProgressBar {
    fn init(&self, group: &Group) {
        self.pb.set_prefix(&group.name());
    }

    fn update(&mut self, len: Duration, progress: Duration) {
        if !self.updated {
            self.pb.set_style(
                ProgressStyle::default_bar().template("{prefix:15} {bar:40.cyan/blue} {msg}"),
            );
            self.pb.disable_steady_tick();
            self.updated = true
        }

        let percentage = ((progress.as_secs_f64() / len.as_secs_f64()) * 100f64).round() as u64;
        self.pb.set_position(percentage);
        if percentage < 100 {
            self.pb.set_message(&format!(
                "{} / {}",
                FormattedDuration(progress),
                FormattedDuration(len)
            ));
        } else {
            self.pb.set_message(&format!("{}", FormattedDuration(len)));
        }
    }

    fn finish(&self) {
        self.pb.finish()
    }
}
