use serde::{Deserialize, Serialize};

/// Represents an individual byte segment of a download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// 0-based index of this segment.
    pub index: usize,
    /// Absolute start byte in the destination file (inclusive).
    pub start: u64,
    /// Absolute end byte in the destination file (inclusive).
    pub end: u64,
    /// Number of bytes successfully downloaded and written so far.
    pub downloaded: u64,
    /// True if this segment has completely finished.
    pub complete: bool,
}

impl Segment {
    /// Total expected size of this segment in bytes.
    #[inline]
    pub fn total_bytes(&self) -> u64 {
        if self.end >= self.start {
            (self.end - self.start) + 1
        } else {
            0
        }
    }

    /// Number of bytes remaining to be downloaded.
    #[inline]
    pub fn remaining_bytes(&self) -> u64 {
        self.total_bytes().saturating_sub(self.downloaded)
    }

    /// Current absolute file offset for resuming download.
    #[inline]
    pub fn current_offset(&self) -> u64 {
        self.start.saturating_add(self.downloaded)
    }

    /// Progress percentage [0.0, 100.0].
    #[inline]
    pub fn progress_percent(&self) -> f64 {
        let total = self.total_bytes();
        if total == 0 {
            100.0
        } else {
            ((self.downloaded as f64) / (total as f64) * 100.0).clamp(0.0, 100.0)
        }
    }
}

/// Calculate byte segments for a file of `total_size` bytes split across `num_segments` parts.
///
/// Rules:
/// - `num_segments` is clamped to [1, 32].
/// - If `total_size == 0`, returns a single segment [0..=0].
/// - If `num_segments > total_size`, `num_segments` is reduced to `total_size`.
/// - Every byte in `0..total_size` is covered with no gaps and no overlaps.
pub fn calculate_segments(total_size: u64, num_segments: usize) -> Vec<Segment> {
    if total_size == 0 {
        return vec![Segment {
            index: 0,
            start: 0,
            end: 0,
            downloaded: 0,
            complete: true,
        }];
    }

    let n = num_segments.clamp(1, 32);
    let count = if (n as u64) > total_size {
        total_size as usize
    } else {
        n
    };

    let chunk_size = total_size / (count as u64);
    let mut segments = Vec::with_capacity(count);

    for i in 0..count {
        let start = (i as u64) * chunk_size;
        let end = if i == count - 1 {
            total_size - 1
        } else {
            ((i as u64) + 1) * chunk_size - 1
        };

        segments.push(Segment {
            index: i,
            start,
            end,
            downloaded: 0,
            complete: false,
        });
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_size() {
        let segs = calculate_segments(0, 8);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start, 0);
        assert_eq!(segs[0].end, 0);
        assert!(segs[0].complete);
    }

    #[test]
    fn test_single_segment() {
        let segs = calculate_segments(1000, 1);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start, 0);
        assert_eq!(segs[0].end, 999);
        assert_eq!(segs[0].total_bytes(), 1000);
    }

    #[test]
    fn test_equal_division() {
        let segs = calculate_segments(1000, 4);
        assert_eq!(segs.len(), 4);
        assert_eq!((segs[0].start, segs[0].end), (0, 249));
        assert_eq!((segs[1].start, segs[1].end), (250, 499));
        assert_eq!((segs[2].start, segs[2].end), (500, 749));
        assert_eq!((segs[3].start, segs[3].end), (750, 999));

        for s in &segs {
            assert_eq!(s.total_bytes(), 250);
        }
    }

    #[test]
    fn test_remainder_handling() {
        let segs = calculate_segments(1003, 4);
        assert_eq!(segs.len(), 4);
        assert_eq!((segs[0].start, segs[0].end), (0, 249));
        assert_eq!((segs[1].start, segs[1].end), (250, 499));
        assert_eq!((segs[2].start, segs[2].end), (500, 749));
        // Remainder goes to the last segment
        assert_eq!((segs[3].start, segs[3].end), (750, 1002));
        assert_eq!(segs[3].total_bytes(), 253);

        let total_covered: u64 = segs.iter().map(|s| s.total_bytes()).sum();
        assert_eq!(total_covered, 1003);
    }

    #[test]
    fn test_small_file_fewer_bytes_than_segments() {
        let segs = calculate_segments(3, 8);
        assert_eq!(segs.len(), 3);
        assert_eq!((segs[0].start, segs[0].end), (0, 0));
        assert_eq!((segs[1].start, segs[1].end), (1, 1));
        assert_eq!((segs[2].start, segs[2].end), (2, 2));
    }

    #[test]
    fn test_continuity_and_no_overlaps() {
        for total in [1, 7, 64, 100, 1024, 10_000_000, 4_294_967_296] {
            for num in [1, 2, 8, 16, 32] {
                let segs = calculate_segments(total, num);
                assert!(!segs.is_empty());
                assert_eq!(segs[0].start, 0);
                assert_eq!(segs.last().unwrap().end, total - 1);

                for i in 1..segs.len() {
                    assert_eq!(
                        segs[i].start,
                        segs[i - 1].end + 1,
                        "Gap or overlap at total={total}, num={num}, seg={i}"
                    );
                }

                let sum: u64 = segs.iter().map(|s| s.total_bytes()).sum();
                assert_eq!(sum, total, "Sum mismatch at total={total}, num={num}");
            }
        }
    }
}
