use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{IsTerminal, stderr};
use std::time::Duration;

const TICK_INTERVAL: Duration = Duration::from_millis(80);

pub struct CheckProgress {
    bars: Vec<ProgressBar>,
    labels: Vec<String>,
    statuses: Vec<Option<bool>>,
    interactive: bool,
}

impl CheckProgress {
    pub fn new(labels: impl IntoIterator<Item = String>) -> Self {
        let labels: Vec<_> = labels.into_iter().collect();
        let interactive = stderr().is_terminal();
        let progress = if interactive {
            MultiProgress::with_draw_target(ProgressDrawTarget::stderr())
        } else {
            MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
        };
        let style = ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("check progress template should be valid")
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
        let bars = labels
            .iter()
            .map(|label| {
                let bar = progress.add(ProgressBar::new_spinner());
                bar.set_style(style.clone());
                bar.set_message(format!("Checking {label}"));
                bar.enable_steady_tick(TICK_INTERVAL);
                bar
            })
            .collect();
        let statuses = vec![None; labels.len()];

        Self {
            bars,
            labels,
            statuses,
            interactive,
        }
    }

    pub fn succeed(&mut self, index: usize) {
        self.finish(index, true);
    }

    pub fn fail(&mut self, index: usize) {
        self.finish(index, false);
    }

    pub fn print_plain(&self) {
        if self.interactive {
            return;
        }
        for (label, status) in self.labels.iter().zip(&self.statuses) {
            let status = if *status == Some(true) {
                "PASS"
            } else {
                "FAIL"
            };
            eprintln!("{status} Checking {label}");
        }
    }

    fn finish(&mut self, index: usize, succeeded: bool) {
        self.statuses[index] = Some(succeeded);
        let (symbol, color) = if succeeded {
            ("✓", "green")
        } else {
            ("✗", "red")
        };
        self.bars[index].set_style(
            ProgressStyle::with_template(&format!("{{prefix:.{color}}} {{msg}}"))
                .expect("check completion template should be valid"),
        );
        self.bars[index].set_prefix(symbol);
        self.bars[index].finish();
    }
}
