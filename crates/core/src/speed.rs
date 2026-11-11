use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Tracks transfer rates over a sliding time window for smooth speed and ETA estimation.
#[derive(Debug, Clone)]
pub struct SpeedTracker {
    window_duration: Duration,
    samples: VecDeque<(Instant, u64)>,
}

impl Default for SpeedTracker {
    fn default() -> Self {
        Self::new(Duration::from_millis(2000))
    }
}

impl SpeedTracker {
    /// Create a new SpeedTracker with a custom sliding window duration.
    pub fn new(window_duration: Duration) -> Self {
        Self {
            window_duration,
            samples: VecDeque::with_capacity(32),
        }
    }

    /// Record a progress sample of total downloaded bytes at the current instant.
    pub fn record(&mut self, total_downloaded: u64) {
        let now = Instant::now();
        self.samples.push_back((now, total_downloaded));

        // Purge samples older than the sliding window
        let cutoff = now.checked_sub(self.window_duration).unwrap_or(now);
        while self.samples.len() > 2 {
            if let Some(&(time, _)) = self.samples.front() {
                if time < cutoff {
                    self.samples.pop_front();
                    continue;
                }
            }
            break;
        }
    }

    /// Calculate current transfer speed in bytes per second.
    pub fn speed_bps(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        let (first_time, first_bytes) = *self.samples.front().unwrap();
        let (last_time, last_bytes) = *self.samples.back().unwrap();

        let elapsed = last_time.duration_since(first_time).as_secs_f64();
        if elapsed <= 0.0001 {
            return 0.0;
        }

        let bytes_diff = last_bytes.saturating_sub(first_bytes) as f64;
        (bytes_diff / elapsed).max(0.0)
    }

    /// Estimate remaining time in seconds based on current speed and remaining bytes.
    pub fn eta_seconds(&self, remaining_bytes: u64) -> Option<u64> {
        let speed = self.speed_bps();
        if speed > 100.0 && remaining_bytes > 0 {
            let secs = (remaining_bytes as f64 / speed).round() as u64;
            Some(secs)
        } else if remaining_bytes == 0 {
            Some(0)
        } else {
            None
        }
    }

    /// Format bytes per second into human-readable representation.
    pub fn format_speed(bps: f64) -> String {
        if bps >= 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} GB/s", bps / (1024.0 * 1024.0 * 1024.0))
        } else if bps >= 1024.0 * 1024.0 {
            format!("{:.2} MB/s", bps / (1024.0 * 1024.0))
        } else if bps >= 1024.0 {
            format!("{:.1} KB/s", bps / 1024.0)
        } else {
            format!("{:.0} B/s", bps)
        }
    }

    /// Format ETA seconds into human-readable string (e.g. "01:25", "02:14:05").
    pub fn format_eta(eta_secs: Option<u64>) -> String {
        match eta_secs {
            Some(secs) => {
                let hours = secs / 3600;
                let minutes = (secs % 3600) / 60;
                let seconds = secs % 60;
                if hours > 0 {
                    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    format!("{:02}:{:02}", minutes, seconds)
                }
            }
            None => "--:--".to_string(),
        }
    }

    /// Format byte counts into human-readable string (e.g. "45.2 MB").
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(SpeedTracker::format_bytes(500), "500 B");
        assert_eq!(SpeedTracker::format_bytes(1024), "1.0 KB");
        assert_eq!(SpeedTracker::format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(SpeedTracker::format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(SpeedTracker::format_eta(None), "--:--");
        assert_eq!(SpeedTracker::format_eta(Some(5)), "00:05");
        assert_eq!(SpeedTracker::format_eta(Some(65)), "01:05");
        assert_eq!(SpeedTracker::format_eta(Some(3665)), "01:01:05");
    }
}
