# Development Guide

This guide covers the development workflow for LLM Edge Agent contributors.

## Table of Contents

1. [Development Setup](#development-setup)
2. [Project Structure](#project-structure)
3. [Building and Testing](#building-and-testing)
4. [Code Style](#code-style)
5. [Adding Features](#adding-features)
6. [Debugging](#debugging)
7. [Performance Profiling](#performance-profiling)

## Development Setup

### Prerequisites

- **Rust**: 1.75 or higher
- **Docker**: For running dependencies (Redis, etc.)
- **Git**: For version control

### Initial Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/llm-edge-agent.git
   cd llm-edge-agent
   ```

2. Install Rust toolchain:
   ```bash
   rustup update stable
   rustup component add rustfmt clippy
   ```

3. Install development tools:
   ```bash
   cargo install cargo-watch  # Auto-rebuild on file changes
   cargo install cargo-tarpaulin  # Code coverage
   cargo install cargo-audit  # Security auditing
   ```

4. Set up environment:
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

5. Start dependencies:
   ```bash
   docker run -d -p 6379:6379 --name redis-dev redis:7-alpine
   ```

## Project Structure

```
llm-edge-agent/
├── crates/                      # Rust workspace crates
│   ├── llm-edge-agent/         # Main binary crate
│   ├── llm-edge-proxy/         # HTTP proxy functionality
│   ├── llm-edge-cache/         # Multi-tier caching
│   ├── llm-edge-routing/       # Intelligent routing
│   ├── llm-edge-providers/     # LLM provider adapters
│   ├── llm-edge-security/      # Auth, validation, PII
│   └── llm-edge-monitoring/    # Metrics and tracing
├── config/                      # Configuration templates
├── deployments/                 # Deployment configurations
├── docs/                        # Documentation
├── tests/                       # Integration tests
└── Cargo.toml                   # Workspace manifest
```

### Crate Responsibilities

- **llm-edge-agent**: Main binary, orchestrates all components
- **llm-edge-proxy**: HTTP server, TLS, middleware
- **llm-edge-cache**: L1/L2/L3 caching logic
- **llm-edge-routing**: Routing strategies, circuit breakers
- **llm-edge-providers**: Provider adapters (OpenAI, Anthropic, etc.)
- **llm-edge-security**: Authentication, validation, PII redaction
- **llm-edge-monitoring**: Metrics, tracing, logging

## Building and Testing

### Build Commands

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (optimized)
cargo build --release

# Build specific crate
cargo build -p llm-edge-cache

# Build with all features
cargo build --all-features
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests for specific crate
cargo test -p llm-edge-cache

# Run integration tests (requires Redis)
cargo test -- --ignored

# Run with coverage
cargo tarpaulin --verbose --all-features --workspace --timeout 120
```

### Watch Mode (Auto-rebuild)

```bash
# Watch and rebuild on changes
cargo watch -x build

# Watch and run tests
cargo watch -x test

# Watch and run specific binary
cargo watch -x "run --bin llm-edge-agent"
```

## Code Style

### Formatting

We use `rustfmt` with default settings:

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Linting

We use `clippy` with strict lints:

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix auto-fixable issues
cargo clippy --fix
```

### Code Standards

1. **Error Handling**: Use `Result` and `?` operator, avoid `unwrap()` in production code
2. **Async**: Use `tokio` for async runtime, prefer `async/await` over raw futures
3. **Documentation**: Add doc comments (`///`) for public APIs
4. **Testing**: Write tests for all new functionality
5. **Security**: Never log secrets, use `secrecy` crate for sensitive data

### Example

```rust
/// Calculates the cost of an LLM request
///
/// # Arguments
/// * `provider` - The LLM provider name
/// * `model` - The model identifier
/// * `tokens` - Token usage
///
/// # Returns
/// The cost in USD, or an error if pricing unavailable
pub fn calculate_cost(
    provider: &str,
    model: &str,
    tokens: usize,
) -> Result<f64, CostError> {
    let pricing = get_pricing(provider, model)
        .ok_or_else(|| CostError::PricingUnavailable)?;

    Ok(pricing.per_token * tokens as f64)
}
```

## Adding Features

### Adding a New Provider

1. Create adapter in `crates/llm-edge-providers/src/`:
   ```rust
   // crates/llm-edge-providers/src/newprovider.rs
   use crate::{LLMProvider, UnifiedRequest, UnifiedResponse};

   pub struct NewProviderAdapter {
       // ...
   }

   #[async_trait]
   impl LLMProvider for NewProviderAdapter {
       // Implement trait methods
   }
   ```

2. Add configuration in `config/config.yaml`:
   ```yaml
   providers:
     newprovider:
       enabled: true
       api_key: "${NEWPROVIDER_API_KEY}"
   ```

3. Register in main application

4. Add tests:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_newprovider_request() {
           // Test implementation
       }
   }
   ```

### Adding a Cache Tier

1. Create module in `crates/llm-edge-cache/src/`:
   ```rust
   // crates/llm-edge-cache/src/l4.rs
   pub struct L4Cache {
       // Implementation
   }
   ```

2. Update `CacheManager` to include new tier

3. Add configuration and tests

## Debugging

### Logging

Set logging level:
```bash
RUST_LOG=debug cargo run
RUST_LOG=llm_edge_agent=trace cargo run
```

### Using VS Code

Create `.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug llm-edge-agent",
      "cargo": {
        "args": ["build", "--bin=llm-edge-agent"],
        "filter": {
          "name": "llm-edge-agent",
          "kind": "bin"
        }
      },
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

### Common Issues

**Issue**: Redis connection timeout
```bash
# Solution: Ensure Redis is running
docker ps | grep redis
docker logs redis-dev
```

**Issue**: Compilation errors after update
```bash
# Solution: Clean and rebuild
cargo clean
cargo build
```

## Performance Profiling

### Benchmarking

Run benchmarks:
```bash
cargo bench
```

Create benchmark in `crates/llm-edge-cache/benches/cache_benchmarks.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn cache_benchmark(c: &mut Criterion) {
    c.bench_function("l1_cache_get", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

criterion_group!(benches, cache_benchmark);
criterion_main!(benches);
```

### CPU Profiling

Using `perf`:
```bash
cargo build --release
perf record --call-graph=dwarf target/release/llm-edge-agent
perf report
```

Using `flamegraph`:
```bash
cargo install flamegraph
cargo flamegraph
```

### Memory Profiling

Using `valgrind`:
```bash
cargo build
valgrind --tool=massif target/debug/llm-edge-agent
```

## CI/CD

### GitHub Actions

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests

Local CI simulation:
```bash
# Run the same checks as CI
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all-features
cargo audit
```

### Pre-commit Hook

Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."
cargo fmt --check
cargo clippy -- -D warnings
cargo test

echo "All checks passed!"
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:
- Submitting pull requests
- Code review process
- Release workflow

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/)
- Project Architecture: [plans/LLM_EDGE_AGENT_CONSOLIDATED_PLAN.md](plans/LLM_EDGE_AGENT_CONSOLIDATED_PLAN.md)
