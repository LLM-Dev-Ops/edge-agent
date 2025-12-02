//! Markdown report generation for benchmark results

use super::result::BenchmarkResult;
use std::fs;
use std::io;
use std::path::Path;

/// Generate a markdown report from benchmark results
///
/// # Arguments
/// * `results` - Vector of benchmark results to format
///
/// # Returns
/// A formatted markdown string
pub fn generate_markdown_report(results: &[BenchmarkResult]) -> String {
    let mut report = String::new();

    report.push_str("# Benchmark Results\n\n");
    report.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    if results.is_empty() {
        report.push_str("No benchmark results available.\n");
        return report;
    }

    report.push_str("## Summary\n\n");
    report.push_str(&format!("Total benchmarks: {}\n\n", results.len()));

    report.push_str("## Detailed Results\n\n");

    for result in results {
        report.push_str(&format!("### {}\n\n", result.target_id));
        report.push_str(&format!(
            "**Timestamp:** {}\n\n",
            result.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        report.push_str("**Metrics:**\n\n");

        // Format metrics as a JSON code block for better readability
        if let Ok(formatted_metrics) = serde_json::to_string_pretty(&result.metrics) {
            report.push_str("```json\n");
            report.push_str(&formatted_metrics);
            report.push_str("\n```\n\n");
        } else {
            report.push_str(&format!("{:?}\n\n", result.metrics));
        }

        report.push_str("---\n\n");
    }

    report
}

/// Write a markdown report to a file
///
/// # Arguments
/// * `results` - Vector of benchmark results to format
/// * `path` - Path to the output file
///
/// # Errors
/// Returns an error if the file cannot be written
pub fn write_markdown_report<P: AsRef<Path>>(
    results: &[BenchmarkResult],
    path: P,
) -> io::Result<()> {
    let report = generate_markdown_report(results);
    fs::write(path, report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_markdown_report_empty() {
        let results = vec![];
        let report = generate_markdown_report(&results);

        assert!(report.contains("# Benchmark Results"));
        assert!(report.contains("No benchmark results available"));
    }

    #[test]
    fn test_generate_markdown_report_with_results() {
        let results = vec![
            BenchmarkResult::new(
                "test_target".to_string(),
                json!({
                    "latency_ms": 1.5,
                    "throughput": 1000
                }),
            ),
        ];

        let report = generate_markdown_report(&results);

        assert!(report.contains("# Benchmark Results"));
        assert!(report.contains("### test_target"));
        assert!(report.contains("latency_ms"));
        assert!(report.contains("1.5"));
        assert!(report.contains("throughput"));
        assert!(report.contains("1000"));
    }
}
