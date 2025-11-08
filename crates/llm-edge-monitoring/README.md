# llm-edge-monitoring

[![Crates.io](https://img.shields.io/crates/v/llm-edge-monitoring.svg)](https://crates.io/crates/llm-edge-monitoring)
[![Documentation](https://docs.rs/llm-edge-monitoring/badge.svg)](https://docs.rs/llm-edge-monitoring)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE)

Observability and monitoring for LLM Edge Agent. This crate provides comprehensive metrics, tracing, and cost tracking capabilities for production LLM deployments.

## Features

- **Prometheus Metrics**: Production-ready metrics exporters with customizable labels
- **OpenTelemetry Tracing**: Distributed tracing support with OTLP exporter
- **Cost Tracking**: Real-time cost monitoring across providers and models
- **Cache Metrics**: Multi-tier cache hit/miss tracking
- **Health Monitoring**: Provider availability and health checks
- **Request Analytics**: Latency, throughput, and error rate tracking

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-edge-monitoring = "0.1.0"
```

## Metrics Catalog

### Request Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_edge_requests_total` | Counter | `provider`, `model`, `status`, `error_type` | Total number of requests processed |
| `llm_edge_request_duration_ms` | Histogram | `provider`, `model` | Request latency in milliseconds |
| `llm_edge_active_requests` | Gauge | - | Number of currently active requests |

### Token Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_edge_tokens_total` | Counter | `provider`, `model`, `type` | Total tokens used (input/output) |

### Cost Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_edge_cost_usd_total` | Counter | `provider`, `model` | Total cost in USD |

### Cache Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_edge_cache_hits_total` | Counter | `tier` | Cache hits by tier (L1/L2/L3) |
| `llm_edge_cache_misses_total` | Counter | `tier` | Cache misses by tier |

### Health Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `llm_edge_provider_available` | Gauge | `provider` | Provider health status (1=healthy, 0=unhealthy) |

## Usage Examples

### Basic Metrics Recording

```rust
use llm_edge_monitoring::metrics;

// Record a successful request
metrics::record_request_success("openai", "gpt-4", 245);

// Record a failed request
metrics::record_request_failure("anthropic", "claude-3", "rate_limit");

// Record token usage
metrics::record_token_usage("openai", "gpt-4", 150, 500);

// Record cost
metrics::record_cost("openai", "gpt-4", 0.0075);
```

### Cache Metrics

```rust
use llm_edge_monitoring::metrics;

// Record cache hits and misses
metrics::record_cache_hit("L1");
metrics::record_cache_miss("L2");
```

### Health Monitoring

```rust
use llm_edge_monitoring::metrics;

// Update provider health status
metrics::record_provider_health("openai", true);
metrics::record_provider_health("anthropic", false);

// Track active requests
metrics::record_active_requests(42);
```

### Complete Request Lifecycle

```rust
use llm_edge_monitoring::metrics;
use std::time::Instant;

async fn handle_llm_request(
    provider: &str,
    model: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Increment active requests
    metrics::record_active_requests(1);

    // Make the LLM request
    let result = make_llm_call(provider, model).await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(response) => {
            // Record success metrics
            metrics::record_request_success(provider, model, latency_ms);
            metrics::record_token_usage(provider, model, 150, 500);
            metrics::record_cost(provider, model, 0.0075);
            Ok(response)
        }
        Err(e) => {
            // Record failure metrics
            metrics::record_request_failure(provider, model, "api_error");
            Err(e)
        }
    }
}

async fn make_llm_call(provider: &str, model: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Your LLM call implementation
    Ok("response".to_string())
}
```

## Tracing Setup

OpenTelemetry tracing support is included for distributed tracing:

```rust
use llm_edge_monitoring::tracing;

// Note: Tracing setup is currently under development
// Future API will support OTLP exporter configuration and span creation
```

## Integration with Grafana

### Prometheus Configuration

Add the metrics endpoint to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'llm-edge-agent'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:9090']
```

### Sample Grafana Queries

**Request Rate by Provider:**
```promql
rate(llm_edge_requests_total[5m])
```

**Average Request Latency:**
```promql
rate(llm_edge_request_duration_ms_sum[5m]) /
rate(llm_edge_request_duration_ms_count[5m])
```

**Cache Hit Rate:**
```promql
sum(rate(llm_edge_cache_hits_total[5m])) /
(sum(rate(llm_edge_cache_hits_total[5m])) + sum(rate(llm_edge_cache_misses_total[5m])))
```

**Cost per Hour:**
```promql
rate(llm_edge_cost_usd_total[1h]) * 3600
```

**Error Rate:**
```promql
rate(llm_edge_requests_total{status="error"}[5m]) /
rate(llm_edge_requests_total[5m])
```

## Cost Tracking

Monitor your LLM costs in real-time:

```rust
use llm_edge_monitoring::metrics;

// Calculate and record costs based on token usage
fn calculate_cost(provider: &str, model: &str, input_tokens: usize, output_tokens: usize) -> f64 {
    let cost = match (provider, model) {
        ("openai", "gpt-4") => {
            (input_tokens as f64 * 0.00003) + (output_tokens as f64 * 0.00006)
        }
        ("anthropic", "claude-3-opus") => {
            (input_tokens as f64 * 0.000015) + (output_tokens as f64 * 0.000075)
        }
        _ => 0.0,
    };

    metrics::record_cost(provider, model, cost);
    cost
}
```

## Error Handling

The crate provides custom error types for monitoring operations:

```rust
use llm_edge_monitoring::{MonitoringError, MonitoringResult};

fn example() -> MonitoringResult<()> {
    // Your monitoring code here
    Ok(())
}
```

## Advanced Features

### Multi-Tier Cache Tracking

Track performance across different cache tiers:

```rust
// L1 (in-memory), L2 (Redis), L3 (DynamoDB)
metrics::record_cache_hit("L1");
metrics::record_cache_miss("L1");
metrics::record_cache_hit("L2");
```

### Provider Failover Monitoring

Monitor provider health during failover scenarios:

```rust
// Primary provider fails
metrics::record_provider_health("primary", false);
metrics::record_request_failure("primary", "gpt-4", "timeout");

// Failover to secondary
metrics::record_provider_health("secondary", true);
metrics::record_request_success("secondary", "claude-3", 180);
```

## Development

Run tests:

```bash
cargo test
```

Check documentation:

```bash
cargo doc --open
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE) for details.

## Contributing

Contributions are welcome! Please see our [Contributing Guide](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/CONTRIBUTING.md) for details.

## Related Crates

- `llm-edge-core` - Core abstractions and traits
- `llm-edge-providers` - Provider implementations
- `llm-edge-cache` - Multi-tier caching layer
- `llm-edge-orchestrator` - Request orchestration and routing

## Resources

- [Repository](https://github.com/globalbusinessadvisors/llm-edge-agent)
- [Documentation](https://docs.rs/llm-edge-monitoring)
- [Issue Tracker](https://github.com/globalbusinessadvisors/llm-edge-agent/issues)
