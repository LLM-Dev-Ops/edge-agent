# Upstream Repositories - Quick Reference

## Overview Table

| # | Project | Repository | Primary Crate | Version | Status | Integration Complexity |
|---|---------|-----------|---------------|---------|--------|----------------------|
| 1 | **Shield** | github.com/LLM-Dev-Ops/shield | llm-shield-core | 0.1.1 | Ready | Medium |
| 2 | **Sentinel** | github.com/LLM-Dev-Ops/sentinel | sentinel | 0.1.0 | Ready | Medium |
| 3 | **Connector-Hub** | github.com/LLM-Dev-Ops/connector-hub | connector-hub-core | 0.1.0 | Ready | High |
| 4 | **CostOps** | github.com/LLM-Dev-Ops/cost-ops | llm-cost-ops | 0.1.0 | Ready | Medium |
| 5 | **Observatory** | github.com/LLM-Dev-Ops/observatory | llm-observatory-core | 0.1.1 | Ready | Low |
| 6 | **Policy-Engine** | github.com/LLM-Dev-Ops/policy-engine | llm-policy-engine | 0.1.0 | Ready | Medium |

---

## 1. LLM-Shield (Security Scanning)

### Quick Facts
- **Type:** Workspace (15 member crates)
- **Primary Module:** llm-shield-core
- **Version:** 0.1.1
- **Edition:** 2021
- **GitHub:** https://github.com/LLM-Dev-Ops/shield

### Key Dependencies
```
tokio 1.35
ort 2.0.0-rc.10 (ALERT: Pre-release)
tokenizers 0.20
ndarray 0.16
llm-policy-engine (git)
llm-config-manager (git)
```

### Functionality
- ML-based threat detection
- Input/output validation
- Content filtering
- Token-level analysis

### Integration Target
- **Crate:** llm-edge-security
- **Module:** Middleware pipeline
- **Use Case:** Security validation layer

### Cargo Entry
```toml
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", branch = "main" }
```

### Notes
- Depends on pre-release ort 2.0.0-rc.10
- Monitor release status
- Consider fallback to stable version if needed

---

## 2. LLM-Sentinel (Threat Detection)

### Quick Facts
- **Type:** Workspace (6 member crates)
- **Primary Module:** sentinel (root)
- **Version:** 0.1.0
- **Edition:** 2021
- **GitHub:** https://github.com/LLM-Dev-Ops/sentinel

### Key Dependencies
```
tokio 1.42
axum 0.7, tonic 0.12, prost 0.13
rdkafka 0.36 (Kafka event streaming)
lapin 2.5 (RabbitMQ)
influxdb2 0.5 (metrics storage)
redis 0.27
ndarray 0.16, nalgebra 0.33, statrs 0.17
llm-shield-core, llm-shield-sdk (git)
llm-analytics-hub (git)
```

### Functionality
- Real-time anomaly detection
- Event ingestion and processing
- Statistical outlier detection
- Alert generation
- Storage of detection events

### Integration Target
- **Crate:** llm-edge-proxy
- **Module:** Request middleware
- **Use Case:** Runtime threat detection

### Cargo Entry
```toml
llm-sentinel = { git = "https://github.com/LLM-Dev-Ops/sentinel", branch = "main" }
```

### Message Queues Supported
- Kafka (rdkafka)
- RabbitMQ (lapin)
- HTTP/REST

### Notes
- Heavy event streaming focus
- Async-first design
- Requires message queue infrastructure (optional)

---

## 3. LLM-Connector-Hub (Provider Integration)

### Quick Facts
- **Type:** Monorepo (Rust + TypeScript)
- **Primary Module:** connector-hub-core
- **Version:** 0.1.0
- **Edition:** 2021
- **GitHub:** https://github.com/LLM-Dev-Ops/connector-hub

### Key Dependencies
```
tokio 1.0
serde 1.0, serde_json 1.0
schema-registry-core (git)
llm-config-core (git)
llm-observatory-core (git)
```

### Functionality
- Dynamic provider registration
- Provider capability discovery
- Schema registry and validation
- Multi-protocol support

### Integration Target
- **Crate:** llm-edge-routing
- **Module:** Provider discovery
- **Use Case:** Dynamic provider management

### Cargo Entry
```toml
connector-hub = { git = "https://github.com/LLM-Dev-Ops/connector-hub", branch = "main" }
```

### Features
- Schema validation for responses
- Protocol adapters (gRPC, REST, GraphQL)
- Provider health checking
- Capability-based selection

### Notes
- Monorepo structure (both Rust and TS)
- Schema registry integration
- Focuses on provider interoperability

---

## 4. LLM-CostOps (Cost Management)

### Quick Facts
- **Type:** Workspace (5 member crates)
- **Primary Crates:**
  - llm-cost-ops (core)
  - llm-cost-ops-sdk (integration)
  - llm-cost-ops-api (REST interface)
  - llm-cost-ops-cli (command-line)
  - llm-cost-ops-compliance (regulatory)
- **Version:** 0.1.0 (workspace)
- **Edition:** 2021
- **Rust:** 1.91+
- **GitHub:** https://github.com/LLM-Dev-Ops/cost-ops

### Key Dependencies
```
tokio 1.35
axum 0.7, tower 0.4, tower-http 0.5
sqlx 0.7 (database queries)
chrono 0.4, chrono-tz 0.8
rust_decimal 1.33 (financial precision)
metrics 0.21, metrics-exporter-prometheus 0.12
clap 4.4 (CLI)
redis 0.24 (optional)
```

### Functionality
- Cost calculation per request
- Token-based billing
- Budget tracking and alerts
- Multi-provider cost comparison
- Financial reporting
- Compliance tracking

### Integration Target
- **Crate:** llm-edge-routing
- **Module:** Routing strategy
- **Use Case:** Cost-based provider selection

### Cargo Entry
```toml
llm-cost-ops = { git = "https://github.com/LLM-Dev-Ops/cost-ops", branch = "main" }
```

### Optional Features
- redis-cache: Redis integration
- postgres-storage: PostgreSQL backend
- sqlite-storage: SQLite backend

### Database Support
- SQLx for compile-time checked queries
- PostgreSQL, SQLite, MySQL compatible

### Notes
- Financial precision: rust_decimal (not f64)
- Timezone-aware: chrono-tz
- Regulatory compliance built-in

---

## 5. LLM-Observatory (Observability)

### Quick Facts
- **Type:** Workspace (10 member crates)
- **Primary Module:** llm-observatory-core
- **Version:** 0.1.1
- **Edition:** 2021
- **GitHub:** https://github.com/LLM-Dev-Ops/observatory

### Workspace Members
```
crates/core          - Core APIs
crates/collector     - Data collection
crates/storage       - Storage backend
crates/api           - REST API
crates/sdk           - Integration SDK
crates/providers     - Provider integrations
crates/cli           - Command-line tools
crates/benchmarks    - Performance testing
crates/adapters      - Protocol adapters
services/analytics-api - Analytics
```

### Key Dependencies
```
tokio 1.42
axum 0.7, tonic 0.12, hyper 1.5
opentelemetry 0.27, tracing 0.1
metrics 0.24, metrics-exporter-prometheus 0.16
influxdb2 0.5 (time-series)
redis 0.27
sqlx 0.7
ndarray 0.16, statrs 0.17
```

### Functionality
- Metrics collection and export
- Distributed tracing (OpenTelemetry)
- Performance analytics
- Multi-backend storage (InfluxDB, PostgreSQL)
- Prometheus scraping
- Grafana dashboard support

### Integration Target
- **Crate:** llm-edge-monitoring
- **Module:** Metrics and tracing
- **Use Case:** Observability enhancement

### Cargo Entry
```toml
llm-observatory-core = { git = "https://github.com/LLM-Dev-Ops/observatory", branch = "main" }
```

### Storage Backends
- InfluxDB (time-series metrics)
- PostgreSQL (structured data)
- Redis (caching layer)

### Metrics Export Formats
- Prometheus text format
- OpenTelemetry OTLP gRPC
- JSON HTTP

### Notes
- Low complexity integration
- Pure additive (no breaking changes)
- Orthogonal to existing systems

---

## 6. LLM-Policy-Engine (Policy Enforcement)

### Quick Facts
- **Type:** Single crate (with sub-modules)
- **Primary Crate:** llm-policy-engine
- **Version:** 0.1.0
- **Edition:** 2021
- **GitHub:** https://github.com/LLM-Dev-Ops/policy-engine

### Key Dependencies
```
tokio 1.35
axum 0.7, tower 0.4, tower-http 0.5
tonic 0.11, prost 0.12 (gRPC)
cel-interpreter 0.7 (policy evaluation)
wasmtime 16.0 (WASM runtime)
moka 0.12 (caching)
redis 0.24 (optional)
jsonwebtoken 9, sha2 0.10
metrics 0.21, metrics-exporter-prometheus 0.13
governor 0.6 (rate limiting)
```

### Functionality
- CEL (Common Expression Language) policy evaluation
- WASM-based custom policies
- Rate limiting policies
- Access control (RBAC)
- Request/response filtering
- Audit logging

### Integration Target
- **Crate:** llm-edge-security
- **Module:** Policy validation
- **Use Case:** Policy enforcement layer

### Cargo Entry
```toml
llm-policy-engine = { git = "https://github.com/LLM-Dev-Ops/policy-engine", branch = "main" }
```

### Policy Languages
- **CEL:** Declarative rules (60% of use cases)
- **WASM:** Custom logic (40% of use cases)

### Optional Features
- redis-cache: Redis caching for policies
- postgres-storage: PostgreSQL policy storage
- sqlite-storage: SQLite policy storage

### Examples

**CEL Policy:**
```
request.user.role == "admin" && request.tokens <= 100000
```

**WASM Policy:**
```
Custom binary code for complex logic
```

### Notes
- Pluggable policy evaluation
- Supports both declarative (CEL) and imperative (WASM)
- Rate limiting integration
- Efficient caching mechanisms

---

## Integration Dependency Matrix

### Which Upstream Depends on Which?

```
Shield (0.1.1)
  └─ Depends on:
     - llm-policy-engine (git)
     - llm-config-manager (git)

Sentinel (0.1.0)
  └─ Depends on:
     - llm-shield-core (git)
     - llm-shield-sdk (git)
     - llm-analytics-hub (git)
     - llm-config-core (git)

Connector-Hub (0.1.0)
  └─ Depends on:
     - schema-registry-core (git)
     - llm-config-core (git)
     - llm-observatory-core (git)

CostOps (0.1.0)
  └─ Independent (no upstream deps)

Observatory (0.1.1)
  └─ Depends on:
     - llm-shield-core (git)
     - llm-shield-sdk (git)

Policy-Engine (0.1.0)
  └─ Independent (no upstream deps)
```

### Recommended Integration Order

**Phase 1:** CostOps, Policy-Engine, Observatory (independent)
**Phase 2:** Shield, Connector-Hub (light deps)
**Phase 3:** Sentinel (heavy deps on Phase 1)

---

## Shared Dependency Versions

### Critical Alignments
| Dependency | Edge-Agent | Shield | Sentinel | CostOps | Observatory | Policy-Engine | Status |
|-----------|-----------|--------|----------|---------|------------|---------------|---------|
| tokio | 1.40 | 1.35 | 1.42 | 1.35 | 1.42 | 1.35 | COMPATIBLE |
| axum | 0.8 | N/A | 0.7 | 0.7 | 0.7 | 0.7 | COMPATIBLE |
| serde | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | ALIGNED |
| tracing | 0.1 | 0.1 | 0.1 | 0.1 | 0.1 | 0.1 | ALIGNED |
| redis | 0.27 | N/A | 0.27 | 0.24 | 0.27 | 0.24 | COMPATIBLE |
| moka | 0.12 | N/A | 0.12 | N/A | N/A | 0.12 | ALIGNED |
| opentelemetry | 0.26 | N/A | 0.27 | N/A | 0.27 | 0.21 | COMPATIBLE |

### Pre-release Alerts
| Crate | Project | Version | Status | Action |
|-------|---------|---------|--------|--------|
| ort | Shield | 2.0.0-rc.10 | Pre-release | Monitor for stable release |

---

## Git Dependency Best Practices

### Development (Use main branch)
```toml
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", branch = "main" }
```

### Production (Pin to commit hash)
```toml
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", rev = "abc1234567890def" }
```

### Fallback to Published Crates (When available)
```toml
llm-shield-core = "0.1.1"  # From crates.io
```

---

## Testing Strategy Per Upstream

### Shield
- [ ] Test Scanner trait implementations
- [ ] Validate detection accuracy
- [ ] Check throughput impact
- [ ] Benchmark: <5ms overhead per request

### Sentinel
- [ ] Test event ingestion
- [ ] Validate anomaly detection
- [ ] Check message queue integration (if used)
- [ ] Benchmark: <10ms latency impact

### Connector-Hub
- [ ] Test provider discovery
- [ ] Validate schema validation
- [ ] Check dynamic registration
- [ ] Benchmark: <50ms discovery time

### CostOps
- [ ] Test cost calculation accuracy
- [ ] Validate billing aggregation
- [ ] Check budget enforcement
- [ ] Benchmark: <2ms cost lookup

### Observatory
- [ ] Test metrics export
- [ ] Validate trace collection
- [ ] Check storage backend integration
- [ ] Benchmark: <1ms metric recording

### Policy-Engine
- [ ] Test CEL evaluation
- [ ] Validate WASM execution
- [ ] Check policy caching
- [ ] Benchmark: <3ms policy evaluation

---

## Troubleshooting Reference

### Common Issues & Solutions

**Issue: Pre-release ort dependency in Shield**
- Solution 1: Monitor for 2.0.0 stable release
- Solution 2: Pin to older stable ort version
- Solution 3: Use feature flags to make optional

**Issue: Git dependency conflicts**
- Solution: Use Cargo.lock, commit hash pinning
- Fallback: Wait for crates.io publication

**Issue: Version mismatch in tokio**
- Status: Compatible (1.35-1.42 range all compatible)
- Action: No intervention needed

**Issue: Observable workspace changes**
- Strategy: Set up Dependabot alerts
- Action: Review changes before merging

---

## Maintenance Schedule

### Weekly
- Check upstream repository main branch status
- Verify CI/CD passes on all upstreams

### Monthly
- Review dependency updates
- Check for security patches
- Update Cargo.lock if needed

### Quarterly
- Full integration test suite
- Performance benchmarking
- Dependency audit with `cargo audit`

---

**Reference Document Version:** 1.0  
**Last Updated:** December 4, 2025  
**Status:** Active Reference

