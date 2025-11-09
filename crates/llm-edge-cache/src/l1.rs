//! L1 In-Memory Cache using Moka
//!
//! High-performance in-process cache with TinyLFU eviction policy.
//! Target latency: <1ms for get/set operations.

use crate::metrics::{CacheMetrics, CacheOperation, CacheTier, LatencyTimer};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Configuration for L1 cache
#[derive(Debug, Clone)]
pub struct L1Config {
    /// Maximum number of entries (default: 1000)
    pub max_capacity: u64,
    /// Time to live in seconds (default: 300 = 5 minutes)
    pub ttl_seconds: u64,
    /// Time to idle in seconds (default: 120 = 2 minutes)
    pub tti_seconds: u64,
}

impl Default for L1Config {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl_seconds: 300,
            tti_seconds: 120,
        }
    }
}

/// Cached response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The actual response content
    pub content: String,
    /// Token usage information
    pub tokens: Option<TokenUsage>,
    /// Model that generated the response
    pub model: String,
    /// When this entry was cached (Unix timestamp)
    pub cached_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// L1 cache implementation using Moka
#[derive(Clone)]
pub struct L1Cache {
    cache: Cache<String, Arc<CachedResponse>>,
    config: L1Config,
    metrics: CacheMetrics,
}

impl L1Cache {
    /// Create a new L1 cache with default configuration
    pub fn new(metrics: CacheMetrics) -> Self {
        Self::with_config(L1Config::default(), metrics)
    }

    /// Create a new L1 cache with custom configuration
    pub fn with_config(config: L1Config, metrics: CacheMetrics) -> Self {
        info!(
            "Initializing L1 cache: capacity={}, ttl={}s, tti={}s",
            config.max_capacity, config.ttl_seconds, config.tti_seconds
        );

        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .time_to_idle(Duration::from_secs(config.tti_seconds))
            .build();

        Self {
            cache,
            config,
            metrics,
        }
    }

    /// Get a value from the cache
    ///
    /// # Performance
    /// Target: <1ms (typically <100μs)
    pub async fn get(&self, key: &str) -> Option<Arc<CachedResponse>> {
        let _timer = LatencyTimer::new(CacheTier::L1, self.metrics.clone());

        let result = self.cache.get(key).await;

        if result.is_some() {
            debug!("L1 cache HIT: key={}", &key[..16.min(key.len())]);
            self.metrics
                .record_operation(CacheTier::L1, CacheOperation::Hit);
        } else {
            debug!("L1 cache MISS: key={}", &key[..16.min(key.len())]);
            self.metrics
                .record_operation(CacheTier::L1, CacheOperation::Miss);
        }

        result
    }

    /// Set a value in the cache
    ///
    /// # Performance
    /// Target: <1ms (non-blocking, async write)
    pub async fn set(&self, key: String, value: CachedResponse) {
        let _timer = LatencyTimer::new(CacheTier::L1, self.metrics.clone());

        debug!("L1 cache WRITE: key={}", &key[..16.min(key.len())]);

        self.cache.insert(key, Arc::new(value)).await;
        self.metrics
            .record_operation(CacheTier::L1, CacheOperation::Write);

        // Update size metrics
        let size = self.cache.entry_count();
        self.metrics.update_cache_size(CacheTier::L1, size);
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &str) {
        self.cache.invalidate(key).await;
        self.metrics
            .record_operation(CacheTier::L1, CacheOperation::Delete);
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        info!("Clearing L1 cache");
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        self.metrics.update_cache_size(CacheTier::L1, 0);
    }

    /// Get the current number of entries in the cache
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Get the cache configuration
    pub fn config(&self) -> &L1Config {
        &self.config
    }

    /// Get cache statistics
    pub fn stats(&self) -> L1Stats {
        L1Stats {
            entry_count: self.cache.entry_count(),
            max_capacity: self.config.max_capacity,
            ttl_seconds: self.config.ttl_seconds,
        }
    }
}

/// L1 cache statistics
#[derive(Debug, Clone)]
pub struct L1Stats {
    pub entry_count: u64,
    pub max_capacity: u64,
    pub ttl_seconds: u64,
}

impl L1Stats {
    /// Calculate the cache utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.max_capacity == 0 {
            0.0
        } else {
            (self.entry_count as f64 / self.max_capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_response(content: &str) -> CachedResponse {
        CachedResponse {
            content: content.to_string(),
            tokens: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            model: "gpt-4".to_string(),
            cached_at: Utc::now().timestamp(),
        }
    }

    #[tokio::test]
    async fn test_l1_basic_get_set() {
        let metrics = CacheMetrics::new();
        let cache = L1Cache::new(metrics);

        let key = "test_key".to_string();
        let response = create_test_response("Hello, world!");

        // Should miss initially
        assert!(cache.get(&key).await.is_none());

        // Set value
        cache.set(key.clone(), response.clone()).await;

        // Should hit now
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_l1_eviction_by_capacity() {
        let metrics = CacheMetrics::new();
        let config = L1Config {
            max_capacity: 2,
            ttl_seconds: 300,
            tti_seconds: 120,
        };
        let cache = L1Cache::with_config(config, metrics);

        // Insert 3 items into cache with capacity of 2
        cache
            .set("key1".to_string(), create_test_response("value1"))
            .await;
        cache
            .set("key2".to_string(), create_test_response("value2"))
            .await;
        cache
            .set("key3".to_string(), create_test_response("value3"))
            .await;

        // Allow Moka to process evictions
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should have at most 2 entries
        assert!(cache.entry_count() <= 2);
    }

    #[tokio::test]
    async fn test_l1_remove() {
        let metrics = CacheMetrics::new();
        let cache = L1Cache::new(metrics);

        let key = "test_key".to_string();
        cache.set(key.clone(), create_test_response("test")).await;

        assert!(cache.get(&key).await.is_some());

        cache.remove(&key).await;

        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_l1_clear() {
        let metrics = CacheMetrics::new();
        let cache = L1Cache::new(metrics);

        cache
            .set("key1".to_string(), create_test_response("value1"))
            .await;
        cache
            .set("key2".to_string(), create_test_response("value2"))
            .await;

        // Force Moka to process pending operations
        cache.cache.run_pending_tasks().await;

        assert!(cache.entry_count() > 0);

        cache.clear().await;

        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn test_l1_stats() {
        let metrics = CacheMetrics::new();
        let config = L1Config {
            max_capacity: 100,
            ttl_seconds: 300,
            tti_seconds: 120,
        };
        let cache = L1Cache::with_config(config, metrics);

        cache
            .set("key1".to_string(), create_test_response("value1"))
            .await;

        // Force Moka to process pending operations
        cache.cache.run_pending_tasks().await;

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.max_capacity, 100);
        assert_eq!(stats.utilization(), 1.0);
    }

    #[tokio::test]
    async fn test_l1_metrics_recording() {
        let metrics = CacheMetrics::new();
        let cache = L1Cache::new(metrics.clone());

        let key = "test_key".to_string();

        // Miss
        cache.get(&key).await;
        assert_eq!(metrics.snapshot().l1_misses, 1);

        // Write
        cache.set(key.clone(), create_test_response("test")).await;
        assert_eq!(metrics.snapshot().l1_writes, 1);

        // Hit
        cache.get(&key).await;
        assert_eq!(metrics.snapshot().l1_hits, 1);
    }
}
