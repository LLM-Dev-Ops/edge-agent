# Integration Layer - Quick Start Guide

## Overview

This guide helps you get started with the integrated LLM Edge Agent system.

## Prerequisites

- Rust 1.75+ installed
- Redis server (optional, for L2 cache)
- OpenAI or Anthropic API key

## Quick Start

### 1. Set Environment Variables

```bash
# Required: At least one provider API key
export OPENAI_API_KEY="sk-..."
# OR
export ANTHROPIC_API_KEY="sk-ant-..."

# Optional: Enable L2 cache
export ENABLE_L2_CACHE=true
export REDIS_URL="redis://localhost:6379"

# Optional: Server configuration
export PORT=8080
export HOST=0.0.0.0
```

### 2. Start Redis (Optional)

If you want to enable L2 cache:

```bash
# Using Docker
docker run -d -p 6379:6379 redis:7-alpine

# Or using local Redis
redis-server
```

### 3. Build and Run

```bash
# Build the project
cargo build --release

# Run the server
cargo run --package llm-edge-agent --release
```

### 4. Test the Integration

```bash
# Health check
curl http://localhost:8080/health | jq

# Send a test request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Hello, world!"}
    ],
    "temperature": 0.7,
    "max_tokens": 100
  }' | jq
```

## Expected Output

### Startup Logs

```
2025-11-08T22:00:00.000Z INFO  llm_edge_agent Starting LLM Edge Agent v0.1.0
2025-11-08T22:00:00.001Z INFO  llm_edge_agent Loading configuration
2025-11-08T22:00:00.002Z INFO  llm_edge_agent Configuration loaded: host=0.0.0.0, port=8080, l2_cache_enabled=true
2025-11-08T22:00:00.003Z INFO  llm_edge_agent Initializing application state
2025-11-08T22:00:00.004Z INFO  llm_edge_cache Initializing cache layer
2025-11-08T22:00:00.005Z INFO  llm_edge_cache L2 cache enabled with Redis: redis://localhost:6379
2025-11-08T22:00:00.010Z INFO  llm_edge_agent Initializing provider adapters
2025-11-08T22:00:00.011Z INFO  llm_edge_agent Initializing OpenAI provider
2025-11-08T22:00:00.012Z INFO  llm_edge_agent Application state initialized successfully
2025-11-08T22:00:00.013Z INFO  llm_edge_agent Health check: status=healthy, cache_l1=true, cache_l2=true, openai=true, anthropic=false
2025-11-08T22:00:00.014Z INFO  llm_edge_agent Starting HTTP server on 0.0.0.0:8080
2025-11-08T22:00:00.015Z INFO  llm_edge_agent LLM Edge Agent is ready to accept requests!
```

### Health Check Response

```json
{
  "status": "healthy",
  "timestamp": "2025-11-08T22:00:01.000Z",
  "version": "0.1.0",
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
      "configured": false,
      "healthy": false
    }
  }
}
```

### Chat Completion Response

```json
{
  "id": "chatcmpl-7a8b9c0d1e2f3g4h",
  "object": "chat.completion",
  "created": 1699488001,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I assist you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 9,
    "total_tokens": 19
  },
  "metadata": {
    "provider": "openai",
    "cached": false,
    "cache_tier": null,
    "latency_ms": 1234,
    "cost_usd": 0.00057
  }
}
```

## Verifying Cache Behavior

### First Request (Cache Miss)

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }' | jq '.metadata'
```

Expected:
```json
{
  "provider": "openai",
  "cached": false,
  "cache_tier": null,
  "latency_ms": 1500
}
```

### Second Request (Cache Hit)

Same request again:

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }' | jq '.metadata'
```

Expected:
```json
{
  "provider": "cache",
  "cached": true,
  "cache_tier": "l1",
  "latency_ms": 1,
  "cost_usd": 0.0
}
```

## Monitoring

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

### Request Logs

Enable debug logging:

```bash
export RUST_LOG=llm_edge_agent=debug,llm_edge_cache=debug
cargo run --package llm-edge-agent --release
```

## Troubleshooting

### No Providers Available

**Error:**
```
Failed to initialize application state: No LLM providers configured
```

**Solution:**
Set at least one provider API key:
```bash
export OPENAI_API_KEY="sk-..."
```

### Redis Connection Failed

**Logs:**
```
WARN llm_edge_cache L2 cache enabled but no Redis URL provided, using L1 only
```

**Solution:**
Set Redis URL:
```bash
export REDIS_URL="redis://localhost:6379"
```

### Provider Request Failed

**Logs:**
```
ERROR llm_edge_agent Provider request failed: Connection timeout
```

**Solutions:**
1. Check internet connectivity
2. Verify API key is valid
3. Check provider status page

## Integration Points Verification

### 1. Cache Integration

```bash
# Test L1 cache
curl http://localhost:8080/v1/chat/completions -X POST -H "Content-Type: application/json" -d '{"model":"gpt-4","messages":[{"role":"user","content":"Test"}]}'

# Check cache hit (same request)
curl http://localhost:8080/v1/chat/completions -X POST -H "Content-Type: application/json" -d '{"model":"gpt-4","messages":[{"role":"user","content":"Test"}]}'
```

### 2. Provider Integration

```bash
# Test OpenAI provider
curl http://localhost:8080/v1/chat/completions -X POST -H "Content-Type: application/json" -d '{"model":"gpt-4","messages":[{"role":"user","content":"Test"}]}'

# Test Anthropic provider (if configured)
curl http://localhost:8080/v1/chat/completions -X POST -H "Content-Type: application/json" -d '{"model":"claude-3-5-sonnet-20240229","messages":[{"role":"user","content":"Test"}]}'
```

### 3. Observability Integration

```bash
# Check metrics
curl http://localhost:8080/metrics

# Check health
curl http://localhost:8080/health

# Check readiness
curl http://localhost:8080/health/ready

# Check liveness
curl http://localhost:8080/health/live
```

## Performance Testing

See [PERFORMANCE_TESTING.md](./PERFORMANCE_TESTING.md) for detailed performance testing instructions.

## Next Steps

1. **Configure Multiple Providers** - Set up both OpenAI and Anthropic
2. **Enable L2 Cache** - Set up Redis for distributed caching
3. **Load Testing** - Run performance benchmarks
4. **Production Deployment** - Deploy to Kubernetes/Docker

## Additional Resources

- [Integration Documentation](./INTEGRATION.md) - Detailed architecture
- [API Reference](./API_REFERENCE.md) - API endpoints and schemas
- [Performance Testing](./PERFORMANCE_TESTING.md) - Benchmarking guide
