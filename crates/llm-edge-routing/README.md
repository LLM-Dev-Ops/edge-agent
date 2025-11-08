# llm-edge-routing

[![Crates.io](https://img.shields.io/crates/v/llm-edge-routing.svg)](https://crates.io/crates/llm-edge-routing)
[![Documentation](https://docs.rs/llm-edge-routing/badge.svg)](https://docs.rs/llm-edge-routing)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

Intelligent routing engine for LLM Edge Agent, providing smart request distribution across multiple LLM providers with built-in resilience and failover capabilities.

## Features

- **Multiple Routing Strategies**: Cost-based, latency-based, hybrid, and round-robin routing
- **Circuit Breakers**: Automatic failure detection and recovery to prevent cascading failures
- **Failover Support**: Seamless fallback to healthy providers when issues occur
- **Performance Optimization**: Intelligent load balancing based on real-time metrics
- **Provider Agnostic**: Works with any LLM provider through the `llm-edge-providers` interface
- **Async First**: Built on Tokio for high-performance concurrent operations
- **Observability**: Integrated tracing and metrics for monitoring routing decisions

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-edge-routing = "0.1.0"
```

## Routing Strategies

### Cost-Based Routing

Routes requests to the provider with the lowest cost per token:

```rust
use llm_edge_routing::strategy::RoutingStrategy;

let strategy = RoutingStrategy::CostBased;
```

**Use Case**: Budget-conscious applications where cost optimization is the primary concern.

### Latency-Based Routing

Routes requests to the fastest available provider based on historical latency:

```rust
use llm_edge_routing::strategy::RoutingStrategy;

let strategy = RoutingStrategy::LatencyBased;
```

**Use Case**: Real-time applications requiring the fastest possible response times.

### Hybrid Routing

Routes based on multiple weighted factors (cost, latency, reliability):

```rust
use llm_edge_routing::strategy::RoutingStrategy;

// Create hybrid strategy with custom weights
let strategy = RoutingStrategy::Hybrid {
    cost_weight: 0.4,
    latency_weight: 0.4,
    reliability_weight: 0.2,
};

// Or use default balanced weights
let strategy = RoutingStrategy::default_hybrid();
```

**Use Case**: Production applications requiring balanced performance across multiple criteria.

**Weight Guidelines**:
- `cost_weight`: 0.0-1.0 (higher = prioritize lower costs)
- `latency_weight`: 0.0-1.0 (higher = prioritize lower latency)
- `reliability_weight`: 0.0-1.0 (higher = prioritize higher uptime)
- Weights should sum to approximately 1.0 for best results

### Round-Robin Routing

Distributes requests evenly across all available providers:

```rust
use llm_edge_routing::strategy::RoutingStrategy;

let strategy = RoutingStrategy::RoundRobin;
```

**Use Case**: Testing, development, or uniform load distribution scenarios.

## Usage Examples

### Basic Routing Decision

```rust
use llm_edge_routing::{RoutingStrategy, RoutingDecision};

#[tokio::main]
async fn main() {
    // Initialize routing strategy
    let strategy = RoutingStrategy::default_hybrid();

    // Make routing decision (implementation would query provider metrics)
    // This is a simplified example showing the return type
    let decision = RoutingDecision {
        provider_name: "openai".to_string(),
        model: "gpt-4".to_string(),
        score: 0.85,
        reason: "Best hybrid score: low cost (0.9) + good latency (0.8)".to_string(),
    };

    println!("Selected provider: {}", decision.provider_name);
    println!("Model: {}", decision.model);
    println!("Routing score: {}", decision.score);
    println!("Reason: {}", decision.reason);
}
```

### Circuit Breaker Configuration

Prevent cascading failures by automatically opening the circuit after repeated failures:

```rust
use llm_edge_routing::circuit_breaker::{CircuitBreaker, CircuitState};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Create circuit breaker: open after 3 failures, retry after 30 seconds
    let circuit_breaker = CircuitBreaker::new(3, Duration::from_secs(30));

    // Check circuit state before making request
    match circuit_breaker.state() {
        CircuitState::Closed => {
            // Normal operation - make request
            println!("Circuit closed, proceeding with request");
        }
        CircuitState::Open => {
            // Circuit open - fail fast
            println!("Circuit open, using fallback provider");
        }
        CircuitState::HalfOpen => {
            // Testing recovery - allow one request
            println!("Circuit half-open, testing recovery");
        }
    }

    // Record success or failure
    circuit_breaker.record_success(); // Resets after 3 consecutive successes
    // circuit_breaker.record_failure(); // Opens after threshold reached
}
```

**Circuit Breaker States**:
- **Closed**: Normal operation, requests flow through
- **Open**: Too many failures detected, failing fast to prevent cascading failures
- **Half-Open**: Timeout elapsed, testing if service has recovered

**Configuration Parameters**:
- `threshold`: Number of consecutive failures before opening circuit (recommended: 3-5)
- `timeout`: Duration to wait before testing recovery (recommended: 30-60 seconds)

### Error Handling

```rust
use llm_edge_routing::{RoutingError, RoutingResult};

async fn route_request() -> RoutingResult<String> {
    // Example error scenarios
    Err(RoutingError::NoProvidersAvailable)
    // or Err(RoutingError::AllProvidersFailed)
    // or Err(RoutingError::CircuitBreakerOpen("openai".to_string()))
}

#[tokio::main]
async fn main() {
    match route_request().await {
        Ok(result) => println!("Success: {}", result),
        Err(RoutingError::NoProvidersAvailable) => {
            eprintln!("No providers configured");
        }
        Err(RoutingError::AllProvidersFailed) => {
            eprintln!("All providers failed, check system health");
        }
        Err(RoutingError::CircuitBreakerOpen(provider)) => {
            eprintln!("Provider {} is temporarily unavailable", provider);
        }
        Err(e) => eprintln!("Routing error: {}", e),
    }
}
```

### Complete Example with Failover

```rust
use llm_edge_routing::{
    RoutingStrategy,
    RoutingDecision,
    circuit_breaker::{CircuitBreaker, CircuitState},
    RoutingError,
};
use std::time::Duration;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Setup routing strategy
    let strategy = RoutingStrategy::Hybrid {
        cost_weight: 0.3,
        latency_weight: 0.5,
        reliability_weight: 0.2,
    };

    // Setup circuit breakers for each provider
    let mut circuit_breakers = HashMap::new();
    circuit_breakers.insert(
        "openai".to_string(),
        CircuitBreaker::new(3, Duration::from_secs(30))
    );
    circuit_breakers.insert(
        "anthropic".to_string(),
        CircuitBreaker::new(3, Duration::from_secs(30))
    );

    // Provider priority list for failover
    let providers = vec!["openai", "anthropic"];

    // Attempt routing with fallback
    for provider in &providers {
        if let Some(cb) = circuit_breakers.get(*provider) {
            match cb.state() {
                CircuitState::Closed | CircuitState::HalfOpen => {
                    println!("Attempting provider: {}", provider);
                    // Make actual request here
                    // On success: cb.record_success()
                    // On failure: cb.record_failure() and continue to next provider
                    break;
                }
                CircuitState::Open => {
                    println!("Provider {} circuit open, trying fallback", provider);
                    continue;
                }
            }
        }
    }
}
```

## Integration with LLM Edge Agent

This crate is designed to work seamlessly with the LLM Edge Agent ecosystem:

```rust
use llm_edge_routing::RoutingStrategy;
use llm_edge_providers::ProviderConfig;

// Configure providers
let provider_configs = vec![
    ProviderConfig::openai("your-api-key"),
    ProviderConfig::anthropic("your-api-key"),
];

// Setup intelligent routing
let routing_strategy = RoutingStrategy::default_hybrid();

// The routing engine will automatically select the best provider
// based on current costs, latency, and reliability metrics
```

## Performance Characteristics

- **Routing Decision**: O(n) where n is the number of providers (typically < 10)
- **Circuit Breaker State Check**: O(1) atomic operations
- **Memory Footprint**: Minimal - approximately 100 bytes per circuit breaker
- **Concurrency**: Lock-free operations for circuit breaker state checks

## Observability

The routing engine integrates with standard Rust observability tools:

```rust
// Tracing integration
tracing::info!(
    provider = %decision.provider_name,
    score = decision.score,
    "Routing decision made"
);

// Metrics integration (via the metrics crate)
metrics::counter!("routing.decisions", 1, "strategy" => "hybrid");
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/CONTRIBUTING.md) for details.

## Related Crates

- [`llm-edge-providers`](../llm-edge-providers) - Provider abstraction layer
- [`llm-edge-core`](../llm-edge-core) - Core types and utilities
- [`llm-edge-protocol`](../llm-edge-protocol) - Protocol definitions

## Repository

https://github.com/globalbusinessadvisors/llm-edge-agent
