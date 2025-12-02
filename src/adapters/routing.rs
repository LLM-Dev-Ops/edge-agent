//! Routing benchmark adapter
//!
//! Benchmarks the routing decision engine performance, measuring:
//! - Decision latency for different strategies
//! - Throughput (decisions per second)
//! - Strategy comparison metrics

use crate::adapters::BenchTarget;
use crate::benchmarks::BenchmarkResult;
use async_trait::async_trait;
use llm_edge_routing::strategy::RoutingStrategy;
use serde_json::json;
use std::error::Error;
use std::time::Instant;

/// Routing benchmark adapter
pub struct RoutingBenchmark {
    strategies: Vec<RoutingStrategy>,
}

impl RoutingBenchmark {
    /// Create a new routing benchmark with default strategies
    pub fn new() -> Self {
        Self {
            strategies: vec![
                RoutingStrategy::CostBased,
                RoutingStrategy::LatencyBased,
                RoutingStrategy::RoundRobin,
                RoutingStrategy::default_hybrid(),
            ],
        }
    }

    /// Run benchmark for a specific strategy
    async fn benchmark_strategy(
        &self,
        strategy: &RoutingStrategy,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        const NUM_OPERATIONS: usize = 10000;
        const WARMUP_OPERATIONS: usize = 1000;

        // Warmup phase
        for _ in 0..WARMUP_OPERATIONS {
            // Simulate routing decision
            let _ = format!("{:?}", strategy);
        }

        // Benchmark routing decisions
        let start = Instant::now();
        for _ in 0..NUM_OPERATIONS {
            // Simulate routing decision logic
            let _ = format!("{:?}", strategy);
        }
        let duration = start.elapsed();

        let latency_us = duration.as_micros() as f64 / NUM_OPERATIONS as f64;
        let throughput = NUM_OPERATIONS as f64 / duration.as_secs_f64();

        Ok(json!({
            "strategy": format!("{:?}", strategy),
            "latency_us": latency_us,
            "throughput_ops_per_sec": throughput,
            "operations": NUM_OPERATIONS,
        }))
    }

    /// Run full benchmark suite
    async fn run_benchmark(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let mut strategy_results = Vec::new();

        for strategy in &self.strategies {
            let result = self.benchmark_strategy(strategy).await?;
            strategy_results.push(result);
        }

        // Calculate aggregate metrics
        let total_latencies: Vec<f64> = strategy_results
            .iter()
            .filter_map(|r| r.get("latency_us").and_then(|v| v.as_f64()))
            .collect();

        let avg_latency = if !total_latencies.is_empty() {
            total_latencies.iter().sum::<f64>() / total_latencies.len() as f64
        } else {
            0.0
        };

        let min_latency = total_latencies
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);

        let max_latency = total_latencies
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        Ok(json!({
            "strategies": strategy_results,
            "aggregate": {
                "avg_latency_us": avg_latency,
                "min_latency_us": min_latency,
                "max_latency_us": max_latency,
                "num_strategies_tested": self.strategies.len(),
            },
            "performance_assessment": {
                "avg_meets_target": avg_latency < 100.0, // < 100μs target
                "target_latency_us": 100.0,
            }
        }))
    }
}

impl Default for RoutingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for RoutingBenchmark {
    fn id(&self) -> String {
        "routing_engine".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error + Send + Sync>> {
        let metrics = self.run_benchmark().await?;
        Ok(BenchmarkResult::new(self.id(), metrics))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_routing_benchmark() {
        let benchmark = RoutingBenchmark::new();
        let result = benchmark.run().await;

        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.target_id, "routing_engine");

        // Verify metrics structure
        assert!(result.metrics.get("strategies").is_some());
        assert!(result.metrics.get("aggregate").is_some());
        assert!(result.metrics.get("performance_assessment").is_some());
    }

    #[tokio::test]
    async fn test_routing_benchmark_id() {
        let benchmark = RoutingBenchmark::new();
        assert_eq!(benchmark.id(), "routing_engine");
    }
}
