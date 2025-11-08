//! Integration tests for the multi-tier caching system

use llm_edge_agent::cache::{
    key::{generate_cache_key, CacheableRequest},
    l1::{CachedResponse, L1Cache, TokenUsage},
    l2::{L2Cache, L2Config},
    metrics::{CacheMetrics, CacheOperation, CacheTier},
    CacheLookupResult, CacheManager,
};
use chrono::Utc;
use std::time::Duration;

// Helper function to create test response
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
async fn test_cache_key_consistency() {
    let req1 = CacheableRequest::new("gpt-4", "Hello, world!")
        .with_temperature(0.7)
        .with_max_tokens(100);

    let req2 = CacheableRequest::new("gpt-4", "Hello, world!")
        .with_temperature(0.7)
        .with_max_tokens(100);

    let key1 = generate_cache_key(&req1);
    let key2 = generate_cache_key(&req2);

    assert_eq!(key1, key2, "Identical requests should have identical keys");
}

#[tokio::test]
async fn test_l1_cache_basic_operations() {
    let metrics = CacheMetrics::new();
    let cache = L1Cache::new(metrics.clone());

    let key = "test_key".to_string();
    let response = create_test_response("Test content");

    // Initial miss
    assert!(cache.get(&key).await.is_none());
    assert_eq!(metrics.snapshot().l1_misses, 1);

    // Store
    cache.set(key.clone(), response.clone()).await;
    assert_eq!(metrics.snapshot().l1_writes, 1);

    // Hit
    let cached = cache.get(&key).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().content, "Test content");
    assert_eq!(metrics.snapshot().l1_hits, 1);
}

#[tokio::test]
async fn test_l1_cache_eviction() {
    let metrics = CacheMetrics::new();
    let config = llm_edge_agent::cache::l1::L1Config {
        max_capacity: 3,
        ttl_seconds: 300,
        tti_seconds: 120,
    };
    let cache = L1Cache::with_config(config, metrics);

    // Fill cache beyond capacity
    for i in 0..5 {
        let key = format!("key_{}", i);
        cache.set(key, create_test_response(&format!("value_{}", i))).await;
    }

    // Allow eviction to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should have at most max_capacity entries
    assert!(cache.entry_count() <= 3);
}

#[tokio::test]
async fn test_cache_manager_flow() {
    let cache = CacheManager::new();

    let request = CacheableRequest::new("gpt-4", "What is Rust?")
        .with_temperature(0.7);

    // First lookup - miss
    let result = cache.lookup(&request).await;
    assert!(matches!(result, CacheLookupResult::Miss));

    // Store response
    let response = create_test_response("Rust is a systems programming language");
    cache.store(&request, response).await;

    // Second lookup - L1 hit
    let result = cache.lookup(&request).await;
    assert!(result.is_hit());

    if let CacheLookupResult::L1Hit(resp) = result {
        assert_eq!(resp.content, "Rust is a systems programming language");
    } else {
        panic!("Expected L1 hit");
    }
}

#[tokio::test]
async fn test_cache_manager_invalidation() {
    let cache = CacheManager::new();

    let request = CacheableRequest::new("gpt-4", "Test prompt");
    let response = create_test_response("Test response");

    // Store and verify
    cache.store(&request, response).await;
    assert!(cache.lookup(&request).await.is_hit());

    // Invalidate
    cache.invalidate(&request).await;

    // Should miss after invalidation
    let result = cache.lookup(&request).await;
    assert!(matches!(result, CacheLookupResult::Miss));
}

#[tokio::test]
async fn test_cache_metrics() {
    let cache = CacheManager::new();

    let request = CacheableRequest::new("gpt-4", "Metrics test");

    // Generate some traffic
    cache.lookup(&request).await; // Miss
    cache.store(&request, create_test_response("Test")).await; // Write
    cache.lookup(&request).await; // Hit
    cache.lookup(&request).await; // Hit

    let snapshot = cache.metrics_snapshot();
    assert_eq!(snapshot.l1_misses, 1);
    assert_eq!(snapshot.l1_hits, 2);
    assert_eq!(snapshot.l1_writes, 1);

    // Hit rate should be 2/3 = ~66.7%
    let hit_rate = snapshot.l1_hit_rate();
    assert!((hit_rate - 0.666).abs() < 0.01);
}

#[tokio::test]
async fn test_cache_health_check() {
    let cache = CacheManager::new();
    let health = cache.health_check().await;

    assert!(health.l1_healthy);
    assert!(!health.l2_configured);
    assert!(health.is_fully_healthy());
}

#[tokio::test]
async fn test_different_parameters_different_keys() {
    let req1 = CacheableRequest::new("gpt-4", "Hello")
        .with_temperature(0.7);
    let req2 = CacheableRequest::new("gpt-4", "Hello")
        .with_temperature(0.9);

    let key1 = generate_cache_key(&req1);
    let key2 = generate_cache_key(&req2);

    assert_ne!(key1, key2, "Different temperatures should produce different keys");
}

#[tokio::test]
async fn test_cache_concurrent_access() {
    let cache = std::sync::Arc::new(CacheManager::new());

    let request = CacheableRequest::new("gpt-4", "Concurrent test");
    let response = create_test_response("Concurrent response");

    // Store once
    cache.store(&request, response).await;

    // Concurrent reads
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let cache_clone = cache.clone();
            let req_clone = request.clone();
            tokio::spawn(async move {
                cache_clone.lookup(&req_clone).await
            })
        })
        .collect();

    // All should hit
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_hit());
    }
}

#[tokio::test]
async fn test_cache_performance_target() {
    let cache = CacheManager::new();

    let request = CacheableRequest::new("gpt-4", "Performance test");
    let response = create_test_response("Fast response");

    // Populate cache
    cache.store(&request, response).await;

    // Measure lookup latency
    let start = std::time::Instant::now();
    let _ = cache.lookup(&request).await;
    let duration = start.elapsed();

    // L1 cache should be sub-millisecond
    assert!(
        duration.as_micros() < 1000,
        "L1 cache lookup should be <1ms, got {:?}",
        duration
    );
}

// Redis tests (require running Redis instance)

#[tokio::test]
#[ignore] // Requires Redis
async fn test_l2_cache_basic_operations() {
    let metrics = CacheMetrics::new();
    let cache = L2Cache::new(metrics.clone())
        .await
        .expect("Redis not available");

    let key = "test_l2_key".to_string();
    let response = create_test_response("L2 test content");

    // Initial miss
    assert!(cache.get(&key).await.unwrap().is_none());

    // Store
    cache.set(key.clone(), response.clone()).await.unwrap();

    // Hit
    let cached = cache.get(&key).await.unwrap();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().content, "L2 test content");

    // Cleanup
    cache.remove(&key).await.unwrap();
}

#[tokio::test]
#[ignore] // Requires Redis
async fn test_cache_manager_l1_l2_flow() {
    let l2_config = L2Config::default();
    let cache = CacheManager::with_l2(l2_config).await;

    // Verify L2 is configured and healthy
    let health = cache.health_check().await;
    if !health.l2_healthy {
        panic!("Redis not available for test");
    }

    let request = CacheableRequest::new("gpt-4", "L1+L2 test");
    let response = create_test_response("Multi-tier response");

    // First lookup - miss both tiers
    assert!(matches!(cache.lookup(&request).await, CacheLookupResult::Miss));

    // Store (writes to both L1 and L2)
    cache.store(&request, response).await;

    // Give L2 time to complete async write
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second lookup - should hit L1
    let result = cache.lookup(&request).await;
    assert!(matches!(result, CacheLookupResult::L1Hit(_)));

    // Clear for cleanup
    cache.invalidate(&request).await;
}

#[tokio::test]
#[ignore] // Requires Redis
async fn test_l2_ttl_expiration() {
    let metrics = CacheMetrics::new();
    let cache = L2Cache::new(metrics).await.expect("Redis not available");

    let key = "ttl_test_key".to_string();
    let response = create_test_response("Will expire");

    // Set with 1 second TTL
    cache.set_with_ttl(key.clone(), response, 1).await.unwrap();

    // Verify exists
    assert!(cache.get(&key).await.unwrap().is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be expired
    assert!(cache.get(&key).await.unwrap().is_none());
}

#[tokio::test]
async fn test_metrics_operations() {
    let metrics = CacheMetrics::new();

    // Record some operations
    metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
    metrics.record_operation(CacheTier::L1, CacheOperation::Hit);
    metrics.record_operation(CacheTier::L1, CacheOperation::Miss);
    metrics.record_operation(CacheTier::L2, CacheOperation::Hit);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.l1_hits, 2);
    assert_eq!(snapshot.l1_misses, 1);
    assert_eq!(snapshot.l2_hits, 1);

    let l1_hit_rate = snapshot.l1_hit_rate();
    assert!((l1_hit_rate - 0.666).abs() < 0.01);
}
