//! Exponential backoff policy for retryable connector failures.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Static backoff policy.
#[derive(Debug, Clone, Copy)]
pub struct BackoffPolicy {
    /// Base delay for attempt 1.
    pub base: Duration,
    /// Maximum delay.
    pub max: Duration,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(200),
            max: Duration::from_secs(10),
        }
    }
}

/// Mutable backoff state.
#[derive(Debug, Clone)]
pub struct BackoffState {
    policy: BackoffPolicy,
    attempt: u32,
}

impl BackoffState {
    /// Start from attempt 0.
    pub fn new(policy: BackoffPolicy) -> Self {
        Self { policy, attempt: 0 }
    }

    /// Reset attempts (e.g. after a success).
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Next delay and increment attempt counter.
    pub fn next_delay(&mut self) -> Duration {
        self.attempt = self.attempt.saturating_add(1);
        let exp = 2u32.saturating_pow(self.attempt.saturating_sub(1).min(16));
        let mut delay = self.policy.base.saturating_mul(exp);
        if delay > self.policy.max {
            delay = self.policy.max;
        }
        delay.saturating_add(jitter(delay))
    }
}

fn jitter(d: Duration) -> Duration {
    // Small jitter (0..10%) without pulling in a RNG dependency.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .subsec_nanos();
    let pct = nanos % 10; // 0..9
    (d / 100) * pct
}
