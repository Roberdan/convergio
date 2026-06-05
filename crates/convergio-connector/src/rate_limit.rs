//! Simple async rate limiter for connector calls.

use std::time::Duration;
use tokio::time::{sleep, Instant};

/// A minimal "one request every N" limiter.
#[derive(Debug)]
pub struct RateLimiter {
    min_interval: Duration,
    next_allowed: Instant,
}

impl RateLimiter {
    /// Build a limiter with `max_per_sec` permits.
    pub fn per_second(max_per_sec: f64) -> Option<Self> {
        if !(max_per_sec.is_finite()) || max_per_sec <= 0.0 {
            return None;
        }
        let secs = 1.0 / max_per_sec;
        let min_interval = Duration::from_secs_f64(secs.max(0.0));
        Some(Self {
            min_interval,
            next_allowed: Instant::now(),
        })
    }

    /// Wait until a permit is available.
    pub async fn acquire(&mut self) {
        let now = Instant::now();
        if now < self.next_allowed {
            sleep(self.next_allowed - now).await;
        }
        self.next_allowed = Instant::now() + self.min_interval;
    }
}
