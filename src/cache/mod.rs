//! Multi-Tier Caching System for LLM Edge Agent
//!
//! This module implements a high-performance multi-tier caching system with:
//! - L1: In-memory cache (Moka) - <1ms latency, TinyLFU eviction
//! - L2: Distributed cache (Redis) - 1-2ms latency, persistent across instances
//!
//! # Architecture
//!
//! ```text
//! Request → L1 Lookup (in-memory)
//!            ├─ HIT → Return (0.1ms)
//!            └─ MISS
//!                ↓
//!           L2 Lookup (Redis)
//!            ├─ HIT → Populate L1 + Return (2ms)
//!            └─ MISS
//!                ↓
//!           Provider Execution
//!                ↓
//!           Async Write → L1 + L2 (non-blocking)
//! ```
//!
//! # Performance Targets
//! - L1 Latency: <1ms (typically <100μs)
//! - L2 Latency: 1-2ms
//! - Overall Hit Rate: >50% (MVP), >70% (Beta)
//! - L1 TTL: 5 minutes (default)
//! - L2 TTL: 1 hour (default)

pub mod key;
pub mod l1;
pub mod l2;
pub mod metrics;

use self::key::{generate_cache_key, CacheableRequest};
use self::l1::{CachedResponse, L1Cache};
use self::l2::{create_l2_cache_optional, L2Cache, L2Config};
use self::metrics::{CacheMetrics, MetricsSnapshot};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Result of a cache lookup operation
#[derive(Debug, Clone)]
pub enum CacheLookupResult {
    /// Cache hit from L1 (in-memory)
    L1Hit(Arc<CachedResponse>),
    /// Cache hit from L2 (Redis)
    L2Hit(Arc<CachedResponse>),
    /// Cache miss (need to fetch from provider)
    Miss,
}

impl CacheLookupResult {
    pub fn is_hit(&self) -> bool {
        matches!(self, Self::L1Hit(_) | Self::L2Hit(_))
    }

    pub fn response(&self) -> Option<Arc<CachedResponse>> {
        match self {
            Self::L1Hit(resp) | Self::L2Hit(resp) => Some(Arc::clone(resp)),
            Self::Miss => None,
        }
    }
}

/// Multi-tier cache orchestrator
///
/// This is the main interface for cache operations. It coordinates
/// lookups and writes across L1 and L2 cache tiers.
pub struct CacheManager {
    l1: L1Cache,
    l2: Option<L2Cache>,
    metrics: CacheMetrics,
}

impl CacheManager {
    /// Create a new cache manager with default L1 and no L2
    pub fn new() -> Self {
        let metrics = CacheMetrics::new();
        let l1 = L1Cache::new(metrics.clone());

        Self {
            l1,
            l2: None,
            metrics,
        }
    }

    /// Create a new cache manager with L1 and L2
    pub async fn with_l2(l2_config: L2Config) -> Self {
        let metrics = CacheMetrics::new();
        let l1 = L1Cache::new(metrics.clone());
        let l2 = create_l2_cache_optional(l2_config, metrics.clone()).await;

        Self { l1, l2, metrics }
    }

    /// Lookup a request in the cache
    ///
    /// # Flow
    /// 1. Check L1 (in-memory)
    /// 2. If miss, check L2 (Redis)
    /// 3. If L2 hit, populate L1
    /// 4. Return result
    ///
    /// # Performance
    /// - L1 hit: <1ms
    /// - L2 hit: 1-2ms
    pub async fn lookup(&self, request: &CacheableRequest) -> CacheLookupResult {
        let cache_key = generate_cache_key(request);

        // L1 lookup
        if let Some(response) = self.l1.get(&cache_key).await {
            debug!("Cache HIT: L1");
            return CacheLookupResult::L1Hit(response);
        }

        // L2 lookup (if available)
        if let Some(ref l2) = self.l2 {
            match l2.get(&cache_key).await {
                Ok(Some(response)) => {
                    debug!("Cache HIT: L2");

                    // Populate L1 asynchronously (fire-and-forget)
                    let l1_clone = self.l1.clone();
                    let key_clone = cache_key.clone();
                    let response_clone = response.clone();
                    tokio::spawn(async move {
                        l1_clone.set(key_clone, (*response_clone).clone()).await;
                    });

                    return CacheLookupResult::L2Hit(Arc::new(response));
                }
                Ok(None) => {
                    debug!("Cache MISS: L2");
                }
                Err(e) => {
                    warn!("L2 cache error during lookup: {}", e);
                }
            }
        }

        debug!("Cache MISS: all tiers");
        CacheLookupResult::Miss
    }

    /// Store a response in the cache
    ///
    /// Writes to both L1 and L2 asynchronously (non-blocking).
    /// This should be called after receiving a response from the LLM provider.
    ///
    /// # Performance
    /// Non-blocking, returns immediately. Cache writes happen in background.
    pub async fn store(&self, request: &CacheableRequest, response: CachedResponse) {
        let cache_key = generate_cache_key(request);

        // Write to L1 (fast, in-memory)
        self.l1.set(cache_key.clone(), response.clone()).await;

        // Write to L2 asynchronously (fire-and-forget)
        if let Some(ref l2) = self.l2 {
            let l2_clone = l2.clone();
            let key_clone = cache_key.clone();
            let response_clone = response.clone();

            tokio::spawn(async move {
                if let Err(e) = l2_clone.set(key_clone, response_clone).await {
                    warn!("L2 cache write error: {}", e);
                }
            });
        }
    }

    /// Store with custom L2 TTL
    pub async fn store_with_ttl(
        &self,
        request: &CacheableRequest,
        response: CachedResponse,
        l2_ttl_seconds: u64,
    ) {
        let cache_key = generate_cache_key(request);

        // Write to L1
        self.l1.set(cache_key.clone(), response.clone()).await;

        // Write to L2 with custom TTL
        if let Some(ref l2) = self.l2 {
            let l2_clone = l2.clone();
            let key_clone = cache_key.clone();
            let response_clone = response.clone();

            tokio::spawn(async move {
                if let Err(e) = l2_clone
                    .set_with_ttl(key_clone, response_clone, l2_ttl_seconds)
                    .await
                {
                    warn!("L2 cache write with TTL error: {}", e);
                }
            });
        }
    }

    /// Invalidate a cache entry across all tiers
    pub async fn invalidate(&self, request: &CacheableRequest) {
        let cache_key = generate_cache_key(request);

        // Remove from L1
        self.l1.remove(&cache_key).await;

        // Remove from L2
        if let Some(ref l2) = self.l2 {
            if let Err(e) = l2.remove(&cache_key).await {
                warn!("L2 cache delete error: {}", e);
            }
        }
    }

    /// Clear all cache entries (use with caution!)
    pub async fn clear_all(&self) {
        info!("Clearing all cache tiers");

        self.l1.clear().await;

        if let Some(ref l2) = self.l2 {
            if let Err(e) = l2.clear().await {
                warn!("L2 cache clear error: {}", e);
            }
        }
    }

    /// Check health of cache tiers
    pub async fn health_check(&self) -> CacheHealthStatus {
        let l1_healthy = true; // L1 is always healthy (in-memory)
        let l2_healthy = if let Some(ref l2) = self.l2 {
            l2.health_check().await
        } else {
            false // L2 not configured
        };

        CacheHealthStatus {
            l1_healthy,
            l2_healthy,
            l2_configured: self.l2.is_some(),
        }
    }

    /// Get metrics snapshot
    pub fn metrics_snapshot(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get L1 cache entry count
    pub fn l1_entry_count(&self) -> u64 {
        self.l1.entry_count()
    }

    /// Get L2 cache approximate size
    pub async fn l2_approximate_size(&self) -> Option<usize> {
        if let Some(ref l2) = self.l2 {
            l2.approximate_size().await.ok()
        } else {
            None
        }
    }

    /// Check if L2 is configured and available
    pub fn has_l2(&self) -> bool {
        self.l2.is_some()
    }

    /// Get shared metrics instance
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        Self {
            l1: L1Cache::with_config(self.l1.config().clone(), self.metrics.clone()),
            l2: None, // L2 uses ConnectionManager which is Clone-able, but we'd need to expose it
            metrics: self.metrics.clone(),
        }
    }
}

/// Cache health status
#[derive(Debug, Clone)]
pub struct CacheHealthStatus {
    pub l1_healthy: bool,
    pub l2_healthy: bool,
    pub l2_configured: bool,
}

impl CacheHealthStatus {
    pub fn is_fully_healthy(&self) -> bool {
        if self.l2_configured {
            self.l1_healthy && self.l2_healthy
        } else {
            self.l1_healthy
        }
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::l1::TokenUsage;
    use chrono::Utc;

    fn create_test_request() -> CacheableRequest {
        CacheableRequest::new("gpt-4", "Hello, world!")
            .with_temperature(0.7)
            .with_max_tokens(100)
    }

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
    async fn test_cache_manager_l1_only() {
        let cache = CacheManager::new();
        let request = create_test_request();

        // Initial lookup should miss
        let result = cache.lookup(&request).await;
        assert!(matches!(result, CacheLookupResult::Miss));

        // Store value
        cache.store(&request, create_test_response("Test response")).await;

        // Should hit L1 now
        let result = cache.lookup(&request).await;
        assert!(result.is_hit());
        if let CacheLookupResult::L1Hit(response) = result {
            assert_eq!(response.content, "Test response");
        } else {
            panic!("Expected L1 hit");
        }
    }

    #[tokio::test]
    async fn test_cache_manager_invalidate() {
        let cache = CacheManager::new();
        let request = create_test_request();

        // Store and verify
        cache.store(&request, create_test_response("Test")).await;
        assert!(cache.lookup(&request).await.is_hit());

        // Invalidate
        cache.invalidate(&request).await;

        // Should miss now
        assert!(matches!(cache.lookup(&request).await, CacheLookupResult::Miss));
    }

    #[tokio::test]
    async fn test_cache_manager_health_check() {
        let cache = CacheManager::new();
        let health = cache.health_check().await;

        assert!(health.l1_healthy);
        assert!(!health.l2_configured);
        assert!(health.is_fully_healthy());
    }

    #[tokio::test]
    async fn test_cache_manager_metrics() {
        let cache = CacheManager::new();
        let request = create_test_request();

        // Miss
        cache.lookup(&request).await;

        // Store
        cache.store(&request, create_test_response("Test")).await;

        // Hit
        cache.lookup(&request).await;

        let snapshot = cache.metrics_snapshot();
        assert!(snapshot.l1_hits >= 1);
        assert!(snapshot.l1_misses >= 1);
    }

    #[tokio::test]
    async fn test_cache_lookup_result() {
        let response = Arc::new(create_test_response("Test"));

        let l1_hit = CacheLookupResult::L1Hit(Arc::clone(&response));
        assert!(l1_hit.is_hit());
        assert!(l1_hit.response().is_some());

        let miss = CacheLookupResult::Miss;
        assert!(!miss.is_hit());
        assert!(miss.response().is_none());
    }
}
