//! Benchmark adapters for edge-agent components
//!
//! This module provides adapters that implement the `BenchTarget` trait
//! for various components of the edge-agent system. Each adapter is responsible
//! for benchmarking a specific component and returning standardized results.

pub mod l1_cache;
pub mod routing;

use crate::benchmarks::BenchmarkResult;
use async_trait::async_trait;
use std::error::Error;

/// Trait for benchmark targets
///
/// Implement this trait to create a new benchmark adapter for a component.
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target
    fn id(&self) -> String;

    /// Runs the benchmark and returns results
    ///
    /// # Errors
    /// Returns an error if the benchmark fails to execute
    async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error + Send + Sync>>;
}

/// Registry of all available benchmark targets
///
/// This function returns all registered benchmark adapters.
/// Add new adapters to this function to include them in benchmark runs.
///
/// # Returns
/// A vector of boxed trait objects implementing `BenchTarget`
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        Box::new(l1_cache::L1CacheBenchmark::new()),
        Box::new(routing::RoutingBenchmark::new()),
    ]
}
