use owo_colors::OwoColorize;
use std::io::{IsTerminal, stderr};

pub struct Logger {
    is_term: bool,
    pub verbose: bool,
}

impl Logger {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            is_term: stderr().is_terminal(),
        }
    }

    pub fn write(&self, msg: impl AsRef<str>) {
        if self.is_term {
            eprintln!(" {} {}", "•".dimmed(), msg.as_ref().dimmed());
        } else {
            eprintln!(" • {}", msg.as_ref());
        }
    }
}

macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.verbose {
            $logger.write(format!($($arg)*));
        }
    };
}

macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.write(format!($($arg)*));
    };
}
