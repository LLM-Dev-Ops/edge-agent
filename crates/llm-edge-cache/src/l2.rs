//! L2 Distributed Cache using Redis
//!
//! Distributed cache layer with persistence and multi-instance sharing.
//! Target latency: 1-2ms for get/set operations.

use crate::l1::CachedResponse;
use crate::metrics::{CacheMetrics, CacheOperation, CacheTier, LatencyTimer};
use redis::{AsyncCommands, RedisError};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// L2 cache errors
#[derive(Debug, Error)]
pub enum L2Error {
    #[error("Redis connection error: {0}")]
    Connection(#[from] RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache operation timeout")]
    Timeout,

    #[error("Cache unavailable")]
    Unavailable,
}

/// Configuration for L2 cache
#[derive(Debug, Clone)]
pub struct L2Config {
    /// Redis connection string (e.g., "redis://127.0.0.1:6379")
    pub redis_url: String,
    /// Default TTL in seconds (default: 3600 = 1 hour)
    pub ttl_seconds: u64,
    /// Connection timeout in milliseconds (default: 1000)
    pub connection_timeout_ms: u64,
    /// Operation timeout in milliseconds (default: 100)
    pub operation_timeout_ms: u64,
    /// Key prefix for namespacing (default: "llm_cache:")
    pub key_prefix: String,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            ttl_seconds: 3600,
            connection_timeout_ms: 1000,
            operation_timeout_ms: 100,
            key_prefix: "llm_cache:".to_string(),
        }
    }
}

/// L2 cache implementation using Redis
#[derive(Clone)]
pub struct L2Cache {
    client: redis::Client,
    config: L2Config,
    metrics: CacheMetrics,
}

impl L2Cache {
    /// Create a new L2 cache with default configuration
    pub async fn new(metrics: CacheMetrics) -> Result<Self, L2Error> {
        Self::with_config(L2Config::default(), metrics).await
    }

    /// Create a new L2 cache with custom configuration
    pub async fn with_config(config: L2Config, metrics: CacheMetrics) -> Result<Self, L2Error> {
        info!(
            "Initializing L2 cache: url={}, ttl={}s",
            config.redis_url, config.ttl_seconds
        );

        let client = redis::Client::open(config.redis_url.as_str())?;

        // Test connection
        let mut conn = client.get_multiplexed_async_connection().await?;
        let _: () = redis::cmd("PING").query_async(&mut conn).await?;

        info!("L2 cache connected to Redis successfully");

        Ok(Self {
            client,
            config,
            metrics,
        })
    }

    /// Get a value from the cache
    ///
    /// # Performance
    /// Target: 1-2ms (network round-trip)
    pub async fn get(&self, key: &str) -> Result<Option<CachedResponse>, L2Error> {
        let _timer = LatencyTimer::new(CacheTier::L2, self.metrics.clone());

        let prefixed_key = self.prefixed_key(key);

        // Use timeout to prevent slow Redis from blocking
        let result = tokio::time::timeout(
            Duration::from_millis(self.config.operation_timeout_ms),
            self.get_internal(&prefixed_key),
        )
        .await;

        match result {
            Ok(Ok(Some(value))) => {
                debug!("L2 cache HIT: key={}", &key[..16.min(key.len())]);
                self.metrics
                    .record_operation(CacheTier::L2, CacheOperation::Hit);
                Ok(Some(value))
            }
            Ok(Ok(None)) => {
                debug!("L2 cache MISS: key={}", &key[..16.min(key.len())]);
                self.metrics
                    .record_operation(CacheTier::L2, CacheOperation::Miss);
                Ok(None)
            }
            Ok(Err(e)) => {
                warn!("L2 cache GET error: {}", e);
                self.metrics
                    .record_operation(CacheTier::L2, CacheOperation::Miss);
                Err(e)
            }
            Err(_) => {
                warn!("L2 cache GET timeout");
                self.metrics
                    .record_operation(CacheTier::L2, CacheOperation::Miss);
                Err(L2Error::Timeout)
            }
        }
    }

    /// Internal get implementation
    async fn get_internal(&self, key: &str) -> Result<Option<CachedResponse>, L2Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let data: Option<String> = conn.get(key).await?;

        match data {
            Some(json) => {
                let response: CachedResponse = serde_json::from_str(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Set a value in the cache
    ///
    /// # Performance
    /// Target: 1-2ms (async, non-blocking)
    /// This is designed to be called asynchronously without blocking the main request
    pub async fn set(&self, key: String, value: CachedResponse) -> Result<(), L2Error> {
        self.set_with_ttl(key, value, self.config.ttl_seconds).await
    }

    /// Set a value in the cache with custom TTL
    pub async fn set_with_ttl(
        &self,
        key: String,
        value: CachedResponse,
        ttl_seconds: u64,
    ) -> Result<(), L2Error> {
        let _timer = LatencyTimer::new(CacheTier::L2, self.metrics.clone());

        let prefixed_key = self.prefixed_key(&key);

        // Use timeout to prevent slow Redis from blocking
        let result = tokio::time::timeout(
            Duration::from_millis(self.config.operation_timeout_ms),
            self.set_internal(prefixed_key, value, ttl_seconds),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                debug!("L2 cache WRITE: key={}", &key[..16.min(key.len())]);
                self.metrics
                    .record_operation(CacheTier::L2, CacheOperation::Write);
                Ok(())
            }
            Ok(Err(e)) => {
                warn!("L2 cache SET error: {}", e);
                Err(e)
            }
            Err(_) => {
                warn!("L2 cache SET timeout");
                Err(L2Error::Timeout)
            }
        }
    }

    /// Internal set implementation
    async fn set_internal(
        &self,
        key: String,
        value: CachedResponse,
        ttl_seconds: u64,
    ) -> Result<(), L2Error> {
        let json = serde_json::to_string(&value)?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // Use SETEX to set value with expiration atomically
        let _: () = conn.set_ex(&key, json, ttl_seconds).await?;

        Ok(())
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &str) -> Result<(), L2Error> {
        let prefixed_key = self.prefixed_key(key);
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let _: () = conn.del(&prefixed_key).await?;
        self.metrics
            .record_operation(CacheTier::L2, CacheOperation::Delete);

        Ok(())
    }

    /// Clear all cache entries (use with caution!)
    pub async fn clear(&self) -> Result<(), L2Error> {
        info!("Clearing L2 cache with prefix: {}", self.config.key_prefix);

        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let pattern = format!("{}*", self.config.key_prefix);

        // Get all keys matching the pattern
        let keys: Vec<String> = conn.keys(&pattern).await?;

        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
            info!("Cleared {} keys from L2 cache", keys.len());
        }

        Ok(())
    }

    /// Check if Redis connection is healthy
    pub async fn health_check(&self) -> bool {
        match self.client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                let result: Result<String, RedisError> =
                    redis::cmd("PING").query_async(&mut conn).await;
                result.is_ok()
            }
            Err(_) => false,
        }
    }

    /// Get the current size of the cache (approximate)
    pub async fn approximate_size(&self) -> Result<usize, L2Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let pattern = format!("{}*", self.config.key_prefix);
        let keys: Vec<String> = conn.keys(&pattern).await?;

        Ok(keys.len())
    }

    /// Add key prefix for namespacing
    fn prefixed_key(&self, key: &str) -> String {
        format!("{}{}", self.config.key_prefix, key)
    }

    /// Get cache configuration
    pub fn config(&self) -> &L2Config {
        &self.config
    }
}

/// Helper function to create L2 cache with graceful fallback
///
/// If Redis is unavailable, returns None and logs a warning.
/// This allows the application to continue with L1-only caching.
pub async fn create_l2_cache_optional(config: L2Config, metrics: CacheMetrics) -> Option<L2Cache> {
    match L2Cache::with_config(config, metrics).await {
        Ok(cache) => Some(cache),
        Err(e) => {
            error!("Failed to initialize L2 cache: {}", e);
            warn!("Continuing with L1-only caching");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l1::TokenUsage;
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

    // Note: These tests require a running Redis instance
    // Run with: docker run -d -p 6379:6379 redis:7-alpine

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_l2_basic_get_set() {
        let metrics = CacheMetrics::new();
        let cache = L2Cache::new(metrics).await.expect("Redis not available");

        let key = "test_key".to_string();
        let response = create_test_response("Hello, Redis!");

        // Should miss initially
        let result = cache.get(&key).await.unwrap();
        assert!(result.is_none());

        // Set value
        cache.set(key.clone(), response.clone()).await.unwrap();

        // Should hit now
        let cached = cache.get(&key).await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Hello, Redis!");

        // Cleanup
        cache.remove(&key).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_l2_ttl_expiration() {
        let metrics = CacheMetrics::new();
        let cache = L2Cache::new(metrics).await.expect("Redis not available");

        let key = "test_ttl_key".to_string();
        let response = create_test_response("Will expire soon");

        // Set with 1 second TTL
        cache.set_with_ttl(key.clone(), response, 1).await.unwrap();

        // Should exist immediately
        assert!(cache.get(&key).await.unwrap().is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_l2_health_check() {
        let metrics = CacheMetrics::new();
        let cache = L2Cache::new(metrics).await.expect("Redis not available");

        assert!(cache.health_check().await);
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_l2_metrics_recording() {
        let metrics = CacheMetrics::new();
        let cache = L2Cache::new(metrics.clone())
            .await
            .expect("Redis not available");

        let key = "test_metrics_key".to_string();

        // Miss
        let _ = cache.get(&key).await;
        assert_eq!(metrics.snapshot().l2_misses, 1);

        // Write
        cache
            .set(key.clone(), create_test_response("test"))
            .await
            .unwrap();
        assert_eq!(metrics.snapshot().l2_writes, 1);

        // Hit
        let _ = cache.get(&key).await;
        assert_eq!(metrics.snapshot().l2_hits, 1);

        // Cleanup
        cache.remove(&key).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_l2_key_prefix() {
        let metrics = CacheMetrics::new();
        let config = L2Config {
            key_prefix: "test_prefix:".to_string(),
            ..Default::default()
        };
        let cache = L2Cache::with_config(config, metrics)
            .await
            .expect("Redis not available");

        let key = "my_key".to_string();
        cache
            .set(key.clone(), create_test_response("test"))
            .await
            .unwrap();

        // The actual Redis key should be prefixed
        let prefixed = cache.prefixed_key(&key);
        assert!(prefixed.starts_with("test_prefix:"));

        // Cleanup
        cache.remove(&key).await.unwrap();
    }
}
