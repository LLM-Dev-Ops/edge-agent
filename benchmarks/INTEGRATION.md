# Benchmark CLI Integration

This document describes the complete integration of benchmarks with the LLM Edge Agent CLI.

## Overview

The benchmark system has been fully integrated with the CLI, allowing developers to easily run performance benchmarks and generate reports in both JSON and Markdown formats.

## Architecture

### Components

1. **Rust Benchmark Runner** (`src/bin/benchmark.rs`)
   - Entry point for benchmark execution
   - Coordinates running all registered benchmarks
   - Generates timestamped output files
   - Writes both JSON and Markdown reports

2. **Benchmark Framework** (`src/benchmarks/`)
   - `mod.rs` - Core `run_all_benchmarks()` function
   - `result.rs` - BenchmarkResult type definition
   - `io.rs` - JSON file I/O utilities
   - `markdown.rs` - Markdown report generation

3. **Benchmark Adapters** (`src/adapters/`)
   - `l1_cache.rs` - L1 cache performance benchmarks
   - `routing.rs` - Routing algorithm benchmarks
   - `mod.rs` - BenchTarget trait and registry

4. **CLI Integration** (`bin/llm-edge-agent.js`)
   - `benchmark` command implementation
   - Cargo detection and error handling
   - User-friendly output formatting

5. **NPM Scripts** (`package.json`)
   - `npm run bench` - Run optimized benchmarks
   - `npm run bench:dev` - Run debug benchmarks

## Output Directory Structure

```
benchmarks/output/
├── summary.md                          # Latest benchmark summary
└── raw/                                # Historical results
    ├── benchmarks-20251202-143000.json
    ├── benchmarks-20251202-150000.json
    └── ...
```

### Output Format

#### JSON (`raw/benchmarks-*.json`)
```json
[
  {
    "target_id": "l1_cache",
    "metrics": {
      "iterations": 1000,
      "write_total_ms": 5.234,
      "write_avg_us": 5.234,
      "read_hit_total_ms": 2.456,
      "read_hit_avg_us": 2.456,
      "throughput_ops_per_sec": 191063.2
    },
    "timestamp": "2025-12-02T14:30:00Z"
  }
]
```

#### Markdown (`summary.md`)
```markdown
# Benchmark Results

Generated: 2025-12-02 14:30:00 UTC

## Summary

Total benchmarks: 2

## Detailed Results

### l1_cache

**Timestamp:** 2025-12-02 14:30:00 UTC

**Metrics:**

​```json
{
  "iterations": 1000,
  "write_total_ms": 5.234,
  ...
}
​```
```

## Usage

### Command Line

```bash
# Using the CLI command
llm-edge-agent benchmark

# With custom output directory
llm-edge-agent benchmark --output custom/path

# Using npm script (recommended)
npm run bench

# Development mode (faster compile, no optimizations)
npm run bench:dev

# Direct cargo invocation
cargo run --bin benchmark --release
```

### Programmatic Usage

```rust
use edge_agent::benchmarks::run_all_benchmarks;

#[tokio::main]
async fn main() {
    let results = run_all_benchmarks().await;

    for result in results {
        println!("{}: {:?}", result.target_id, result.metrics);
    }
}
```

## Adding New Benchmarks

### Step 1: Create Adapter

Create a new file in `src/adapters/` (e.g., `src/adapters/my_benchmark.rs`):

```rust
use super::BenchTarget;
use crate::benchmarks::BenchmarkResult;
use async_trait::async_trait;
use serde_json::json;
use std::error::Error;

pub struct MyBenchmark {
    // Configuration fields
}

impl MyBenchmark {
    pub fn new() -> Self {
        Self { /* ... */ }
    }
}

#[async_trait]
impl BenchTarget for MyBenchmark {
    fn id(&self) -> String {
        "my_benchmark".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error + Send + Sync>> {
        // Benchmark implementation
        let metrics = json!({
            "metric1": value1,
            "metric2": value2,
        });

        Ok(BenchmarkResult::new(self.id(), metrics))
    }
}
```

### Step 2: Register Adapter

Add to `src/adapters/mod.rs`:

```rust
pub mod my_benchmark;

pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        Box::new(l1_cache::L1CacheBenchmark::new()),
        Box::new(routing::RoutingBenchmark::new()),
        Box::new(my_benchmark::MyBenchmark::new()),  // Add here
    ]
}
```

### Step 3: Run Benchmarks

Your new benchmark will automatically be included:

```bash
npm run bench
```

## Configuration

### Benchmark Settings

Benchmarks can be configured through adapter constructors:

```rust
// Default settings
L1CacheBenchmark::new()

// Custom settings (if implemented)
L1CacheBenchmark::with_iterations(10000)
```

### Output Customization

The CLI command accepts options:

```bash
# Custom output directory
llm-edge-agent benchmark --output /path/to/output

# JSON only (skip markdown generation)
llm-edge-agent benchmark --json-only
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Benchmarks

on:
  pull_request:
  schedule:
    - cron: '0 0 * * *'  # Daily

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Benchmarks
        run: npm run bench

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benchmarks/output/
```

## Performance Targets

### L1 Cache
- Write operations: < 100μs average
- Read hits: < 50μs average
- Read misses: < 10μs average
- Throughput: > 100k ops/sec

### Routing
- Decision time: < 1ms
- Scalability: Handle 50+ providers efficiently

## Troubleshooting

### Issue: Cargo not found

**Solution:** Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Issue: Compilation errors

**Solution:** Check dependencies
```bash
cargo check --bin benchmark
cargo build --bin benchmark
```

### Issue: Output directory not created

**Solution:** Directory is created automatically, but you can create manually:
```bash
mkdir -p benchmarks/output/raw
```

### Issue: Permission denied

**Solution:** Ensure write permissions
```bash
chmod -R u+w benchmarks/output/
```

## Best Practices

1. **Run in Release Mode**
   - Always use `--release` flag for accurate measurements
   - Use `npm run bench` (not `bench:dev`) for production benchmarks

2. **Consistent Environment**
   - Run on same hardware for comparisons
   - Minimize background processes
   - Use dedicated benchmark machine for CI/CD

3. **Statistical Significance**
   - Run benchmarks multiple times
   - Look for trends over multiple runs
   - Use Criterion for statistical analysis

4. **Version Control**
   - Don't commit `benchmarks/output/` directory
   - Track summary results in pull requests
   - Archive historical data separately

5. **Regression Detection**
   - Compare results against baseline
   - Set thresholds for acceptable degradation
   - Fail CI if performance drops significantly

## Related Tools

- **Criterion.rs** - For detailed statistical benchmarks
  ```bash
  cargo bench
  ```

- **Flamegraph** - For profiling hotspots
  ```bash
  cargo install flamegraph
  cargo flamegraph --bin benchmark
  ```

- **perf** - Linux performance analysis
  ```bash
  perf record cargo run --bin benchmark --release
  perf report
  ```

## Maintenance

### Updating Benchmarks

When updating benchmark code:
1. Update adapter implementation
2. Test locally: `npm run bench:dev`
3. Run optimized: `npm run bench`
4. Compare results with previous runs
5. Update documentation if metrics change

### Adding Dependencies

If benchmarks need new dependencies:
1. Add to `Cargo.toml` under `[dependencies]`
2. Test compilation: `cargo check --bin benchmark`
3. Update this documentation

## Support

For issues or questions:
- Check [README.md](./README.md) for general usage
- See main project [CONTRIBUTING.md](../CONTRIBUTING.md)
- Open an issue on GitHub
