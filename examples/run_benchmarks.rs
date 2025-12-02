//! Example: Running all benchmarks
//!
//! This example demonstrates how to use the benchmark framework to run
//! all registered benchmark targets and generate reports.
//!
//! Usage:
//!   cargo run --example run_benchmarks

use llm_edge_agent::benchmarks::{io, markdown, run_all_benchmarks};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("LLM Edge Agent - Benchmark Runner");
    println!("========================================\n");

    // Run all benchmarks
    println!("Executing all registered benchmarks...\n");
    let results = run_all_benchmarks().await;

    println!("\n========================================");
    println!("Benchmark Execution Complete");
    println!("========================================\n");
    println!("Total benchmarks completed: {}\n", results.len());

    // Create output directory if it doesn't exist
    std::fs::create_dir_all("benchmarks/output/raw")?;

    // Write JSON results
    let json_path = Path::new("benchmarks/output/raw/results.json");
    io::write_json_results(&results, json_path)?;
    println!("✓ JSON results written to: {}", json_path.display());

    // Write Markdown summary
    let md_path = Path::new("benchmarks/output/summary.md");
    markdown::write_markdown_report(&results, md_path)?;
    println!("✓ Markdown summary written to: {}", md_path.display());

    println!("\n========================================");
    println!("Results Summary");
    println!("========================================\n");

    for result in &results {
        println!("Target: {}", result.target_id);
        println!("Timestamp: {}", result.timestamp);
        if let Some(metrics) = result.metrics.as_object() {
            println!("Metrics:");
            for (key, value) in metrics {
                println!("  {}: {:?}", key, value);
            }
        }
        println!();
    }

    println!("========================================");
    println!("Benchmark run completed successfully!");
    println!("========================================");

    Ok(())
}
