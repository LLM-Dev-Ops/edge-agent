# LLM Edge Agent Benchmarks

This directory contains benchmark infrastructure for measuring the performance of LLM Edge Agent components.

## Directory Structure

```
benchmarks/
├── README.md           # This file
├── output/             # Benchmark results (created automatically)
│   ├── summary.md      # Human-readable summary
│   └── raw/            # Raw JSON results with timestamps
├── cache_benchmarks.rs # Criterion-based cache benchmarks
└── routing_benchmarks.rs # Criterion-based routing benchmarks
```

## Running Benchmarks

### Quick Start

Run all benchmarks and generate reports:

```bash
# Using npm
npm run bench

# Using CLI
llm-edge-agent benchmark

# Direct cargo
cargo run --bin benchmark --release
```

### Output

Benchmark results are automatically saved to:
- `benchmarks/output/summary.md` - Markdown summary with latest results
- `benchmarks/output/raw/benchmarks-YYYYMMDD-HHMMSS.json` - Raw JSON data

### Development

For faster iteration during development (without optimizations):

```bash
npm run bench:dev
```

## Benchmark Categories

### L1 Cache Performance
Tests in-memory cache operations:
- Write operations (store)
- Read hits (lookup cached items)
- Read misses (lookup non-existent items)
- Throughput (operations per second)

**Performance Targets:**
- Write: < 100μs average
- Read hit: < 50μs average
- Read miss: < 10μs average
- Throughput: > 100k ops/sec

### Routing Performance
Tests routing decision algorithms:
- Model-based routing
- Cost-optimized routing
- Latency-optimized routing
- Failover routing
- Scalability (varying provider counts)

**Performance Targets:**
- Routing decision: < 1ms
- Handles 50+ providers efficiently

## Architecture

### Benchmark Framework

The benchmark infrastructure is modular and extensible:

```
src/
├── benchmarks/          # Core benchmark framework
│   ├── mod.rs          # run_all_benchmarks() entry point
│   ├── result.rs       # BenchmarkResult type
│   ├── io.rs           # JSON file I/O
│   └── markdown.rs     # Markdown report generation
└── adapters/           # Benchmark implementations
    ├── mod.rs          # BenchTarget trait & registry
    └── l1_cache.rs     # L1 cache benchmark adapter
```

### Adding New Benchmarks

1. Create a new adapter in `src/adapters/`:

```rust
use super::BenchTarget;
use crate::benchmarks::BenchmarkResult;
use async_trait::async_trait;
use serde_json::json;

pub struct MyBenchmark;

#[async_trait]
impl BenchTarget for MyBenchmark {
    fn id(&self) -> String {
        "my_benchmark".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error + Send + Sync>> {
        // Run benchmark and collect metrics
        let metrics = json!({
            "metric_name": value,
        });

        Ok(BenchmarkResult::new(self.id(), metrics))
    }
}
```

2. Register in `src/adapters/mod.rs`:

```rust
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        Box::new(l1_cache::L1CacheBenchmark::new()),
        Box::new(my_benchmark::MyBenchmark::new()), // Add here
    ]
}
```

3. Run benchmarks - your new benchmark will be included automatically!

## Criterion Benchmarks

For more detailed statistical analysis, use Criterion-based benchmarks:

```bash
# Run all Criterion benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench cache_benchmarks
cargo bench --bench routing_benchmarks

# View results
open target/criterion/report/index.html
```

Criterion provides:
- Statistical analysis with confidence intervals
- Regression detection
- HTML reports with charts
- Comparison with previous runs

## Continuous Integration

Benchmarks run automatically in CI/CD:
- On pull requests (to detect performance regressions)
- Nightly builds (for trend analysis)
- Release builds (for performance validation)

Results are archived and tracked over time to monitor performance trends.

## Troubleshooting

### Cargo not found

Ensure Rust is installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Benchmark fails to compile

Check dependencies:
```bash
cargo check --bin benchmark
```

### Results not generated

Ensure output directory exists (created automatically):
```bash
mkdir -p benchmarks/output/raw
```

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flamegraph Profiling](https://github.com/flamegraph-rs/flamegraph)
