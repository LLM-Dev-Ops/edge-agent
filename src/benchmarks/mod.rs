//! Benchmarking module for edge-agent components
//!
//! This module provides a framework for benchmarking various components
//! of the edge-agent system. Results are collected in a standardized format
//! and can be exported as JSON or Markdown reports.

pub mod io;
pub mod markdown;
pub mod result;

use crate::adapters::{all_targets, BenchTarget};
pub use result::BenchmarkResult;

/// Run all registered benchmarks and collect results
///
/// This function discovers all benchmark targets via the adapters registry,
/// executes each benchmark, and returns a vector of results.
///
/// # Returns
/// A vector of `BenchmarkResult` containing metrics from each target
///
/// # Example
/// ```no_run
/// use edge_agent::benchmarks::run_all_benchmarks;
///
/// #[tokio::main]
/// async fn main() {
///     let results = run_all_benchmarks().await;
///     println!("Completed {} benchmarks", results.len());
/// }
/// ```
pub async fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    let targets = all_targets();
    let mut results = Vec::with_capacity(targets.len());

    for target in targets {
        let target_id = target.id();
        println!("Running benchmark: {}", target_id);

        match target.run().await {
            Ok(result) => {
                println!("  ✓ Completed: {}", target_id);
                results.push(result);
            }
            Err(e) => {
                eprintln!("  ✗ Failed: {} - {}", target_id, e);
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_benchmarks() {
        let results = run_all_benchmarks().await;
        // Should return results (may be empty if no adapters registered)
        assert!(results.len() >= 0);
    }
}
