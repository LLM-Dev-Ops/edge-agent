//! Benchmark runner binary
//!
//! This binary runs all registered benchmarks and outputs results to the
//! canonical output directory structure.

use chrono::Utc;
use edge_agent::benchmarks::{io, markdown, run_all_benchmarks};
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LLM Edge Agent - Benchmark Runner");
    println!("==================================\n");

    // Ensure output directories exist
    let output_dir = PathBuf::from("benchmarks/output");
    let raw_dir = output_dir.join("raw");
    fs::create_dir_all(&raw_dir)?;

    println!("Running all benchmarks...\n");

    // Run all benchmarks
    let results = run_all_benchmarks().await;

    println!("\n==================================");
    println!("Benchmark run complete!");
    println!("Total benchmarks: {}\n", results.len());

    // Generate timestamp for filenames
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");

    // Write JSON results
    let json_path = raw_dir.join(format!("benchmarks-{}.json", timestamp));
    io::write_json_results(&results, &json_path)?;
    println!("Raw JSON results: {}", json_path.display());

    // Write/update markdown summary
    let summary_path = output_dir.join("summary.md");
    markdown::write_markdown_report(&results, &summary_path)?;
    println!("Markdown summary: {}", summary_path.display());

    println!("\nBenchmark results have been written successfully.");

    Ok(())
}
