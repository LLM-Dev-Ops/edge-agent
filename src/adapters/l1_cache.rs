//! L1 Cache benchmark adapter

use super::BenchTarget;
use crate::benchmarks::BenchmarkResult;
use crate::cache::key::CacheableRequest;
use crate::cache::l1::CachedResponse;
use crate::cache::CacheManager;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::error::Error;
use std::time::Instant;

/// Benchmark adapter for L1 cache operations
pub struct L1CacheBenchmark {
    iterations: usize,
}

impl L1CacheBenchmark {
    /// Create a new L1 cache benchmark with default iterations
    pub fn new() -> Self {
        Self { iterations: 1000 }
    }

    /// Create a new L1 cache benchmark with custom iterations
    #[allow(dead_code)]
    pub fn with_iterations(iterations: usize) -> Self {
        Self { iterations }
    }

    /// Helper to create a test request
    fn create_request(&self, i: usize) -> CacheableRequest {
        CacheableRequest::new("gpt-4", format!("Benchmark prompt {}", i))
            .with_temperature(0.7)
            .with_max_tokens(100)
    }

    /// Helper to create a test response
    fn create_response(&self, i: usize) -> CachedResponse {
        CachedResponse {
            content: format!("Benchmark response {}", i),
            tokens: None,
            model: "gpt-4".to_string(),
            cached_at: Utc::now().timestamp(),
        }
    }
}

impl Default for L1CacheBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for L1CacheBenchmark {
    fn id(&self) -> String {
        "l1_cache".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error + Send + Sync>> {
        let cache = CacheManager::new();

        // Benchmark: Cache write operation
        let write_start = Instant::now();
        for i in 0..self.iterations {
            let request = self.create_request(i);
            let response = self.create_response(i);
            cache.store(&request, response).await;
        }
        let write_duration = write_start.elapsed();

        // Benchmark: Cache read hit
        let read_start = Instant::now();
        for i in 0..self.iterations {
            let request = self.create_request(i);
            let _ = cache.lookup(&request).await;
        }
        let read_hit_duration = read_start.elapsed();

        // Benchmark: Cache read miss
        let read_miss_start = Instant::now();
        for i in 0..self.iterations {
            let request = CacheableRequest::new("gpt-4", format!("Missing prompt {}", i));
            let _ = cache.lookup(&request).await;
        }
        let read_miss_duration = read_miss_start.elapsed();

        let metrics = json!({
            "iterations": self.iterations,
            "write_total_ms": write_duration.as_secs_f64() * 1000.0,
            "write_avg_us": (write_duration.as_micros() as f64) / (self.iterations as f64),
            "read_hit_total_ms": read_hit_duration.as_secs_f64() * 1000.0,
            "read_hit_avg_us": (read_hit_duration.as_micros() as f64) / (self.iterations as f64),
            "read_miss_total_ms": read_miss_duration.as_secs_f64() * 1000.0,
            "read_miss_avg_us": (read_miss_duration.as_micros() as f64) / (self.iterations as f64),
            "throughput_ops_per_sec": (self.iterations as f64) / write_duration.as_secs_f64(),
        });

        Ok(BenchmarkResult::new(self.id(), metrics))
    }
}
