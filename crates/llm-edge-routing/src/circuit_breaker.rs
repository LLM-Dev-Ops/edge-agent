//! Circuit breaker implementation
//!
//! Prevents cascading failures by opening circuit after N consecutive failures

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Circuit open, fail fast
    HalfOpen, // Testing if service recovered
}

pub struct CircuitBreaker {
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    threshold: u64,
    timeout: Duration,
    last_failure_time: Arc<parking_lot::Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(threshold: u64, timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            threshold,
            timeout,
            last_failure_time: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    pub fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Relaxed);

        if failures < self.threshold {
            return CircuitState::Closed;
        }

        // Check if timeout has elapsed
        let last_failure = self.last_failure_time.lock();
        if let Some(time) = *last_failure {
            if time.elapsed() > self.timeout {
                return CircuitState::HalfOpen;
            }
        }

        CircuitState::Open
    }

    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);

        // Reset failures after 3 consecutive successes
        if self.success_count.load(Ordering::Relaxed) >= 3 {
            self.failure_count.store(0, Ordering::Relaxed);
            self.success_count.store(0, Ordering::Relaxed);
        }
    }

    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.last_failure_time.lock() = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(30));

        assert_eq!(cb.state(), CircuitState::Closed);

        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Record successes to reset
        cb.record_success();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
