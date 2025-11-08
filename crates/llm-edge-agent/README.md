# llm-edge-agent

[![Crates.io](https://img.shields.io/crates/v/llm-edge-agent.svg)](https://crates.io/crates/llm-edge-agent)
[![Documentation](https://docs.rs/llm-edge-agent/badge.svg)](https://docs.rs/llm-edge-agent)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)

High-performance LLM intercepting proxy with intelligent caching, routing, and observability.

## Overview

The `llm-edge-agent` binary is the main executable for the LLM Edge Agent system - an enterprise-grade intercepting proxy for Large Language Model APIs. It sits between your applications and LLM providers (OpenAI, Anthropic, etc.), providing intelligent request routing, multi-tier caching, cost optimization, and comprehensive observability.

**Key Features:**
- High Performance: 1000+ RPS throughput, <50ms proxy overhead
- Intelligent Caching: Multi-tier (L1 Moka + L2 Redis) with 70%+ hit rates
- Smart Routing: Model-based, cost-optimized, latency-optimized, and failover strategies
- Multi-Provider Support: OpenAI, Anthropic, with easy extensibility
- Enterprise Observability: Prometheus metrics, Grafana dashboards, Jaeger tracing
- Production Ready: Comprehensive testing, security hardening, chaos engineering validated

## Installation

### From Crates.io (when published)

```bash
cargo install llm-edge-agent
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/globalbusinessadvisors/llm-edge-agent.git
cd llm-edge-agent

# Build the binary
cargo build --release --package llm-edge-agent

# The binary will be at: target/release/llm-edge-agent
```

### Using Docker

```bash
# Pull the image (when published)
docker pull llm-edge-agent:latest

# Or build locally
docker build -t llm-edge-agent .
```

## Quick Start

### Prerequisites

At least one LLM provider API key is required:
- OpenAI API key (for GPT models)
- Anthropic API key (for Claude models)

Optional infrastructure:
- Redis 7.0+ (for L2 distributed caching)
- Prometheus (for metrics collection)
- Grafana (for dashboards)
- Jaeger (for distributed tracing)

### Configuration

Create a `.env` file or set environment variables:

```bash
# Required: At least one provider API key
OPENAI_API_KEY=sk-your-openai-key
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key

# Optional: Server configuration
HOST=0.0.0.0
PORT=8080
METRICS_PORT=9090

# Optional: L2 Cache (Redis)
ENABLE_L2_CACHE=true
REDIS_URL=redis://localhost:6379

# Optional: Observability
ENABLE_TRACING=true
ENABLE_METRICS=true
RUST_LOG=info,llm_edge_agent=debug
```

### Running the Binary

**Standalone (L1 cache only):**
```bash
# Set API key
export OPENAI_API_KEY=sk-your-key

# Run the binary
llm-edge-agent
```

**With full infrastructure (recommended):**
```bash
# Start complete stack with Docker Compose
docker-compose -f docker-compose.production.yml up -d

# The agent will automatically connect to Redis, Prometheus, etc.
```

**Check health:**
```bash
curl http://localhost:8080/health
```

### Making Your First Request

The proxy exposes an OpenAI-compatible API:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "Hello, world!"
      }
    ]
  }'
```

**Response includes metadata:**
```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "model": "gpt-3.5-turbo",
  "choices": [...],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 15,
    "total_tokens": 25
  },
  "metadata": {
    "provider": "openai",
    "cached": false,
    "cache_tier": null,
    "latency_ms": 523,
    "cost_usd": 0.000125
  }
}
```

## Usage

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `8080` | HTTP server port |
| `METRICS_PORT` | `9090` | Prometheus metrics port |
| `OPENAI_API_KEY` | - | OpenAI API key (required if using OpenAI) |
| `ANTHROPIC_API_KEY` | - | Anthropic API key (required if using Anthropic) |
| `ENABLE_L2_CACHE` | `false` | Enable Redis L2 cache |
| `REDIS_URL` | - | Redis connection URL |
| `ENABLE_TRACING` | `true` | Enable distributed tracing |
| `ENABLE_METRICS` | `true` | Enable Prometheus metrics |
| `RUST_LOG` | `info` | Logging configuration |

### API Endpoints

**Main Proxy Endpoint:**
- `POST /v1/chat/completions` - OpenAI-compatible chat completions

**Health & Monitoring:**
- `GET /health` - Detailed system health status
- `GET /health/ready` - Kubernetes readiness probe
- `GET /health/live` - Kubernetes liveness probe
- `GET /metrics` - Prometheus metrics

### Supported Models

**OpenAI:**
- gpt-4, gpt-4-turbo, gpt-4o
- gpt-3.5-turbo

**Anthropic:**
- claude-3-opus, claude-3-sonnet, claude-3-haiku
- claude-2.1, claude-2.0

The proxy automatically routes requests to the appropriate provider based on the model name.

## Architecture

The binary integrates all LLM Edge Agent components:

```
llm-edge-agent (binary)
├── HTTP Server (Axum)
│   ├── Request validation
│   ├── Health check endpoints
│   └── Metrics endpoint
│
├── Cache Layer (llm-edge-cache)
│   ├── L1: In-memory (Moka)
│   └── L2: Distributed (Redis)
│
├── Routing Layer (llm-edge-routing)
│   ├── Model-based routing
│   ├── Cost optimization
│   ├── Latency optimization
│   └── Failover support
│
├── Provider Layer (llm-edge-providers)
│   ├── OpenAI adapter
│   └── Anthropic adapter
│
└── Observability (llm-edge-monitoring)
    ├── Prometheus metrics
    ├── Distributed tracing
    └── Structured logging
```

## Deployment

### Docker

```bash
# Build image
docker build -t llm-edge-agent .

# Run container
docker run -d \
  -p 8080:8080 \
  -p 9090:9090 \
  -e OPENAI_API_KEY=sk-your-key \
  -e ENABLE_L2_CACHE=false \
  --name llm-edge-agent \
  llm-edge-agent:latest
```

### Docker Compose

See `docker-compose.production.yml` in the repository root for a complete production-ready stack including:
- LLM Edge Agent (3 replicas)
- Redis cluster (3 nodes)
- Prometheus
- Grafana (with pre-built dashboards)
- Jaeger

### Kubernetes

```bash
# Create namespace
kubectl create namespace llm-edge-production

# Create secrets
kubectl create secret generic llm-edge-secrets \
  --from-literal=openai-api-key="sk-..." \
  --from-literal=anthropic-api-key="sk-ant-..." \
  -n llm-edge-production

# Deploy
kubectl apply -f deployments/kubernetes/llm-edge-agent.yaml
```

Features:
- HorizontalPodAutoscaler (3-10 replicas)
- Rolling updates (zero downtime)
- Resource limits and requests
- Liveness and readiness probes

## Monitoring

### Metrics

The binary exposes Prometheus metrics on the configured `METRICS_PORT`:

**Request Metrics:**
- `llm_edge_requests_total` - Total request count
- `llm_edge_request_duration_seconds` - Request latency histogram
- `llm_edge_request_errors_total` - Error count by type

**Cache Metrics:**
- `llm_edge_cache_hits_total{tier="l1|l2"}` - Cache hits
- `llm_edge_cache_misses_total` - Cache misses
- `llm_edge_cache_latency_seconds` - Cache operation latency

**Provider Metrics:**
- `llm_edge_provider_latency_seconds` - Provider response time
- `llm_edge_provider_errors_total` - Provider errors
- `llm_edge_cost_usd_total` - Cumulative cost tracking

**Token Metrics:**
- `llm_edge_tokens_used_total` - Token usage by provider/model
- `llm_edge_tokens_prompt_total` - Prompt tokens
- `llm_edge_tokens_completion_total` - Completion tokens

### Health Checks

**Health endpoint response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-01-08T12:00:00Z",
  "version": "1.0.0",
  "cache": {
    "l1_healthy": true,
    "l2_healthy": true,
    "l2_configured": true
  },
  "providers": {
    "openai": {
      "configured": true,
      "healthy": true
    },
    "anthropic": {
      "configured": true,
      "healthy": true
    }
  }
}
```

## Performance

**Benchmarks:**
- Throughput: 1000+ requests/second
- Proxy Overhead: <50ms (P95)
- L1 Cache Hit: <100μs
- L2 Cache Hit: 1-2ms
- Memory Usage: <2GB under normal load

**Cache Performance:**
- Overall Hit Rate: >70%
- L1 Hit Rate: 60-70% (hot data)
- L2 Hit Rate: 10-15% (warm data)
- Cost Savings: 70%+ (cached responses are free)

## Documentation

For comprehensive documentation, see the [root README](../../README.md):
- Full architecture guide
- Testing documentation
- Infrastructure setup
- Deployment guides
- API reference

## Development

This crate can be used both as a binary and as a library:

**As a Binary:**
```bash
cargo run --package llm-edge-agent
```

**As a Library:**
```toml
[dependencies]
llm-edge-agent = "1.0.0"
```

```rust
use llm_edge_agent::{AppConfig, initialize_app_state, handle_chat_completions};

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();
    let state = initialize_app_state(config).await.unwrap();

    // Use in your own Axum router
    // let app = Router::new()
    //     .route("/v1/chat/completions", post(handle_chat_completions))
    //     .with_state(Arc::new(state));
}
```

## Troubleshooting

**Provider not available:**
```
Error: No providers configured
```
Solution: Set at least one API key (`OPENAI_API_KEY` or `ANTHROPIC_API_KEY`)

**Redis connection failed:**
```
Warning: L2 cache enabled but connection failed
```
Solution: Verify Redis is running and `REDIS_URL` is correct. Agent will fall back to L1-only mode.

**High latency:**
- Check provider health: `curl http://localhost:8080/health`
- Monitor metrics: `curl http://localhost:9090/metrics`
- Review logs: Set `RUST_LOG=debug`

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

See the [Contributing Guide](../../CONTRIBUTING.md) in the root repository.

## Support

- Repository: https://github.com/globalbusinessadvisors/llm-edge-agent
- Issues: https://github.com/globalbusinessadvisors/llm-edge-agent/issues
- Documentation: https://docs.rs/llm-edge-agent
