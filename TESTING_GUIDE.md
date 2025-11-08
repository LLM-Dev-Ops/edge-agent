# LLM Edge Agent - Comprehensive Testing Guide

**Version**: 1.0.0
**Date**: 2025-11-08
**Status**: Production Ready

---

## Table of Contents

1. [Overview](#overview)
2. [Test Types](#test-types)
3. [Quick Start](#quick-start)
4. [Detailed Testing Procedures](#detailed-testing-procedures)
5. [CI/CD Integration](#cicd-integration)
6. [Performance Baselines](#performance-baselines)
7. [Troubleshooting](#troubleshooting)

---

## Overview

This guide covers all testing procedures for the LLM Edge Agent project. The testing strategy includes:

- **Unit Tests**: Component-level testing
- **Integration Tests**: End-to-end flow testing
- **Load Tests**: Performance under load (k6)
- **Security Tests**: OWASP, penetration testing, dependency scanning
- **Performance Tests**: Benchmarking and profiling
- **Chaos Tests**: Resilience and failure scenarios
- **Regression Tests**: Performance regression detection

### Test Coverage Goals

- Unit Test Coverage: >80%
- Integration Test Coverage: >70%
- Critical Path Coverage: 100%
- Security Vulnerability Tolerance: 0 critical, 0 high

---

## Test Types

### 1. Unit Tests

**Location**: `*/tests/` directories in each crate
**Framework**: Rust `#[test]` and `#[tokio::test]`
**Duration**: ~2-5 minutes

```bash
# Run all unit tests
cargo test --workspace --lib

# Run tests for specific crate
cargo test --package llm-edge-cache --lib

# Run with output
cargo test --workspace --lib -- --nocapture
```

**Coverage**:
- Cache operations (L1/L2)
- Routing logic
- Provider adapters
- Authentication
- Rate limiting
- Metrics collection

### 2. Integration Tests

**Location**: `/tests/integration_tests.rs`
**Duration**: ~5-10 minutes

```bash
# Run all integration tests
cargo test --workspace --test '*'

# Run specific test category
cargo test --test integration_tests request_flow
```

**Coverage**:
- Complete request flow (cache miss)
- Complete request flow (cache hit)
- Provider failover
- Multi-tier caching
- Error handling
- Observability integration

### 3. Load Tests

**Location**: `/tests/load/`
**Tool**: [k6](https://k6.io/)
**Duration**: Varies (2-10 minutes per test)

#### Baseline Load Test

Tests normal operating conditions.

```bash
k6 run tests/load/baseline-load-test.js \
  --env BASE_URL=http://localhost:8080 \
  --env API_KEY=your-key \
  --out json=load-results.json
```

**Targets**:
- 100 RPS sustained for 10 minutes
- P95 latency < 2000ms
- Error rate < 1%
- Cache hit rate > 50%

#### Spike Test

Tests sudden traffic increases.

```bash
k6 run tests/load/spike-test.js
```

**Profile**:
- Normal: 100 VUs
- Spike: 500 VUs for 2 minutes
- Recovery: Back to 100 VUs

#### Stress Test

Identifies system limits.

```bash
k6 run tests/load/stress-test.js
```

**Profile**:
- Gradual increase: 100 → 1000 VUs
- Identifies breaking point
- Measures degradation patterns

#### Soak Test (Endurance)

Tests long-term stability.

```bash
k6 run tests/load/soak-test.js
```

**Duration**: 4 hours at 50 VUs
**Detects**: Memory leaks, performance degradation

#### Cache Effectiveness Test

Validates caching behavior.

```bash
k6 run tests/load/cache-effectiveness-test.js
```

**Validates**:
- L1 cache hit rate > 60%
- L2 cache hit rate measured
- Cached response time < 100ms
- Uncached response time < 2s

### 4. Security Tests

**Location**: `/tests/security/`
**Duration**: 10-30 minutes

#### Dependency Vulnerability Scan

```bash
./tests/security/dependency-scan.sh
```

Uses `cargo-audit` to scan for known vulnerabilities.

**Failure Criteria**: Any high/critical vulnerabilities

#### Penetration Testing

```bash
./tests/security/penetration-test.sh
```

**Tests**:
- SQL Injection prevention
- XSS prevention
- Command injection prevention
- Path traversal prevention
- Authentication bypass attempts
- Rate limiting enforcement
- Input validation
- Security headers
- SSRF prevention
- Information disclosure

**Expected**: All 10+ tests should pass

#### OWASP ZAP Scan

```bash
# Baseline scan (passive only)
SCAN_TYPE=baseline ./tests/security/owasp-zap-scan.sh

# Full scan (active + passive)
SCAN_TYPE=full ./tests/security/owasp-zap-scan.sh

# API scan (requires OpenAPI spec)
SCAN_TYPE=api OPENAPI_SPEC=docs/openapi.yaml ./tests/security/owasp-zap-scan.sh
```

**Reports**: HTML, JSON, and Markdown in `./security-reports/`

### 5. Performance Tests

**Location**: `/benches/` and `/tests/performance/`
**Framework**: Criterion.rs

#### Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark suite
cargo bench --bench cache_benchmarks
cargo bench --bench routing_benchmarks

# Compare with baseline
cargo bench --workspace -- --baseline main
```

**Benchmark Suites**:
- Cache operations (read/write, hit/miss)
- Cache key generation
- Concurrent cache access
- Routing decisions (all strategies)
- Routing scalability

#### Performance Profiling

```bash
# Generate CPU flamegraph
./tests/performance/flamegraph.sh

# Custom duration
DURATION=120 ./tests/performance/flamegraph.sh

# Output directory
OUTPUT_DIR=./my-reports ./tests/performance/flamegraph.sh
```

**Output**: SVG flamegraph for CPU hotspot analysis

#### Regression Testing

```bash
./tests/performance/regression-test.sh
```

**Threshold**: 10% performance regression allowed
**Baseline**: Stored in `./performance-reports/baseline-metrics.json`

### 6. Chaos Engineering Tests

**Location**: `/tests/chaos/`
**Duration**: 15-20 minutes
**Requires**: Docker Compose infrastructure running

```bash
./tests/chaos/chaos-engineering.sh
```

**Scenarios**:
1. Redis node failure
2. Complete cache cluster failure
3. Network latency injection (500ms)
4. Prometheus monitoring failure
5. Jaeger tracing failure
6. CPU stress (resource exhaustion)
7. Memory stress
8. Application container restart
9. Network partition
10. Clock skew

**Expected**: System should gracefully handle all failure scenarios

---

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install k6 (for load tests)
# macOS
brew install k6

# Linux (Debian/Ubuntu)
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Install Docker & Docker Compose
# https://docs.docker.com/get-docker/

# Install cargo tools
cargo install cargo-audit  # Security scanning
cargo install flamegraph   # Performance profiling
```

### Run All Tests

```bash
# Make test runner executable
chmod +x run-tests.sh

# Run complete test suite
./run-tests.sh all

# Run specific test type
./run-tests.sh unit
./run-tests.sh integration
./run-tests.sh load
./run-tests.sh security
./run-tests.sh performance
./run-tests.sh chaos
```

### Test Reports

All test reports are saved to `./test-reports/`:

```
test-reports/
├── unit/
│   └── test-output.txt
├── integration/
│   └── test-output.txt
├── load/
│   ├── baseline-results.json
│   ├── spike-results.json
│   └── cache-results.json
├── security/
│   ├── dependency-scan.txt
│   ├── penetration-test.txt
│   ├── baseline-report.html
│   └── baseline-report.json
├── performance/
│   ├── benchmark-output.txt
│   └── regression-report.txt
└── chaos/
    └── chaos-results.txt
```

---

## Detailed Testing Procedures

### Pre-Test Setup

1. **Start Infrastructure**

```bash
# Start complete stack
docker-compose -f docker-compose.production.yml up -d

# Wait for health checks
sleep 30

# Verify all services are healthy
docker-compose -f docker-compose.production.yml ps
curl http://localhost:8080/health
curl http://localhost:9091/-/healthy  # Prometheus
curl http://localhost:3000/api/health  # Grafana
```

2. **Set Environment Variables**

```bash
# API Keys (optional for testing)
export OPENAI_API_KEY=sk-your-key
export ANTHROPIC_API_KEY=sk-ant-your-key

# Test Configuration
export BASE_URL=http://localhost:8080
export API_KEY=test-key
```

### Running Individual Test Suites

#### 1. Unit Tests Only

```bash
# Fast feedback loop during development
cargo test --workspace --lib

# With coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --lib --out Html
```

#### 2. Load Tests Only

```bash
# Start infrastructure
docker-compose -f docker-compose.production.yml up -d

# Run baseline test
k6 run tests/load/baseline-load-test.js

# Run with custom parameters
k6 run tests/load/baseline-load-test.js \
  --vus 50 \
  --duration 5m \
  --env BASE_URL=http://localhost:8080

# Monitor during test
watch -n 1 'curl -s http://localhost:9090/metrics | grep llm_edge'
```

#### 3. Security Tests Only

```bash
# Quick security check
./tests/security/dependency-scan.sh
./tests/security/penetration-test.sh

# Full OWASP scan (takes longer)
SCAN_TYPE=full ./tests/security/owasp-zap-scan.sh
```

#### 4. Performance Benchmarks Only

```bash
# Run benchmarks
cargo bench --workspace

# Generate flamegraph
sudo ./tests/performance/flamegraph.sh

# Compare with baseline
cargo bench -- --baseline main --save-baseline current
```

---

## CI/CD Integration

### GitHub Actions

The project includes a comprehensive CI/CD workflow at `.github/workflows/comprehensive-tests.yml`.

**Triggers**:
- Push to `main` or `develop`
- Pull requests
- Nightly (scheduled)

**Jobs**:
1. **unit-tests**: Runs all Rust unit and integration tests
2. **security-scan**: Dependency vulnerability scanning
3. **benchmarks**: Performance benchmarking with regression detection
4. **code-quality**: Rustfmt and Clippy checks
5. **docker-build**: Docker image build test
6. **infrastructure-tests**: K8s manifest validation
7. **load-tests**: k6 load testing (main branch only)
8. **owasp-scan**: Security scanning (weekly)

**Artifacts**:
- Test results
- Benchmark reports
- Security scan reports
- Load test results

### Local CI Simulation

```bash
# Run the same checks as CI locally
./run-tests.sh all

# Individual checks
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo bench --workspace
```

---

## Performance Baselines

### Expected Performance Metrics

| Metric | Target | Baseline | Notes |
|--------|--------|----------|-------|
| **Request Throughput** | 100 RPS | 1000+ RPS | Under normal load |
| **P50 Latency** | <500ms | ~150ms | Cache hit |
| **P95 Latency** | <2000ms | ~1100ms | Overall |
| **P99 Latency** | <5000ms | ~2500ms | Overall |
| **Error Rate** | <1% | <0.1% | Under normal load |
| **Cache Hit Rate (L1)** | >50% | >70% | With repeated prompts |
| **Cached Response** | <100ms | ~50ms | L1 cache hit |
| **L1 Cache Write** | N/A | <1ms | Per operation |
| **L1 Cache Read** | N/A | <100μs | Per operation |
| **Routing Decision** | <1ms | <100μs | All strategies |

### Benchmark Results (Approximate)

```
Cache Benchmarks:
- l1_cache/write:      500-800ns
- l1_cache/read_hit:   100-200ns
- l1_cache/read_miss:  50-100ns

Routing Benchmarks:
- model_based:         50-100ns
- cost_optimized:      100-200ns
- latency_optimized:   80-150ns
- failover:            150-300ns
```

---

## Troubleshooting

### Common Issues

#### 1. Tests Fail to Connect to Services

```bash
# Check if services are running
docker-compose -f docker-compose.production.yml ps

# Restart services
docker-compose -f docker-compose.production.yml restart

# Check logs
docker-compose -f docker-compose.production.yml logs llm-edge-agent
```

#### 2. Load Tests Timeout

- Increase timeout in k6 scripts
- Reduce number of virtual users
- Check system resources (CPU/memory)

#### 3. Security Scan False Positives

Edit `tests/security/penetration-test.sh` to adjust test expectations based on your security requirements.

#### 4. Performance Regression Detected

```bash
# Review detailed benchmark results
cat test-reports/performance/benchmark-output.txt

# Check for resource constraints
docker stats

# Review flamegraph for hotspots
open performance-reports/flamegraph.svg
```

#### 5. Chaos Tests Fail

- Ensure Docker permissions are correct
- Check that infrastructure is fully started
- Review individual test outputs in `chaos-reports/`

### Debug Mode

```bash
# Run tests with verbose output
RUST_LOG=debug cargo test -- --nocapture

# Run k6 with debug output
k6 run --http-debug tests/load/baseline-load-test.js

# Docker Compose logs
docker-compose -f docker-compose.production.yml logs -f
```

---

## Test Maintenance

### Updating Baselines

```bash
# Update performance baselines
cargo bench --workspace -- --save-baseline main

# Update load test expectations
# Edit tests/load/*.js threshold values

# Update security test expectations
# Edit tests/security/*.sh validation criteria
```

### Adding New Tests

1. **Unit Tests**: Add to `*/tests/` in relevant crate
2. **Integration Tests**: Add to `/tests/integration_tests.rs`
3. **Load Tests**: Create new k6 script in `/tests/load/`
4. **Security Tests**: Add checks to existing scripts or create new
5. **Benchmarks**: Add to `/benches/`

---

## Conclusion

This comprehensive testing suite ensures the LLM Edge Agent meets enterprise production standards for:

- ✅ **Functional Correctness** (unit + integration tests)
- ✅ **Performance** (load tests + benchmarks)
- ✅ **Security** (OWASP + penetration tests)
- ✅ **Reliability** (chaos engineering)
- ✅ **Quality** (code quality checks)

For questions or issues, refer to the [main documentation](README.md) or open an issue.

---

**Last Updated**: 2025-11-08
**Version**: 1.0.0
