//! I/O utilities for benchmark results

use super::result::BenchmarkResult;
use std::fs;
use std::io;
use std::path::Path;

/// Write benchmark results to a JSON file
///
/// # Arguments
/// * `results` - Vector of benchmark results to write
/// * `path` - Path to the output file
///
/// # Errors
/// Returns an error if the file cannot be written
pub fn write_json_results<P: AsRef<Path>>(
    results: &[BenchmarkResult],
    path: P,
) -> io::Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    fs::write(path, json)?;
    Ok(())
}

/// Read benchmark results from a JSON file
///
/// # Arguments
/// * `path` - Path to the input file
///
/// # Errors
/// Returns an error if the file cannot be read or parsed
pub fn read_json_results<P: AsRef<Path>>(path: P) -> io::Result<Vec<BenchmarkResult>> {
    let contents = fs::read_to_string(path)?;
    let results: Vec<BenchmarkResult> = serde_json::from_str(&contents)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_and_read_json_results() {
        let results = vec![
            BenchmarkResult::new(
                "test_target_1".to_string(),
                json!({
                    "latency_ms": 1.5,
                    "throughput": 1000
                }),
            ),
            BenchmarkResult::new(
                "test_target_2".to_string(),
                json!({
                    "latency_ms": 2.0,
                    "throughput": 800
                }),
            ),
        ];

        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write results
        write_json_results(&results, path).unwrap();

        // Read results back
        let read_results = read_json_results(path).unwrap();

        assert_eq!(read_results.len(), 2);
        assert_eq!(read_results[0].target_id, "test_target_1");
        assert_eq!(read_results[1].target_id, "test_target_2");
    }
}
