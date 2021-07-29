#![allow(dead_code)]
use std::default::Default;
use std::fmt;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Stopwatch {
    /// The time the stopwatch was started last, if ever.
    start_time: Option<Instant>,
    /// The time the stopwatch was split last, if ever.
    split_time: Option<Instant>,
    /// The time elapsed while the stopwatch was running (between start() and stop()).
    elapsed: Duration,
}

impl Default for Stopwatch {
    fn default() -> Stopwatch {
        Stopwatch {
            start_time: None,
            split_time: None,
            elapsed: Duration::from_secs(0),
        }
    }
}

impl fmt::Display for Stopwatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}ms", self.elapsed_ms());
    }
}

impl Stopwatch {
    /// Returns a new stopwatch.
    pub fn new() -> Stopwatch {
        let sw: Stopwatch = Default::default();
        sw
    }

    /// Returns a new stopwatch which will immediately be started.
    pub fn start_new() -> Stopwatch {
        let mut sw = Stopwatch::new();
        sw.start();
        sw
    }

    /// Starts the stopwatch.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Stops the stopwatch.
    pub fn stop(&mut self) {
        self.elapsed = self.elapsed();
        self.start_time = None;
        self.split_time = None;
    }

    /// Resets all counters and stops the stopwatch.
    pub fn reset(&mut self) {
        self.elapsed = Duration::from_secs(0);
        self.start_time = None;
        self.split_time = None;
    }

    /// Resets and starts the stopwatch again.
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Returns whether the stopwatch is running.
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }

    /// Returns the elapsed time since the start of the stopwatch.
    pub fn elapsed(&self) -> Duration {
        match self.start_time {
            // stopwatch is running
            Some(t1) => t1.elapsed() + self.elapsed,
            // stopwatch is not running
            None => self.elapsed,
        }
    }

    /// Returns the elapsed time since the start of the stopwatch in milliseconds.
    pub fn elapsed_ms(&self) -> i64 {
        let dur = self.elapsed();
        (dur.as_secs() * 1000 + dur.subsec_millis() as u64) as i64
    }

    /// Returns the elapsed time since last split or start/restart.
    ///
    /// If the stopwatch is in stopped state this will always return a zero Duration.
    pub fn elapsed_split(&mut self) -> Duration {
        match self.start_time {
            // stopwatch is running
            Some(start) => {
                let res = match self.split_time {
                    Some(split) => split.elapsed(),
                    None => start.elapsed(),
                };
                self.split_time = Some(Instant::now());
                res
            }
            // stopwatch is not running
            None => Duration::from_secs(0),
        }
    }

    /// Returns the elapsed time since last split or start/restart in milliseconds.
    ///
    /// If the stopwatch is in stopped state this will always return zero.
    pub fn elapsed_split_ms(&mut self) -> i64 {
        let dur = self.elapsed_split();
        (dur.as_secs() * 1000 + dur.subsec_millis() as u64) as i64
    }
}
