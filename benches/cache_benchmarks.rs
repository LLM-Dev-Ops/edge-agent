// Cache Performance Benchmarks
// Measures L1 and L2 cache performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use llm_edge_cache::{CacheManager, CacheableRequest, CachedResponse, L2Config};
use tokio::runtime::Runtime;

fn create_test_request(id: usize) -> CacheableRequest {
    CacheableRequest {
        model: format!("gpt-3.5-turbo"),
        messages: vec![format!("Test message {}", id)],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
    }
}

fn create_test_response() -> CachedResponse {
    CachedResponse {
        content: "Test response".to_string(),
        model: "gpt-3.5-turbo".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cached_at: chrono::Utc::now(),
    }
}

// Benchmark L1 cache operations
fn bench_l1_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("l1_cache");

    group.throughput(Throughput::Elements(1));

    // Benchmark: L1 write
    group.bench_function("write", |b| {
        b.to_async(&rt).iter(|| async {
            let cache = CacheManager::new();
            let request = create_test_request(0);
            let response = create_test_response();
            cache.store(&request, response).await
        });
    });

    // Benchmark: L1 read (hit)
    group.bench_function("read_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let cache = CacheManager::new();
            let request = create_test_request(0);
            let response = create_test_response();

            // Warm up cache
            cache.store(&request, response).await;

            // Measure read
            black_box(cache.lookup(&request).await)
        });
    });

    // Benchmark: L1 read (miss)
    group.bench_function("read_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let cache = CacheManager::new();
            let request = create_test_request(0);
            black_box(cache.lookup(&request).await)
        });
    });

    group.finish();
}

// Benchmark cache with different sizes
fn bench_cache_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_sizes");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let cache = CacheManager::new();

                // Fill cache to specified size
                for i in 0..size {
                    let request = create_test_request(i);
                    let response = create_test_response();
                    cache.store(&request, response).await;
                }

                // Measure random access
                let request = create_test_request(size / 2);
                black_box(cache.lookup(&request).await)
            });
        });
    }

    group.finish();
}

// Benchmark cache key generation
fn bench_cache_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_keys");

    group.bench_function("generate_key", |b| {
        let request = create_test_request(0);
        b.iter(|| {
            black_box(llm_edge_cache::generate_cache_key(&request))
        });
    });

    group.finish();
}

// Benchmark concurrent cache access
fn bench_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_access");

    for concurrency in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async move {
                    let cache = std::sync::Arc::new(CacheManager::new());

                    // Spawn concurrent tasks
                    let mut handles = vec![];
                    for i in 0..concurrency {
                        let cache_clone = cache.clone();
                        let handle = tokio::spawn(async move {
                            let request = create_test_request(i);
                            let response = create_test_response();
                            cache_clone.store(&request, response).await;
                            cache_clone.lookup(&request).await
                        });
                        handles.push(handle);
                    }

                    // Wait for all tasks
                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_l1_cache_operations,
    bench_cache_sizes,
    bench_cache_key_generation,
    bench_concurrent_access
);

criterion_main!(benches);
