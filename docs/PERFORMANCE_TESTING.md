# Performance Testing Guide

## Overview

This guide provides instructions for measuring and validating the performance of the LLM Edge Agent integration.

## Performance Targets

### MVP Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Total Overhead | <20ms | End-to-end - Provider latency |
| L1 Cache Hit Latency | <1ms | Request timestamp - Response timestamp |
| L2 Cache Hit Latency | 1-2ms | Request timestamp - Response timestamp |
| Cache Hit Rate | >50% | Hits / Total Requests |
| Request Throughput | >1000 req/s | Load testing |
| Error Rate | <1% | Failed requests / Total requests |

## Test Scenarios

### 1. Cache Performance Test

#### L1 Cache Hit Performance

```bash
# Script: test_l1_cache.sh

# First request (cache miss)
time curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }' > /dev/null 2>&1

# Second request (L1 cache hit)
for i in {1..100}; do
  time curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "gpt-4",
      "messages": [{"role": "user", "content": "What is 2+2?"}]
    }' > /dev/null 2>&1
done
```

**Expected Results:**
- First request: 500-2000ms (provider latency)
- Subsequent requests: <1ms (L1 cache hit)

#### L2 Cache Hit Performance

```bash
# Script: test_l2_cache.sh

# Clear L1 cache (restart server or wait for TTL expiration)
# Request should hit L2 cache

for i in {1..100}; do
  # Vary the request slightly to avoid L1 cache
  time curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d "{
      \"model\": \"gpt-4\",
      \"messages\": [{\"role\": \"user\", \"content\": \"Request $i\"}]
    }" > /dev/null 2>&1
done
```

### 2. Throughput Test

Using Apache Bench (ab):

```bash
# Install ab
sudo apt-get install apache2-utils

# Prepare request body
cat > request.json <<EOF
{
  "model": "gpt-4",
  "messages": [{"role": "user", "content": "Hello"}]
}
EOF

# Run throughput test (1000 requests, 10 concurrent)
ab -n 1000 -c 10 -p request.json -T application/json \
  http://localhost:8080/v1/chat/completions
```

**Expected Results:**
```
Requests per second:    1200 [#/sec] (mean)
Time per request:       8.333 [ms] (mean)
Time per request:       0.833 [ms] (mean, across all concurrent requests)
```

### 3. Load Testing with k6

```javascript
// load_test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  stages: [
    { duration: '30s', target: 50 },  // Ramp up to 50 users
    { duration: '1m', target: 50 },   // Stay at 50 users
    { duration: '30s', target: 100 }, // Ramp up to 100 users
    { duration: '1m', target: 100 },  // Stay at 100 users
    { duration: '30s', target: 0 },   // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<100'], // 95% of requests under 100ms
    http_req_failed: ['rate<0.01'],   // Less than 1% error rate
  },
};

export default function() {
  const payload = JSON.stringify({
    model: 'gpt-4',
    messages: [
      { role: 'user', content: 'Hello, world!' }
    ],
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  let response = http.post('http://localhost:8080/v1/chat/completions', payload, params);

  check(response, {
    'status is 200': (r) => r.status === 200,
    'response time < 100ms': (r) => r.timings.duration < 100,
    'has metadata': (r) => JSON.parse(r.body).metadata !== undefined,
  });

  sleep(1);
}
```

Run the test:
```bash
k6 run load_test.js
```

### 4. Latency Breakdown Test

```bash
# Script: latency_breakdown.sh

# Measure individual components
echo "Testing latency breakdown..."

# 1. Cache hit latency
echo "=== L1 Cache Hit ==="
for i in {1..10}; do
  curl -w "Time: %{time_total}s\n" -o /dev/null -s \
    -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{"model":"gpt-4","messages":[{"role":"user","content":"Test"}]}'
done

# 2. Provider latency (cache disabled)
echo "=== Provider Request (no cache) ==="
for i in {1..10}; do
  curl -w "Time: %{time_total}s\n" -o /dev/null -s \
    -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d "{\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Unique request $RANDOM\"}]}"
done
```

## Metrics Collection

### 1. Prometheus Metrics

Query Prometheus metrics:

```bash
# Cache hit rate
curl -s http://localhost:8080/metrics | grep llm_edge_cache_hits_total
curl -s http://localhost:8080/metrics | grep llm_edge_cache_misses_total

# Request latency
curl -s http://localhost:8080/metrics | grep llm_edge_request_duration_ms

# Token usage
curl -s http://localhost:8080/metrics | grep llm_edge_tokens_total

# Cost
curl -s http://localhost:8080/metrics | grep llm_edge_cost_usd_total
```

### 2. Application Logs

Enable detailed logging:

```bash
export RUST_LOG=llm_edge_agent=debug,llm_edge_cache=debug,llm_edge_providers=debug
cargo run --release
```

Extract latency from logs:

```bash
# Extract request latencies
grep "Request completed" logs.txt | grep -oP 'total_latency_ms=\K\d+'

# Calculate average
grep "Request completed" logs.txt | grep -oP 'total_latency_ms=\K\d+' | \
  awk '{sum+=$1; count++} END {print "Average:", sum/count, "ms"}'
```

## Performance Benchmarking

### Benchmark Suite

Create a comprehensive benchmark:

```bash
#!/bin/bash
# benchmark_suite.sh

echo "LLM Edge Agent Performance Benchmark Suite"
echo "=========================================="
echo

# 1. Cache Performance
echo "1. Cache Performance"
echo "-------------------"

# L1 Cache Hit
echo -n "L1 Cache Hit (avg of 100 requests): "
time_sum=0
for i in {1..100}; do
  time=$(curl -w "%{time_total}" -o /dev/null -s \
    -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{"model":"gpt-4","messages":[{"role":"user","content":"Test"}]}')
  time_sum=$(echo "$time_sum + $time" | bc)
done
avg=$(echo "scale=3; $time_sum / 100" | bc)
echo "${avg}s"

# 2. Provider Latency
echo
echo "2. Provider Latency"
echo "-------------------"

# OpenAI
echo -n "OpenAI (avg of 10 requests): "
time_sum=0
for i in {1..10}; do
  time=$(curl -w "%{time_total}" -o /dev/null -s \
    -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d "{\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Request $RANDOM\"}]}")
  time_sum=$(echo "$time_sum + $time" | bc)
done
avg=$(echo "scale=3; $time_sum / 10" | bc)
echo "${avg}s"

# 3. Throughput
echo
echo "3. Throughput Test"
echo "------------------"
ab -n 1000 -c 10 -p request.json -T application/json \
  http://localhost:8080/v1/chat/completions | \
  grep "Requests per second"

echo
echo "Benchmark complete!"
```

### Expected Results

```
LLM Edge Agent Performance Benchmark Suite
==========================================

1. Cache Performance
-------------------
L1 Cache Hit (avg of 100 requests): 0.001s

2. Provider Latency
-------------------
OpenAI (avg of 10 requests): 1.234s

3. Throughput Test
------------------
Requests per second:    1200.00 [#/sec] (mean)

Benchmark complete!
```

## Overhead Calculation

Calculate proxy overhead:

```
Overhead = Total Latency - Provider Latency

Example:
- Total Latency: 1250ms
- Provider Latency: 1234ms
- Overhead: 16ms ✓ (within 20ms target)
```

## Continuous Performance Monitoring

### Setup Grafana Dashboard

1. Install Prometheus and Grafana
2. Configure Prometheus to scrape metrics from `:9090/metrics`
3. Import dashboard template

### Key Metrics to Monitor

1. **Request Latency (p50, p95, p99)**
   - Target: p95 < 100ms for cache hits

2. **Cache Hit Rate**
   - Target: >50% (MVP), >70% (Production)

3. **Provider Latency**
   - Track by provider (OpenAI, Anthropic)

4. **Error Rate**
   - Target: <1%

5. **Throughput**
   - Requests per second

6. **Cost per Request**
   - Track actual vs. cached requests

## Performance Regression Testing

### Automated Performance Tests

Create a CI/CD pipeline step:

```yaml
# .github/workflows/performance.yml
name: Performance Tests

on:
  pull_request:
    branches: [ main ]

jobs:
  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Start Redis
        run: docker run -d -p 6379:6379 redis:7-alpine

      - name: Build and run server
        run: |
          cargo build --release
          cargo run --release &
          sleep 5

      - name: Run performance tests
        run: |
          ./scripts/benchmark_suite.sh

      - name: Check performance thresholds
        run: |
          # Fail if overhead > 20ms
          # Fail if cache hit latency > 1ms
          ./scripts/check_performance.sh
```

## Optimization Tips

If performance targets are not met:

1. **High L1 Latency (>1ms)**
   - Check L1 cache size
   - Verify TinyLFU is working correctly
   - Profile CPU usage

2. **High L2 Latency (>2ms)**
   - Check Redis network latency
   - Verify Redis is not swapping
   - Consider Redis clustering

3. **Low Cache Hit Rate (<50%)**
   - Increase L1 cache size
   - Increase L2 TTL
   - Analyze request patterns

4. **High Overhead (>20ms)**
   - Profile request handling code
   - Check for blocking operations
   - Optimize serialization/deserialization

## Reporting

Generate a performance report:

```bash
# performance_report.sh

echo "LLM Edge Agent Performance Report"
echo "=================================="
echo "Date: $(date)"
echo
echo "Configuration:"
echo "- L1 Cache: Enabled (1000 entries, 5min TTL)"
echo "- L2 Cache: Enabled (Redis)"
echo "- Providers: OpenAI, Anthropic"
echo
echo "Results:"
echo "--------"
echo "L1 Cache Hit Latency: <measurement>ms"
echo "L2 Cache Hit Latency: <measurement>ms"
echo "Cache Hit Rate: <measurement>%"
echo "Provider Latency (OpenAI): <measurement>ms"
echo "Total Overhead: <measurement>ms"
echo "Throughput: <measurement> req/s"
echo "Error Rate: <measurement>%"
echo
echo "Status: PASS/FAIL"
```

## Conclusion

This performance testing guide provides comprehensive methods to validate the LLM Edge Agent integration meets its performance targets. Regular testing ensures the system maintains optimal performance as features are added.
