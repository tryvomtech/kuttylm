use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Downloading,
    /// Checking size and sha256 after a file is complete.
    Verifying,
    Paused,
    Done,
    Failed,
    Cancelled,
}

/// Sent to the UI as the `download-progress` event.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadEvent {
    pub model_id: String,
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    /// Bytes per second over the last few seconds.
    pub speed_bps: u64,
    /// Seconds left. `None` when the speed is not known yet.
    pub eta_secs: Option<u64>,
    /// Friendly message when `status` is `failed`.
    pub error: Option<String>,
}

/// What the download task reports while it works.
#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_secs: Option<u64>,
}

const WINDOW: Duration = Duration::from_secs(5);

/// Average speed over the last `WINDOW`.
pub struct SpeedMeter {
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedMeter {
    pub fn new() -> Self {
        Self { samples: VecDeque::new() }
    }

    pub fn record(&mut self, now: Instant, downloaded_bytes: u64) {
        self.samples.push_back((now, downloaded_bytes));
        while let Some(&(t, _)) = self.samples.front() {
            if now.duration_since(t) > WINDOW && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn bytes_per_sec(&self) -> u64 {
        match (self.samples.front(), self.samples.back()) {
            (Some(&(t0, b0)), Some(&(t1, b1))) if t1 > t0 => {
                let secs = t1.duration_since(t0).as_secs_f64();
                (b1.saturating_sub(b0) as f64 / secs) as u64
            }
            _ => 0,
        }
    }
}

pub fn eta_secs(remaining: u64, speed_bps: u64) -> Option<u64> {
    (speed_bps > 0).then(|| remaining.div_ceil(speed_bps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_eta() {
        let start = Instant::now();
        let mut m = SpeedMeter::new();
        m.record(start, 0);
        assert_eq!(m.bytes_per_sec(), 0);
        m.record(start + Duration::from_secs(2), 2_000_000);
        assert_eq!(m.bytes_per_sec(), 1_000_000);
        assert_eq!(eta_secs(10_000_000, 1_000_000), Some(10));
        assert_eq!(eta_secs(10, 0), None);
    }

    #[test]
    fn old_samples_are_dropped() {
        let start = Instant::now();
        let mut m = SpeedMeter::new();
        m.record(start, 0); // very slow start
        m.record(start + Duration::from_secs(10), 1_000);
        m.record(start + Duration::from_secs(11), 11_000_000);
        m.record(start + Duration::from_secs(12), 21_000_000);
        // Only the last 5 seconds count: (21M - 1K) / 2 s.
        assert_eq!(m.bytes_per_sec(), 10_499_500);
    }
}
