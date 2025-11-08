//! Test metrics tracking

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;

/// Test metrics collector
pub struct TestMetrics {
    requests_total: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    l1_hits: AtomicU64,
    l1_misses: AtomicU64,
    l1_writes: AtomicU64,
    l2_hits: AtomicU64,
    l2_misses: AtomicU64,
    l2_writes: AtomicU64,
    provider_requests: AtomicU64,
    cache_writes: AtomicU64,
    total_cost: Arc<RwLock<f64>>,
    provider_stats: Arc<RwLock<HashMap<String, ProviderStats>>>,
}

#[derive(Default)]
struct ProviderStats {
    requests: u64,
    cost: f64,
}

/// Metrics snapshot
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l1_writes: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub l2_writes: u64,
    pub provider_requests: u64,
    pub cache_writes: u64,
    pub total_cost: f64,
}

impl TestMetrics {
    /// Create new test metrics
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            l1_hits: AtomicU64::new(0),
            l1_misses: AtomicU64::new(0),
            l1_writes: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l2_misses: AtomicU64::new(0),
            l2_writes: AtomicU64::new(0),
            provider_requests: AtomicU64::new(0),
            cache_writes: AtomicU64::new(0),
            total_cost: Arc::new(RwLock::new(0.0)),
            provider_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increment request counter
    pub fn increment_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment cache hits
    pub fn increment_cache_hits(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment cache misses
    pub fn increment_cache_misses(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L1 hits
    pub fn increment_l1_hits(&self) {
        self.l1_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L1 misses
    pub fn increment_l1_misses(&self) {
        self.l1_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L1 writes
    pub fn increment_l1_writes(&self) {
        self.l1_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L2 hits
    pub fn increment_l2_hits(&self) {
        self.l2_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L2 misses
    pub fn increment_l2_misses(&self) {
        self.l2_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment L2 writes
    pub fn increment_l2_writes(&self) {
        self.l2_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment provider requests
    pub fn increment_provider_requests(&self, provider: &str) {
        self.provider_requests.fetch_add(1, Ordering::Relaxed);

        let mut stats = self.provider_stats.write();
        stats.entry(provider.to_string())
            .or_insert_with(ProviderStats::default)
            .requests += 1;
    }

    /// Increment cache writes
    pub fn increment_cache_writes(&self) {
        self.cache_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Add cost
    pub fn add_cost(&self, provider: &str, cost: f64) {
        *self.total_cost.write() += cost;

        let mut stats = self.provider_stats.write();
        stats.entry(provider.to_string())
            .or_insert_with(ProviderStats::default)
            .cost += cost;
    }

    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.l1_misses.load(Ordering::Relaxed),
            l1_writes: self.l1_writes.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.l2_misses.load(Ordering::Relaxed),
            l2_writes: self.l2_writes.load(Ordering::Relaxed),
            provider_requests: self.provider_requests.load(Ordering::Relaxed),
            cache_writes: self.cache_writes.load(Ordering::Relaxed),
            total_cost: *self.total_cost.read(),
        }
    }

    /// Get provider request count by name
    pub fn provider_requests_by_name(&self, provider: &str) -> u64 {
        self.provider_stats.read()
            .get(provider)
            .map(|stats| stats.requests)
            .unwrap_or(0)
    }

    /// Get provider cost by name
    pub fn cost_by_provider(&self, provider: &str) -> f64 {
        self.provider_stats.read()
            .get(provider)
            .map(|stats| stats.cost)
            .unwrap_or(0.0)
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.requests_total.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.l1_hits.store(0, Ordering::Relaxed);
        self.l1_misses.store(0, Ordering::Relaxed);
        self.l1_writes.store(0, Ordering::Relaxed);
        self.l2_hits.store(0, Ordering::Relaxed);
        self.l2_misses.store(0, Ordering::Relaxed);
        self.l2_writes.store(0, Ordering::Relaxed);
        self.provider_requests.store(0, Ordering::Relaxed);
        self.cache_writes.store(0, Ordering::Relaxed);
        *self.total_cost.write() = 0.0;
        self.provider_stats.write().clear();
    }
}

impl MetricsSnapshot {
    /// Calculate L1 hit rate
    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total == 0 {
            0.0
        } else {
            self.l1_hits as f64 / total as f64
        }
    }

    /// Calculate L2 hit rate
    pub fn l2_hit_rate(&self) -> f64 {
        let total = self.l2_hits + self.l2_misses;
        if total == 0 {
            0.0
        } else {
            self.l2_hits as f64 / total as f64
        }
    }

    /// Calculate overall cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}
