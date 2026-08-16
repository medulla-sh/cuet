use owo_colors::OwoColorize;
use std::fmt;
use std::io::{IsTerminal, stderr};

pub struct Logger {
    enabled: bool,
    is_term: bool,
    pub verbose: bool,
}

impl Logger {
    pub fn new(verbose: bool) -> Self {
        Self {
            enabled: true,
            verbose,
            is_term: stderr().is_terminal(),
        }
    }

    pub fn silent() -> Self {
        Self {
            enabled: false,
            verbose: false,
            is_term: false,
        }
    }

    pub fn write(&self, msg: fmt::Arguments<'_>) {
        if !self.enabled {
            return;
        }
        if self.is_term {
            eprintln!(" {} {}", "•".dimmed(), msg.dimmed());
        } else {
            eprintln!(" • {msg}");
        }
    }
}

macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {
        if $logger.verbose {
            $logger.write(format_args!($($arg)*));
        }
    };
}

macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.write(format_args!($($arg)*));
    };
}
