//! Per-stage wall time and peak RSS (DESIGN §4.3; guidelines §12: instrument each stage
//! separately). Measurements are not content: they are reported, never stored in Delta.

use std::time::{Duration, Instant};

/// One stage: its name, wall time, and the process's peak RSS when it ended (Linux `VmHWM`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    pub name: String,
    pub seconds: f64,
    pub peak_rss_bytes: Option<u64>,
}

/// Stages in the order they ran.
#[derive(Debug, Clone)]
pub struct Stages {
    last: Instant,
    pub stages: Vec<Stage>,
}

impl Default for Stages {
    fn default() -> Self {
        Self::new()
    }
}

impl Stages {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            stages: Vec::new(),
        }
    }

    /// End the stage that began at the previous mark (or at `new`).
    pub fn mark(&mut self, name: impl Into<String>) {
        let now = Instant::now();
        self.push(name, now - self.last);
        self.last = now;
    }

    /// Record a stage measured elsewhere (a sum over modules), without moving the mark.
    pub fn push(&mut self, name: impl Into<String>, took: Duration) {
        self.stages.push(Stage {
            name: name.into(),
            seconds: took.as_secs_f64(),
            peak_rss_bytes: peak_rss_bytes(),
        });
    }

    /// Restart the clock without recording (after `push`ed sub-stages).
    pub fn reset(&mut self) {
        self.last = Instant::now();
    }
}

/// The process's peak resident set size so far, from `/proc/self/status` (`VmHWM`); `None`
/// off Linux.
pub fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}
