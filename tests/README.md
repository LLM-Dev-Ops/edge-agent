# Integration Test Suite Documentation

## Overview

This directory contains comprehensive end-to-end integration tests for the LLM Edge Agent project. The test suite validates the complete system functionality, including request flows, routing strategies, provider integrations, caching behavior, and observability features.

## Test Structure

```
tests/
├── integration_tests.rs       # Main integration test suite (39 tests)
├── cache_tests.rs             # Cache-specific tests (existing)
├── helpers/                   # Test utilities and helpers
│   ├── mod.rs                # Helper module exports
│   ├── server.rs             # Test server implementation
│   ├── config.rs             # Test configuration builder
│   ├── metrics.rs            # Test metrics tracking
│   └── fixtures.rs           # Sample data and fixtures
└── mocks/                     # Mock implementations
    ├── mod.rs                # Mock module exports
    ├── provider.rs           # Mock LLM providers
    ├── cache.rs              # Mock cache implementations
    └── router.rs             # Mock routing engine
```

## Test Categories

### 1. Smoke Tests (5 tests)
Basic functionality validation to ensure the system is operational.

- **smoke_test_server_starts_successfully**: Validates server initialization
- **smoke_test_health_endpoint_responds**: Checks health check endpoint
- **smoke_test_metrics_endpoint_works**: Verifies Prometheus metrics
- **smoke_test_basic_auth_works**: Tests authentication flow
- **smoke_test_basic_request_succeeds**: Validates basic LLM request

### 2. Request Flow Tests (10 tests)
Complete request lifecycle validation.

- **flow_test_complete_with_cache_miss**: Auth → Cache miss → Route → Provider → Cache write → Response
- **flow_test_complete_with_l1_cache_hit**: Auth → L1 cache hit → Response (no provider call)
- **flow_test_complete_with_l2_cache_hit**: Auth → L2 cache hit → L1 promotion → Response
- **flow_test_request_with_invalid_auth**: Tests authentication failure
- **flow_test_request_with_rate_limit_exceeded**: Tests rate limiting (429 response)
- **flow_test_request_timeout_handling**: Tests timeout behavior (408 response)
- **flow_test_large_request_handling**: Tests handling of ~10MB requests
- **flow_test_concurrent_requests**: Tests 100 parallel requests (thread safety)
- **flow_test_streaming_response**: Tests SSE streaming responses
- **flow_test_graceful_degradation**: Tests L2 failure with L1 fallback

### 3. Routing Tests (8 tests)
Validates all routing strategies and circuit breaker behavior.

- **routing_test_round_robin_distributes_evenly**: Tests round-robin distribution
- **routing_test_failover_uses_priority**: Tests priority-based routing
- **routing_test_least_latency_selects_fastest**: Tests latency-based routing
- **routing_test_cost_optimized_selects_cheapest**: Tests cost-based routing
- **routing_test_circuit_breaker_opens_on_failures**: Tests circuit breaker opening
- **routing_test_circuit_breaker_recovers**: Tests circuit breaker recovery
- **routing_test_provider_health_affects_routing**: Tests health-aware routing
- **routing_test_fallback_chain_works**: Tests fallback provider chain

### 4. Provider Tests (6 tests)
Tests LLM provider integrations and error handling.

- **provider_test_openai_request_succeeds**: Tests OpenAI integration
- **provider_test_anthropic_request_succeeds**: Tests Anthropic integration
- **provider_test_retry_on_failure**: Tests retry policy (3 retries)
- **provider_test_timeout_handling**: Tests provider timeout
- **provider_test_invalid_model_handling**: Tests invalid model error
- **provider_test_cost_tracking_works**: Tests cost calculation accuracy

### 5. Cache Tests (5 tests)
Validates multi-tier caching system.

- **cache_test_l1_stores_and_retrieves**: Tests L1 in-memory cache
- **cache_test_l2_stores_and_retrieves**: Tests L2 distributed cache
- **cache_test_invalidation_works**: Tests cache invalidation
- **cache_test_ttl_expiration**: Tests TTL-based expiration
- **cache_test_graceful_l2_degradation**: Tests L2 failure handling

### 6. Observability Tests (5 tests)
Validates metrics, tracing, and logging.

- **observability_test_metrics_recorded_correctly**: Tests Prometheus metrics
- **observability_test_traces_created_for_requests**: Tests OpenTelemetry traces
- **observability_test_pii_redacted_in_logs**: Tests PII redaction
- **observability_test_error_logs_captured**: Tests error logging
- **observability_test_cost_tracking_accurate**: Tests cost tracking accuracy

## Running Tests

### Run All Integration Tests
```bash
cargo test --test integration_tests
```

### Run Specific Test Category
```bash
# Smoke tests only
cargo test --test integration_tests smoke_test

# Request flow tests
cargo test --test integration_tests flow_test

# Routing tests
cargo test --test integration_tests routing_test

# Provider tests
cargo test --test integration_tests provider_test

# Cache tests
cargo test --test integration_tests cache_test

# Observability tests
cargo test --test integration_tests observability_test
```

### Run Tests with Redis (L2 Cache)
```bash
# Start Redis
docker run -d -p 6379:6379 redis:7-alpine

# Run tests including Redis-dependent tests
REDIS_URL=redis://localhost:6379 cargo test --test integration_tests -- --include-ignored
```

### Run with Coverage
```bash
# Install coverage tools
cargo install cargo-llvm-cov

# Run tests with coverage
cargo llvm-cov --test integration_tests --html

# Open coverage report
open target/llvm-cov/html/index.html
```

### Run with Nextest (Faster)
```bash
# Install nextest
cargo install cargo-nextest

# Run tests
cargo nextest run --test integration_tests
```

## Test Configuration

### Environment Variables
- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`)
- `LOG_LEVEL`: Log level for tests (default: `info`)
- `TEST_TIMEOUT`: Test timeout in seconds (default: `30`)

### Test Fixtures
The `helpers/fixtures.rs` module provides pre-built test data:

```rust
use crate::helpers::fixtures::*;

// Sample requests
let simple_req = requests::simple();
let streaming_req = requests::streaming();
let pii_req = requests::with_pii();

// Sample responses
let simple_resp = responses::simple();
let anthropic_resp = responses::from_anthropic();

// Sample errors
let auth_error = errors::unauthorized();
let rate_limit_error = errors::rate_limited();
```

## Mock Infrastructure

### Mock Providers
```rust
use crate::mocks::*;

// Create mock provider
let mock = MockProvider::new();
mock.expect_chat_completion()
    .returning(|_| Ok(create_test_response("Mock response")));

// Mock OpenAI
let openai = MockOpenAI::new()
    .with_response(responses::simple())
    .with_latency(Duration::from_millis(100));
```

### Mock Cache
```rust
// Mock L1 cache
let l1 = MockL1Cache::new();
l1.set("key", response, Some(Duration::from_secs(300))).await;

// Mock L2 cache with failure mode
let l2 = MockL2Cache::new();
l2.set_failure_mode(true); // Simulate Redis failure
```

### Mock Routing
```rust
// Mock routing engine
let router = MockRoutingEngine::new(RoutingStrategy::LeastLatency);
router.set_provider_latency("openai", Duration::from_millis(50)).await;

// Mock circuit breaker
let cb = MockCircuitBreaker::new(5); // 5 failure threshold
cb.trip().await; // Open circuit
```

## Test Helpers

### Test Server
```rust
// Create test server with default config
let server = TestServer::new(TestConfig::default()).await;

// Make requests
let response = server.post("/v1/chat/completions")
    .json(&request)
    .send()
    .await;

// Get metrics
let metrics = server.metrics().snapshot();
assert_eq!(metrics.cache_hits, 1);

// Shutdown
server.shutdown().await;
```

### Test Configuration
```rust
// Build custom test config
let config = TestConfig::default()
    .with_auth()
    .with_rate_limit(100, Duration::from_secs(60))
    .with_routing_strategy(RoutingStrategy::CostOptimized)
    .with_circuit_breaker(5, Duration::from_secs(30))
    .with_retry_policy(3, Duration::from_millis(100));

let server = TestServer::new(config).await;
```

## Performance Targets

### Latency Requirements
- **L1 Cache Lookup**: <1ms (sub-millisecond)
- **L2 Cache Lookup**: <5ms
- **Provider Request**: <500ms (P95)
- **Complete Request Flow**: <600ms (P95)

### Throughput Requirements
- **Concurrent Requests**: 100+ simultaneous
- **Requests per Second**: 1000+ (with 70% cache hit rate)

### Resource Limits
- **Memory Usage**: <500MB under load
- **CPU Usage**: <80% under load
- **Connection Pool**: 100+ concurrent connections

## CI/CD Integration

Tests run automatically on:
- Push to `main` or `develop` branches
- Pull requests
- Manual workflow dispatch

### Coverage Requirements
- **Minimum Coverage**: 80%
- **Target Coverage**: 85%+
- **Integration Coverage**: >80%

### Test Matrix
- OS: Ubuntu Latest
- Rust: Stable
- Redis: 7.x Alpine

## Troubleshooting

### Common Issues

#### Redis Connection Failed
```bash
# Check Redis is running
redis-cli ping

# Start Redis if needed
docker run -d -p 6379:6379 redis:7-alpine
```

#### Tests Timeout
```bash
# Increase timeout
TEST_TIMEOUT=60 cargo test --test integration_tests
```

#### Mock Provider Errors
```rust
// Ensure mocks are properly configured
let mock = MockProvider::new();
mock.expect_chat_completion()
    .times(1) // Specify expected calls
    .returning(|_| Ok(response));
```

## Best Practices

### Writing New Tests
1. Use descriptive test names (e.g., `test_category_what_it_does`)
2. Test one thing per test function
3. Use test fixtures for common data
4. Clean up resources in test shutdown
5. Use appropriate assertions (assert_eq!, assert!, assert_matches!)

### Test Organization
1. Group related tests by category prefix
2. Use helper functions for repetitive setup
3. Keep tests independent and isolated
4. Avoid test interdependencies

### Performance Testing
1. Use `tokio::time::Instant` for timing
2. Assert performance targets explicitly
3. Test under realistic load conditions
4. Profile slow tests

## Metrics & Reporting

### Coverage Report
```bash
cargo llvm-cov report --summary-only
```

### Test Results
Test results are exported to JUnit XML for CI integration:
```
target/nextest/default/junit.xml
```

### Benchmark Results
Benchmark results are stored in:
```
target/criterion/
```

## Contributing

When adding new tests:
1. Follow the existing test structure
2. Add appropriate documentation
3. Update this README if adding new categories
4. Ensure tests pass in CI
5. Maintain >80% coverage

## License

Apache 2.0 - See LICENSE file for details
