# llm-edge-cache

[![Crates.io](https://img.shields.io/crates/v/llm-edge-cache.svg)](https://crates.io/crates/llm-edge-cache)
[![Documentation](https://docs.rs/llm-edge-cache/badge.svg)](https://docs.rs/llm-edge-cache)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE)

Multi-tier caching system for LLM Edge Agent with intelligent cache hierarchy and performance monitoring.

## Features

- **Multi-Tier Architecture**: L1 (in-memory) + L2 (distributed) caching for optimal performance
- **High Performance**: Sub-millisecond L1 latency, 1-2ms L2 latency
- **Intelligent Eviction**: TinyLFU algorithm for L1 cache with configurable TTL/TTI
- **Redis-Backed L2**: Distributed caching for multi-instance deployments
- **SHA-256 Key Generation**: Collision-resistant cache keys with parameter normalization
- **Comprehensive Metrics**: Prometheus-compatible metrics for monitoring and observability
- **Graceful Degradation**: Automatic fallback to L1-only mode if L2 is unavailable
- **Type-Safe API**: Strongly typed request/response structures with full async/await support

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                        Cache Lookup Flow                        │
└─────────────────────────────────────────────────────────────────┘

    Request
       │
       ▼
  ┌─────────┐
  │L1 Cache │  In-Memory (Moka)
  │ Lookup  │  Target: <1ms (typically <100μs)
  └────┬────┘
       │
    ┌──┴──┐
    │ HIT │──────────────────────────────► Return (0.1ms)
    └──┬──┘
       │
    ┌──▼──┐
    │MISS │
    └──┬──┘
       │
       ▼
  ┌─────────┐
  │L2 Cache │  Distributed (Redis)
  │ Lookup  │  Target: 1-2ms
  └────┬────┘
       │
    ┌──┴──┐
    │ HIT │──► Populate L1 ──────────────► Return (2ms)
    └──┬──┘
       │
    ┌──▼──┐
    │MISS │
    └──┬──┘
       │
       ▼
  ┌─────────┐
  │Provider │  LLM API Call
  │Execution│  Target: 500-2000ms
  └────┬────┘
       │
       ▼
  ┌─────────┐
  │  Write  │  Async Write to L1 + L2
  │L1 + L2  │  (non-blocking)
  └────┬────┘
       │
       ▼
    Return
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-edge-cache = "0.1.0"
```

## Usage

### Basic Usage (L1 Only)

```rust
use llm_edge_cache::{CacheManager, key::CacheableRequest, l1::CachedResponse};

#[tokio::main]
async fn main() {
    // Create cache manager with default L1 configuration
    let cache = CacheManager::new();

    // Create a cacheable request
    let request = CacheableRequest::new("gpt-4", "What is the meaning of life?")
        .with_temperature(0.7)
        .with_max_tokens(100);

    // Check cache
    let result = cache.lookup(&request).await;

    match result {
        llm_edge_cache::CacheLookupResult::L1Hit(response) => {
            println!("Cache hit! Response: {}", response.content);
        }
        llm_edge_cache::CacheLookupResult::Miss => {
            println!("Cache miss - calling LLM provider");

            // Call your LLM provider here...
            let response = CachedResponse {
                content: "42".to_string(),
                tokens: Some(llm_edge_cache::l1::TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
                model: "gpt-4".to_string(),
                cached_at: chrono::Utc::now().timestamp(),
            };

            // Store in cache
            cache.store(&request, response).await;
        }
        _ => {}
    }
}
```

### Advanced Usage (L1 + L2)

```rust
use llm_edge_cache::{CacheManager, l2::L2Config};

#[tokio::main]
async fn main() {
    // Configure L2 cache (Redis)
    let l2_config = L2Config {
        redis_url: "redis://127.0.0.1:6379".to_string(),
        ttl_seconds: 3600,  // 1 hour
        connection_timeout_ms: 1000,
        operation_timeout_ms: 100,
        key_prefix: "llm_cache:".to_string(),
    };

    // Create cache manager with L1 + L2
    let cache = CacheManager::with_l2(l2_config).await;

    // Use the cache (same API as L1-only)
    // ...
}
```

### Custom L1 Configuration

```rust
use llm_edge_cache::{CacheManager, l1::L1Config};

let l1_config = L1Config {
    max_capacity: 10_000,    // 10k entries
    ttl_seconds: 600,        // 10 minutes
    tti_seconds: 300,        // 5 minutes idle
};

// Note: For custom L1 config, you'll need to construct manually
// or use the builder pattern if available in your version
```

### Health Checks

```rust
// Check cache health
let health = cache.health_check().await;
println!("L1 healthy: {}", health.l1_healthy);
println!("L2 healthy: {}", health.l2_healthy);
println!("L2 configured: {}", health.l2_configured);

if health.is_fully_healthy() {
    println!("All cache tiers operational");
}
```

### Metrics and Monitoring

```rust
// Get metrics snapshot
let metrics = cache.metrics_snapshot();

println!("L1 hits: {}", metrics.l1_hits);
println!("L1 misses: {}", metrics.l1_misses);
println!("L1 hit rate: {:.2}%", metrics.l1_hit_rate() * 100.0);

println!("L2 hits: {}", metrics.l2_hits);
println!("L2 misses: {}", metrics.l2_misses);
println!("L2 hit rate: {:.2}%", metrics.l2_hit_rate() * 100.0);

println!("Overall hit rate: {:.2}%", metrics.overall_hit_rate() * 100.0);

// Get cache sizes
println!("L1 entries: {}", cache.l1_entry_count());
if let Some(l2_size) = cache.l2_approximate_size().await {
    println!("L2 entries: {}", l2_size);
}
```

### Cache Invalidation

```rust
// Invalidate specific entry
cache.invalidate(&request).await;

// Clear all caches (use with caution!)
cache.clear_all().await;
```

### Custom TTL for L2

```rust
// Store with custom L2 TTL (7 days for this response)
cache.store_with_ttl(&request, response, 7 * 24 * 3600).await;
```

## Performance Targets

| Metric | Target | Typical |
|--------|--------|---------|
| L1 Latency | <1ms | <100μs |
| L2 Latency | 1-2ms | ~1.5ms |
| Overall Hit Rate (MVP) | >50% | 55-60% |
| Overall Hit Rate (Beta) | >70% | 75-80% |
| L1 Eviction Algorithm | TinyLFU | - |
| L2 Persistence | Redis TTL | - |

### Default Configuration

| Parameter | L1 Default | L2 Default |
|-----------|------------|------------|
| TTL | 300s (5 min) | 3600s (1 hour) |
| TTI | 120s (2 min) | N/A |
| Max Capacity | 1,000 entries | Limited by Redis memory |
| Eviction Policy | TinyLFU (LFU + LRU) | Redis TTL |
| Key Prefix | N/A | `llm_cache:` |

## Cache Key Generation

Cache keys are generated using SHA-256 hashing of the following components:

- Model name
- Prompt content
- Temperature (normalized to 2 decimal places)
- Max tokens
- Additional parameters (sorted for consistency)

```rust
use llm_edge_cache::key::{generate_cache_key, CacheableRequest};

let request = CacheableRequest::new("gpt-4", "Hello, world!")
    .with_temperature(0.7)
    .with_max_tokens(100);

let cache_key = generate_cache_key(&request);
// Returns: 64-character hex-encoded SHA-256 hash
```

**Note**: Temperature values are normalized to 2 decimal places to avoid floating-point precision issues. For example, `0.7` and `0.700001` will produce the same cache key.

## Prometheus Metrics

The crate exports the following Prometheus-compatible metrics:

- `llm_edge_cache_hits_total{tier="l1|l2"}` - Total cache hits per tier
- `llm_edge_cache_misses_total{tier="l1|l2"}` - Total cache misses per tier
- `llm_edge_cache_writes_total{tier="l1|l2"}` - Total cache writes per tier
- `llm_edge_cache_latency_ms{tier="l1|l2"}` - Cache operation latency histogram
- `llm_edge_cache_size_entries{tier="l1|l2"}` - Current cache size in entries
- `llm_edge_cache_memory_bytes{tier="l1|l2"}` - Current cache memory usage
- `llm_edge_requests_total` - Total requests processed

## Error Handling

The crate uses a graceful degradation model:

- If L2 (Redis) is unavailable at startup, the system falls back to L1-only mode
- If L2 becomes unavailable during operation, errors are logged but don't affect L1 operations
- All L2 writes are fire-and-forget (non-blocking)
- Timeouts are enforced on all Redis operations (default: 100ms)

```rust
// L2 errors don't crash the application
let cache = CacheManager::with_l2(l2_config).await;
// Even if Redis is down, this will succeed with L1-only mode

// Check if L2 is actually available
if cache.has_l2() {
    println!("L2 cache is available");
} else {
    println!("Running in L1-only mode");
}
```

## Testing

Run the test suite:

```bash
# Unit tests (no Redis required)
cargo test

# Integration tests (requires Redis)
docker run -d -p 6379:6379 redis:7-alpine
cargo test -- --ignored
```

## Performance Considerations

### L1 Cache (Moka)

- **Pros**: Extremely fast (<100μs), no network overhead, TinyLFU eviction
- **Cons**: Per-instance (not shared), limited capacity, lost on restart
- **Best for**: Hot data, frequently accessed prompts, high-throughput scenarios

### L2 Cache (Redis)

- **Pros**: Shared across instances, persistent, larger capacity
- **Cons**: Network latency (1-2ms), requires Redis infrastructure
- **Best for**: Warm data, multi-instance deployments, cost reduction

### Optimization Tips

1. **Adjust L1 capacity** based on your working set size and memory constraints
2. **Tune TTL values** based on your use case (longer for stable prompts, shorter for dynamic content)
3. **Monitor hit rates** and adjust configuration accordingly
4. **Use custom TTLs** for responses that should be cached longer (e.g., documentation lookups)
5. **Consider L1-only mode** for single-instance deployments to reduce infrastructure complexity

## Examples

See the [examples directory](../../examples/) for complete examples:

- `basic_cache.rs` - Simple L1-only caching
- `distributed_cache.rs` - L1 + L2 setup with Redis
- `metrics_monitoring.rs` - Prometheus metrics integration

## Contributing

Contributions are welcome! Please see the [contributing guidelines](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/CONTRIBUTING.md) for more information.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE) for details.

## Links

- [Repository](https://github.com/globalbusinessadvisors/llm-edge-agent)
- [Documentation](https://docs.rs/llm-edge-cache)
- [Crates.io](https://crates.io/crates/llm-edge-cache)
- [Issue Tracker](https://github.com/globalbusinessadvisors/llm-edge-agent/issues)
