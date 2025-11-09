//! Cache metrics tracking and reporting
//!
//! Tracks cache performance metrics including hit rates, latencies, and sizes.
//! Integrates with Prometheus for monitoring.

use metrics::{counter, gauge, histogram};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache tier identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    L1,
    L2,
    L3,
}

impl CacheTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheTier::L1 => "l1",
            CacheTier::L2 => "l2",
            CacheTier::L3 => "l3",
        }
    }
}

/// Cache operation type
#[derive(Debug, Clone, Copy)]
pub enum CacheOperation {
    Hit,
    Miss,
    Write,
    Delete,
}

/// Metrics collector for cache operations
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    // L1 metrics
    l1_hits: Arc<AtomicU64>,
    l1_misses: Arc<AtomicU64>,
    l1_writes: Arc<AtomicU64>,

    // L2 metrics
    l2_hits: Arc<AtomicU64>,
    l2_misses: Arc<AtomicU64>,
    l2_writes: Arc<AtomicU64>,

    // Overall metrics
    total_requests: Arc<AtomicU64>,
}

impl CacheMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            l1_hits: Arc::new(AtomicU64::new(0)),
            l1_misses: Arc::new(AtomicU64::new(0)),
            l1_writes: Arc::new(AtomicU64::new(0)),
            l2_hits: Arc::new(AtomicU64::new(0)),
            l2_misses: Arc::new(AtomicU64::new(0)),
            l2_writes: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a cache operation
    pub fn record_operation(&self, tier: CacheTier, operation: CacheOperation) {
        match (tier, operation) {
            (CacheTier::L1, CacheOperation::Hit) => {
                self.l1_hits.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_hits_total", "tier" => "l1").increment(1);
            }
            (CacheTier::L1, CacheOperation::Miss) => {
                self.l1_misses.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_misses_total", "tier" => "l1").increment(1);
            }
            (CacheTier::L1, CacheOperation::Write) => {
                self.l1_writes.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_writes_total", "tier" => "l1").increment(1);
            }
            (CacheTier::L2, CacheOperation::Hit) => {
                self.l2_hits.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_hits_total", "tier" => "l2").increment(1);
            }
            (CacheTier::L2, CacheOperation::Miss) => {
                self.l2_misses.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_misses_total", "tier" => "l2").increment(1);
            }
            (CacheTier::L2, CacheOperation::Write) => {
                self.l2_writes.fetch_add(1, Ordering::Relaxed);
                counter!("llm_edge_cache_writes_total", "tier" => "l2").increment(1);
            }
            (CacheTier::L3, CacheOperation::Hit) => {
                counter!("llm_edge_cache_hits_total", "tier" => "l3").increment(1);
            }
            (CacheTier::L3, CacheOperation::Miss) => {
                counter!("llm_edge_cache_misses_total", "tier" => "l3").increment(1);
            }
            (CacheTier::L3, CacheOperation::Write) => {
                counter!("llm_edge_cache_writes_total", "tier" => "l3").increment(1);
            }
            _ => {}
        }
    }

    /// Record cache lookup latency
    pub fn record_latency(&self, tier: CacheTier, duration: Duration) {
        let latency_ms = duration.as_secs_f64() * 1000.0;
        histogram!(
            "llm_edge_cache_latency_ms",
            "tier" => tier.as_str()
        )
        .record(latency_ms);
    }

    /// Record a request (for overall metrics)
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        counter!("llm_edge_requests_total").increment(1);
    }

    /// Update cache size gauge
    pub fn update_cache_size(&self, tier: CacheTier, size: u64) {
        gauge!(
            "llm_edge_cache_size_entries",
            "tier" => tier.as_str()
        )
        .set(size as f64);
    }

    /// Update cache memory usage
    pub fn update_cache_memory(&self, tier: CacheTier, bytes: u64) {
        gauge!(
            "llm_edge_cache_memory_bytes",
            "tier" => tier.as_str()
        )
        .set(bytes as f64);
    }

    /// Calculate L1 hit rate
    pub fn l1_hit_rate(&self) -> f64 {
        let hits = self.l1_hits.load(Ordering::Relaxed);
        let misses = self.l1_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64) / (total as f64)
        }
    }

    /// Calculate L2 hit rate
    pub fn l2_hit_rate(&self) -> f64 {
        let hits = self.l2_hits.load(Ordering::Relaxed);
        let misses = self.l2_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64) / (total as f64)
        }
    }

    /// Calculate overall cache hit rate (L1 + L2)
    pub fn overall_hit_rate(&self) -> f64 {
        let l1_hits = self.l1_hits.load(Ordering::Relaxed);
        let l2_hits = self.l2_hits.load(Ordering::Relaxed);
        let l1_misses = self.l1_misses.load(Ordering::Relaxed);

        let total_hits = l1_hits + l2_hits;
        let total_requests = l1_hits + l1_misses; // L1 sees all requests

        if total_requests == 0 {
            0.0
        } else {
            (total_hits as f64) / (total_requests as f64)
        }
    }

    /// Get total number of requests
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.l1_misses.load(Ordering::Relaxed),
            l1_writes: self.l1_writes.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.l2_misses.load(Ordering::Relaxed),
            l2_writes: self.l2_writes.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
        }
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of cache metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l1_writes: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub l2_writes: u64,
    pub total_requests: u64,
}

impl MetricsSnapshot {
    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total == 0 {
            0.0
        } else {
            (self.l1_hits as f64) / (total as f64)
        }
    }

    pub fn l2_hit_rate(&self) -> f64 {
        let total = self.l2_hits + self.l2_misses;
        if total == 0 {
            0.0
        } else {
            (self.l2_hits as f64) / (total as f64)
        }
    }

    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits + self.l2_hits;
        let total_requests = self.l1_hits + self.l1_misses;

        if total_requests == 0 {
            0.0
        } else {
            (total_hits as f64) / (total_requests as f64)
        }
    }
}

/// Helper to measure operation latency
pub struct LatencyTimer {
    start: Instant,
    tier: CacheTier,
    metrics: CacheMetrics,
}

impl LatencyTimer {
    pub fn new(tier: CacheTier, metrics: CacheMetrics) -> Self {
        Self {
            start: Instant::now(),
            tier,
            metrics,
        }
    }

    pub fn finish(self) {
        let duration = self.start.elapsed();
        self.metrics.record_latency(self.tier, duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = CacheMetrics::new();

        metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
        metrics.record_operation(CacheTier::L1, CacheOperation::Miss);
        metrics.record_operation(CacheTier::L1, CacheOperation::Hit);

        assert_eq!(metrics.l1_hits.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.l1_misses.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let metrics = CacheMetrics::new();

        // Record 7 hits and 3 misses = 70% hit rate
        for _ in 0..7 {
            metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
        }
        for _ in 0..3 {
            metrics.record_operation(CacheTier::L1, CacheOperation::Miss);
        }

        let hit_rate = metrics.l1_hit_rate();
        assert!(
            (hit_rate - 0.7).abs() < 0.01,
            "Expected 70% hit rate, got {}",
            hit_rate
        );
    }

    #[test]
    fn test_empty_metrics_hit_rate() {
        let metrics = CacheMetrics::new();
        assert_eq!(
            metrics.l1_hit_rate(),
            0.0,
            "Empty metrics should have 0% hit rate"
        );
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = CacheMetrics::new();

        metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
        metrics.record_operation(CacheTier::L2, CacheOperation::Miss);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.l1_hits, 1);
        assert_eq!(snapshot.l2_misses, 1);
    }

    #[test]
    fn test_overall_hit_rate() {
        let metrics = CacheMetrics::new();

        // 10 L1 requests: 6 hits, 4 misses
        for _ in 0..6 {
            metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
        }
        for _ in 0..4 {
            metrics.record_operation(CacheTier::L1, CacheOperation::Miss);
        }

        // Of the 4 L1 misses, 2 hit L2, 2 miss L2
        for _ in 0..2 {
            metrics.record_operation(CacheTier::L2, CacheOperation::Hit);
        }

        // Overall: 8 hits (6 L1 + 2 L2) out of 10 requests = 80%
        let overall = metrics.overall_hit_rate();
        assert!(
            (overall - 0.8).abs() < 0.01,
            "Expected 80% overall hit rate, got {}",
            overall
        );
    }
}
