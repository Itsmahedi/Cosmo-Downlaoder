use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Token-bucket based asynchronous rate limiter.
#[derive(Debug, Clone)]
pub struct TokenBucketRateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

#[derive(Debug)]
struct RateLimiterInner {
    rate_bps: f64,
    capacity: f64,
    tokens: f64,
    last_update: Instant,
}

impl TokenBucketRateLimiter {
    /// Create a new rate limiter with the specified bytes per second (0 = unlimited).
    pub fn new(rate_bps: u64) -> Self {
        let rate = rate_bps as f64;
        let capacity = if rate > 0.0 {
            (rate * 2.0).max(64.0 * 1024.0)
        } else {
            0.0
        };

        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                rate_bps: rate,
                capacity,
                tokens: capacity,
                last_update: Instant::now(),
            })),
        }
    }

    /// Dynamically update the rate limit in bytes per second (0 = unlimited).
    pub async fn set_rate(&self, rate_bps: u64) {
        let mut inner = self.inner.lock().await;
        let rate = rate_bps as f64;
        inner.rate_bps = rate;
        inner.capacity = if rate > 0.0 {
            (rate * 2.0).max(64.0 * 1024.0)
        } else {
            0.0
        };
        inner.tokens = inner.capacity;
        inner.last_update = Instant::now();
    }

    /// Wait until the requested number of byte tokens are available.
    pub async fn acquire(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }

        loop {
            let sleep_duration = {
                let mut inner = self.inner.lock().await;
                if inner.rate_bps <= 0.0 {
                    return;
                }

                let now = Instant::now();
                let elapsed = now.duration_since(inner.last_update).as_secs_f64();
                inner.last_update = now;

                // Refill tokens
                inner.tokens = (inner.tokens + elapsed * inner.rate_bps).min(inner.capacity);

                let needed = bytes as f64;
                if inner.tokens >= needed {
                    inner.tokens -= needed;
                    return;
                }

                // Calculate required sleep duration to accumulate missing tokens
                let deficit = needed - inner.tokens;
                let wait_secs = deficit / inner.rate_bps;
                Duration::from_secs_f64(wait_secs)
            };

            sleep(sleep_duration).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unlimited_limiter() {
        let limiter = TokenBucketRateLimiter::new(0);
        let start = Instant::now();
        limiter.acquire(1024 * 1024).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_throttling() {
        // Limit to 50 KB/s
        let limiter = TokenBucketRateLimiter::new(50 * 1024);
        // Drain initial burst
        limiter.acquire(100 * 1024).await;

        let start = Instant::now();
        // Request 25 KB, should take approx 0.5s
        limiter.acquire(25 * 1024).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(300), "Elapsed: {:?}", elapsed);
    }
}
