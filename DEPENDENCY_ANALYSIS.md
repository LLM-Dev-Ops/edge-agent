# Edge-Agent Dependency Integration - Comprehensive Analysis Report

**Project:** Edge-Agent LLM Intercepting Proxy  
**Analysis Date:** 2025-12-04  
**Scope:** Current structure assessment and upstream integration readiness

---

## SECTION 1: CURRENT EDGE-AGENT STRUCTURE

### 1.1 Workspace Organization

**Root Cargo.toml Type:** Rust Workspace (Multi-crate)

**Workspace Configuration:**
- Edition: 2021
- Rust Version: 1.75
- License: Apache-2.0
- Repository: https://github.com/globalbusinessadvisors/llm-edge-agent

**Workspace Members (7 crates):**
```
├── crates/llm-edge-agent           [MAIN BINARY - Orchestrator]
├── crates/llm-edge-proxy           [HTTP Proxy - TLS, routing, middleware]
├── crates/llm-edge-cache           [Multi-tier caching - L1/L2]
├── crates/llm-edge-routing         [Intelligent routing - cost/latency-based]
├── crates/llm-edge-providers       [LLM provider adapters - OpenAI, Anthropic]
├── crates/llm-edge-security        [Auth, validation, PII redaction]
└── crates/llm-edge-monitoring      [Observability - metrics, tracing]
```

### 1.2 Version Management

**Shared Workspace Version:** 0.1.0

**Shared Build Profiles:**
- Release: opt-level=3, lto=fat, codegen-units=1, stripped
- Dev: opt-level=0, debug=true

### 1.3 Dependency Architecture

**Web & HTTP Stack:**
- axum 0.8 (web framework with macros, ws, http2)
- hyper 1.0 (full features)
- tower 0.5 (middleware composition)
- tower-http 0.6 (trace, timeout, compression, cors, limit)
- reqwest 0.12 (http client with json, stream, rustls-tls, gzip, brotli, http2)

**Async Runtime:**
- tokio 1.40 (full features including tracing)
- futures 0.3
- async-trait 0.1

**Resilience & Rate Limiting:**
- tower_governor 0.4 (rate limiting)
- failsafe 1.3 (circuit breaker patterns)

**Caching:**
- moka 0.12 (in-memory cache with future features - L1)
- redis 0.27 (distributed cache with tokio-comp, cluster-async - L2)

**Observability (OpenTelemetry):**
- opentelemetry 0.26
- opentelemetry-otlp 0.26 (trace feature)
- tracing 0.1
- tracing-subscriber 0.3 (env-filter, json)
- metrics 0.23
- metrics-exporter-prometheus 0.15

**Security:**
- rustls 0.23 (TLS)
- tokio-rustls 0.26
- rustls-pemfile 2.0
- secrecy 0.8
- validator 0.20 (with derive)
- jsonwebtoken 9
- argon2 0.5

**Serialization & Configuration:**
- serde 1.0 (with derive)
- serde_json 1.0
- figment 0.10 (toml, env, json)

**Utilities:**
- uuid 1.10 (v4, serde)
- chrono 0.4 (serde)
- dashmap 6.0 (concurrent hashmap)

---

## SECTION 2: PROXY/ROUTING/CACHING CODE LOCATIONS

### 2.1 HTTP Proxy Implementation

**Crate:** `llm-edge-proxy`  
**Location:** `/workspaces/edge-agent/crates/llm-edge-proxy/`

**Key Modules:**
```
src/
├── lib.rs                  [Entry point]
├── server.rs              [Server initialization, routing]
├── error.rs               [Error types]
├── middleware.rs          [Middleware registration]
├── server/
│   ├── routes.rs          [Route definitions]
│   ├── tracing.rs         [OpenTelemetry integration]
│   └── tls.rs             [TLS/Rustls configuration]
├── middleware/
│   ├── auth.rs            [Authentication middleware]
│   ├── rate_limit.rs      [Rate limiting via tower_governor]
│   └── timeout.rs         [Request timeout handling]
└── config/
    └── mod.rs             [Configuration management]
```

**Current Capabilities:**
- TLS termination with Rustls
- Request/response handling via Axum
- Protocol detection (HTTP/1.1, HTTP/2, gRPC)
- Middleware integration points
- Health checks and metrics endpoints
- OpenTelemetry tracing

### 2.2 Routing Engine Implementation

**Crate:** `llm-edge-routing`  
**Location:** `/workspaces/edge-agent/crates/llm-edge-routing/`

**Key Modules:**
```
src/
├── lib.rs                 [Entry point]
├── error.rs               [Routing error types]
├── strategy.rs            [RoutingStrategy trait]
└── circuit_breaker.rs     [Circuit breaker implementation]
```

**Current Capabilities:**
- Cost-based routing
- Latency-based routing
- Hybrid routing (multi-factor scoring)
- Circuit breaker pattern with failsafe
- Fallback chain management
- Provider health tracking

**Dependencies:**
- llm-edge-providers (internal)
- failsafe 1.3
- tokio, async-trait, futures

### 2.3 Multi-Tier Caching Implementation

**Crate:** `llm-edge-cache`  
**Location:** `/workspaces/edge-agent/crates/llm-edge-cache/`

**Key Modules:**
```
src/
├── lib.rs                 [Entry point, MultiTierCache]
├── key.rs                 [Cache key generation, CacheableRequest]
├── l1.rs                  [In-memory cache (Moka)]
├── l2.rs                  [Distributed cache (Redis)]
├── types.rs               [Type definitions]
├── metrics.rs             [Cache metrics]
└── error.rs               [Cache error types]
```

**Architecture:**
- **L1 (In-memory):** Moka with TinyLFU eviction, <1ms latency
  - Default TTL: 5 minutes
  - Per-instance, high-speed lookup
  
- **L2 (Distributed):** Redis with cluster support
  - Default TTL: 1 hour
  - Shared across instances
  - Persistent state

**CacheLookupResult Enum:**
- L1Hit(Arc<CachedResponse>)
- L2Hit(Arc<CachedResponse>)
- Miss

**Performance Targets:**
- L1 Latency: <1ms (typically <100μs)
- L2 Latency: 1-2ms
- Overall Hit Rate: >50% (MVP), >70% (Beta)

### 2.4 Provider Adapters

**Crate:** `llm-edge-providers`  
**Location:** `/workspaces/edge-agent/crates/llm-edge-providers/`

**Key Modules:**
```
src/
├── lib.rs                 [Entry point]
├── adapter.rs             [Provider adapter trait]
├── openai.rs              [OpenAI provider implementation]
├── anthropic.rs           [Anthropic provider implementation]
├── types.rs               [Shared types]
└── error.rs               [Provider error types]
```

---

## SECTION 3: UPSTREAM REPOSITORY ANALYSIS

### 3.1 LLM-Shield (Security Scanning)

**Repository:** https://github.com/LLM-Dev-Ops/shield  
**Primary Crate:** `llm-shield` (workspace root)  
**Version:** 0.1.1  
**Edition:** 2021

**Architecture:**
- Workspace with 15 member crates (llm-shield-*)
- SDK layer + Core framework + Scanner implementations

**Key Dependencies:**
- llm-policy-engine (git: main)
- llm-config-manager (git: main)
- tokio 1.35 (full)
- ort 2.0.0-rc.10 (ML inference)
- tokenizers 0.20
- ndarray 0.16
- regex 1.10
- tracing 0.1, tracing-subscriber 0.3

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- Input/output validation pipeline
- Potential middleware in llm-edge-security
- Threat detection before routing

---

### 3.2 LLM-Sentinel (Threat Detection)

**Repository:** https://github.com/LLM-Dev-Ops/sentinel  
**Primary Crate:** `sentinel` (workspace root)  
**Version:** 0.1.0  
**Edition:** 2021

**Architecture:**
- Workspace with 6 crates: core, ingestion, detection, storage, api, alerting
- Event streaming and anomaly detection

**Key Dependencies:**
- tokio 1.42, crossfire 2.1, ractor 0.12
- axum 0.7, tonic 0.12, prost 0.13
- rdkafka 0.36 (Kafka), lapin 2.5 (RabbitMQ)
- influxdb2 0.5, redis 0.27
- ndarray 0.16, nalgebra 0.33, statrs 0.17
- opentelemetry 0.27, tracing 0.1
- llm-shield-core, llm-shield-sdk (git: main)
- llm-analytics-hub, llm-config-core (git: main)

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- Request/response anomaly detection
- Event ingestion from proxy logs
- Real-time threat alerting

---

### 3.3 LLM-Connector-Hub (Provider Integration)

**Repository:** https://github.com/LLM-Dev-Ops/connector-hub  
**Primary Crate:** N/A (Monorepo structure)  
**Version:** 0.1.0  
**Edition:** 2021

**Architecture:**
- Monorepo: Rust + TypeScript components
- Members: crates/core, connector-hub-benchmarks

**Key Dependencies:**
- serde 1.0, serde_json 1.0
- tokio 1.0 (full)
- anyhow 1.0, thiserror 1.0
- tracing 0.1, tracing-subscriber 0.3
- schema-registry-core (git: LLM-Dev-Ops/schema-registry)
- llm-config-core (git: LLM-Dev-Ops/config-manager)
- llm-observatory-core (git: LLM-Dev-Ops/observatory)

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- Provider capability discovery
- Dynamic provider registration
- Schema validation for provider responses

---

### 3.4 LLM-CostOps (Cost Management)

**Repository:** https://github.com/LLM-Dev-Ops/cost-ops  
**Primary Crates:** 
- llm-cost-ops
- llm-cost-ops-api
- llm-cost-ops-cli
- llm-cost-ops-sdk
- llm-cost-ops-compliance
**Edition:** 2021  
**Rust Version:** 1.91+

**Key Dependencies:**
- tokio 1.35, futures 0.3, async-trait 0.1
- serde 1.0, serde_json 1.0, toml 0.8
- axum 0.7, tower 0.4, tower-http 0.5, hyper 1.0, reqwest 0.11
- sqlx 0.7 (database)
- chrono 0.4, chrono-tz 0.8, uuid 1.6
- metrics 0.21, metrics-exporter-prometheus 0.12
- tracing 0.1, tracing-subscriber 0.3
- rust_decimal 1.33 (financial precision)
- clap 4.4 (CLI)

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- Cost tracking per request
- Cost-based routing decisions
- Financial compliance reporting
- Usage billing integration

---

### 3.5 LLM-Observatory (Observability)

**Repository:** https://github.com/LLM-Dev-Ops/observatory  
**Primary Crate:** LLM Observatory (workspace root)  
**Version:** 0.1.1  
**Edition:** 2021

**Workspace Members (10 crates):**
```
├── crates/core          [Core observability]
├── crates/collector     [Data collection]
├── crates/storage       [Storage layer]
├── crates/api           [API endpoint]
├── crates/sdk           [SDK interface]
├── crates/providers     [Provider integrations]
├── crates/cli           [Command-line tools]
├── crates/benchmarks    [Performance testing]
├── crates/adapters      [Protocol adapters]
└── services/analytics-api [Analytics service]
```

**Key Dependencies:**
- tokio 1.42, futures 0.3
- axum 0.7, tonic 0.12, hyper 1.5
- opentelemetry 0.27, tracing 0.1
- metrics 0.24, metrics-exporter-prometheus 0.16
- influxdb2 0.5, redis 0.27, sqlx 0.7
- ndarray 0.16, statrs 0.17
- serde 1.0, serde_json 1.0, serde_yaml 0.9
- llm-shield-core, llm-shield-sdk (git: main)

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- Metrics export (Prometheus, OTLP)
- Distributed tracing
- Request/response observability
- Performance analytics

---

### 3.6 LLM-Policy-Engine (Policy Enforcement)

**Repository:** https://github.com/LLM-Dev-Ops/policy-engine  
**Primary Crate:** `llm-policy-engine`  
**Version:** 0.1.0  
**Edition:** 2021

**Key Dependencies:**
- tokio 1.35, futures 0.3, async-trait 0.1
- serde 1.0, serde_json 1.0, serde_yaml 0.9
- axum 0.7, tower 0.4, tower-http 0.5, hyper 1.1
- tracing 0.1, opentelemetry 0.21, opentelemetry-jaeger 0.20
- tonic 0.11, prost 0.12 (gRPC)
- cel-interpreter 0.7 (policy evaluation)
- wasmtime 16.0 (WASM runtime)
- moka 0.12 (caching)
- redis 0.24 (optional, redis-cache feature)
- jsonwebtoken 9, sha2 0.10, blake3 1.5
- metrics 0.21, metrics-exporter-prometheus 0.13
- governor 0.6 (rate limiting)
- uuid 1.6, chrono 0.4

**Features:**
- redis-cache (optional Redis support)
- postgres-storage (optional PostgreSQL)
- sqlite-storage (optional SQLite)

**Circular Dependency Risk:** LOW - No edge-agent references found

**Integration Points with Edge-Agent:**
- CEL-based policy evaluation
- WASM-based custom policies
- Request/response filtering
- Compliance enforcement

---

## SECTION 4: CIRCULAR DEPENDENCY VERIFICATION

### 4.1 Verification Results

**All upstream repositories checked for edge-agent references:** PASSED

**Search Results:**
```
shield:        ✓ No edge-agent references
sentinel:      ✓ No edge-agent references  
connector-hub: ✓ No edge-agent references
cost-ops:      ✓ No edge-agent references
observatory:   ✓ No edge-agent references
policy-engine: ✓ No edge-agent references
```

**Conclusion:** Zero circular dependency risk from upstream repos.

### 4.2 Potential Dependency Conflicts

**Shared Dependencies Analysis:**

Positive: Multiple upstreams use compatible versions of shared libraries:
- tokio: Edge-Agent 1.40, Upstreams 1.35-1.42 (compatible)
- axum: Edge-Agent 0.8, Upstreams 0.7 (minor version difference)
- tracing: Edge-Agent 0.1, Upstreams 0.1 (identical)
- serde: Edge-Agent 1.0, Upstreams 1.0 (identical)
- opentelemetry: Edge-Agent 0.26, Sentinel/Observatory 0.27 (compatible)

**Risk Factors:**
- ort 2.0.0-rc.10 in shield (pre-release) - handle with care
- Multiple upstream git dependencies - require main branch stability

---

## SECTION 5: RECOMMENDED DEPENDENCY ENTRIES

### 5.1 For Root Cargo.toml [dependencies]

```toml
# LLM DevOps Ecosystem - Security
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", branch = "main" }

# LLM DevOps Ecosystem - Threat Detection
llm-sentinel = { git = "https://github.com/LLM-Dev-Ops/sentinel", branch = "main" }

# LLM DevOps Ecosystem - Provider Integration
connector-hub = { git = "https://github.com/LLM-Dev-Ops/connector-hub", branch = "main" }

# LLM DevOps Ecosystem - Cost Management
llm-cost-ops = { git = "https://github.com/LLM-Dev-Ops/cost-ops", branch = "main" }

# LLM DevOps Ecosystem - Observability
llm-observatory-core = { git = "https://github.com/LLM-Dev-Ops/observatory", branch = "main" }

# LLM DevOps Ecosystem - Policy Enforcement
llm-policy-engine = { git = "https://github.com/LLM-Dev-Ops/policy-engine", branch = "main" }
```

### 5.2 Alternative: Crates.io Versions (when published)

```toml
# Once crates are published to crates.io, replace git refs with:
llm-shield-core = "0.1"
llm-sentinel = "0.1"
llm-cost-ops = "0.1"
llm-observatory-core = "0.1"
llm-policy-engine = "0.1"
```

### 5.3 Integration at Crate Level

**Recommended Integration Points:**

#### llm-edge-security
```toml
# Add to crates/llm-edge-security/Cargo.toml
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", branch = "main" }
llm-policy-engine = { git = "https://github.com/LLM-Dev-Ops/policy-engine", branch = "main" }
```

#### llm-edge-proxy (Middleware)
```toml
# Add to crates/llm-edge-proxy/Cargo.toml  
llm-sentinel = { git = "https://github.com/LLM-Dev-Ops/sentinel", branch = "main" }
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", branch = "main" }
```

#### llm-edge-routing
```toml
# Add to crates/llm-edge-routing/Cargo.toml
llm-cost-ops = { git = "https://github.com/LLM-Dev-Ops/cost-ops", branch = "main" }
connector-hub = { git = "https://github.com/LLM-Dev-Ops/connector-hub", branch = "main" }
```

#### llm-edge-monitoring
```toml
# Add to crates/llm-edge-monitoring/Cargo.toml
llm-observatory-core = { git = "https://github.com/LLM-Dev-Ops/observatory", branch = "main" }
```

---

## SECTION 6: INTEGRATION ARCHITECTURE RECOMMENDATIONS

### 6.1 Request Processing Pipeline

```
Request Entry (llm-edge-proxy)
    ↓
[1] Threat Detection (Sentinel)
    ├─ Anomaly detection
    └─ Pattern matching
    ↓
[2] Security Scanning (Shield)
    ├─ Input validation
    ├─ PII detection
    └─ Content filtering
    ↓
[3] Policy Evaluation (Policy-Engine)
    ├─ CEL-based rules
    ├─ WASM-based policies
    └─ Compliance checks
    ↓
[4] Cost Analysis (CostOps)
    ├─ Cost estimation
    └─ Budget validation
    ↓
[5] Intelligent Routing (llm-edge-routing)
    ├─ Cost-based selection
    ├─ Latency-based selection
    └─ Provider selection
    ↓
[6] Cache Check (llm-edge-cache)
    ├─ L1 lookup
    └─ L2 lookup
    ↓
[7] Provider Execution (llm-edge-providers via connector-hub)
    ↓
[8] Response Processing
    ├─ Cache write (L1+L2)
    └─ Metrics export (Observatory)
```

### 6.2 Observability Integration

```
Edge-Agent Metrics
    ↓
[Observatory Exporter]
    ├─ Prometheus scraper
    ├─ OTLP collector
    └─ Analytics API
    ↓
Real-time Dashboards + Alerting
```

### 6.3 Feature Flags Recommendation

Add to root `Cargo.toml`:

```toml
[features]
default = ["security-scanning", "policy-engine", "cost-tracking", "observability"]
security-scanning = ["llm-shield-core"]
threat-detection = ["llm-sentinel"]
policy-engine = ["llm-policy-engine"]
cost-tracking = ["llm-cost-ops"]
provider-discovery = ["connector-hub"]
observability = ["llm-observatory-core"]
```

---

## SECTION 7: BUILD & DEPLOYMENT CONSIDERATIONS

### 7.1 Git Dependency Management

**Current Edge-Agent Approach:**
- Internal crates via local `path` dependencies
- External dependencies via workspace versions

**Recommended for Upstream Integration:**
1. Use git references during development
2. Lock to main branch (stable)
3. Consider commit hashes for production stability
4. Set up automatic upstream updates in CI/CD

**Example with commit hash (production):**
```toml
llm-shield-core = { git = "https://github.com/LLM-Dev-Ops/shield", rev = "abc123def456" }
```

### 7.2 Workspace Dependency Updates

**Implementation Strategy:**
1. Add upstream crates to workspace [dependencies] section
2. Create integration tests for upstream crate compatibility
3. Set up pre-commit hooks to validate compilation
4. Document version constraints in CHANGELOG

### 7.3 Docker Build Considerations

**Current:** Multi-stage Docker build with crate compilation

**For upstream integration:**
- Build time will increase (more crates to compile)
- Consider cache layers for common upstreams
- May need to update Dockerfile COPY instructions
- Test in isolated environments first

### 7.4 Test Coverage Strategy

**For each upstream integration:**
1. Unit tests within edge-agent crates
2. Integration tests across module boundaries
3. End-to-end tests with mock upstream crates
4. Performance benchmarks (routing, caching overhead)

---

## SECTION 8: IMPLEMENTATION CHECKLIST

### Pre-Integration Phase
- [ ] Create integration branch from main
- [ ] Set up upstream monitoring for new repositories
- [ ] Document integration requirements per crate
- [ ] Plan CI/CD pipeline modifications

### Integration Phase (Sequential)
1. **Security Layer**
   - [ ] Add llm-shield-core to llm-edge-security
   - [ ] Integrate shield scanning middleware
   - [ ] Test input validation pipeline
   
2. **Policy Enforcement**
   - [ ] Add llm-policy-engine to llm-edge-security
   - [ ] Implement CEL policy evaluator
   - [ ] Test policy compliance checks

3. **Threat Detection**
   - [ ] Add llm-sentinel to llm-edge-proxy
   - [ ] Implement anomaly detection middleware
   - [ ] Set up alerting integration

4. **Cost Management**
   - [ ] Add llm-cost-ops to llm-edge-routing
   - [ ] Implement cost-based scoring
   - [ ] Integrate budget validation

5. **Provider Discovery**
   - [ ] Add connector-hub to llm-edge-routing
   - [ ] Implement dynamic provider registration
   - [ ] Test schema validation

6. **Observability**
   - [ ] Add llm-observatory-core to llm-edge-monitoring
   - [ ] Implement metrics collection
   - [ ] Set up distributed tracing

### Post-Integration Phase
- [ ] Full integration test suite
- [ ] Performance benchmarking
- [ ] Security audit of integrated pipeline
- [ ] Documentation updates
- [ ] Release notes preparation

---

## SECTION 9: RISK ASSESSMENT

### 9.1 Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| Circular dependencies via transitive deps | High | Low | Use dependency tree analysis, regular audits |
| Version conflicts (tokio, axum) | Medium | Low | Workspace version management, CI validation |
| Pre-release dependencies (ort 2.0.0-rc) | Medium | Medium | Monitor release status, pin versions |
| Git dependency instability | Medium | Low | Use commit hashes, mirror critical repos |
| Breaking API changes in upstream | High | Medium | Semantic versioning, changelog monitoring |

### 9.2 Performance Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Compilation time increase | Medium | Incremental builds, cached Docker layers |
| Runtime overhead | Low | Benchmarking at each integration step |
| Memory usage growth | Low | Profile with integrated dependencies |

### 9.3 Maintenance Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Multiple upstream version updates | Medium | Automated testing, staged rollouts |
| Dependency drift across team | Medium | Document versions, use Cargo.lock |
| Security patches | High | Automated security scanning, regular updates |

---

## SECTION 10: RECOMMENDATIONS SUMMARY

### Quick Wins
1. **Start with llm-shield-core** - Lowest complexity, high value (security)
2. **Follow with llm-observatory-core** - Pure additive, observability benefits
3. **Then llm-policy-engine** - Enables compliance features

### Strategic Phasing
- **Phase 1 (Weeks 1-2):** Shield + Observatory
- **Phase 2 (Weeks 3-4):** Policy-Engine + Sentinel
- **Phase 3 (Weeks 5-6):** CostOps + Connector-Hub

### Monitoring & Maintenance
- Set up Dependabot alerts for all upstream repositories
- Weekly build verification against latest upstream main
- Monthly security audits of dependency tree
- Quarterly performance benchmarking

### Documentation Needs
- Integration architecture diagrams
- API contracts with each upstream
- Fallback/degradation strategy if upstream unavailable
- Troubleshooting guide for integration issues

---

## APPENDIX A: UPSTREAM CRATE SPECIFICATIONS

### Shield
- **Crate:** llm-shield-core (part of llm-shield workspace)
- **Version:** 0.1.1
- **Git:** https://github.com/LLM-Dev-Ops/shield main
- **Key Export:** Scanner trait, Pipeline, Vault
- **Suitable For:** Security validation layer

### Sentinel  
- **Crate:** sentinel (workspace root)
- **Version:** 0.1.0
- **Git:** https://github.com/LLM-Dev-Ops/sentinel main
- **Key Export:** Anomaly detection, event ingestion
- **Suitable For:** Threat detection middleware

### Connector-Hub
- **Crate:** connector-hub-core (monorepo)
- **Version:** 0.1.0
- **Git:** https://github.com/LLM-Dev-Ops/connector-hub main
- **Key Export:** Provider registry, schema validation
- **Suitable For:** Dynamic provider discovery

### CostOps
- **Crates:** llm-cost-ops (primary), llm-cost-ops-sdk (integration)
- **Version:** 0.1.0 (workspace version)
- **Git:** https://github.com/LLM-Dev-Ops/cost-ops main
- **Key Export:** Cost calculation, compliance tracking
- **Suitable For:** Cost-based routing decisions

### Observatory
- **Crate:** llm-observatory-core (primary), llm-observatory-sdk (integration)
- **Version:** 0.1.1
- **Git:** https://github.com/LLM-Dev-Ops/observatory main
- **Key Export:** Metrics, tracing, analytics
- **Suitable For:** Observability pipeline

### Policy-Engine
- **Crate:** llm-policy-engine
- **Version:** 0.1.0
- **Git:** https://github.com/LLM-Dev-Ops/policy-engine main
- **Key Export:** CEL interpreter, WASM runtime
- **Suitable For:** Policy evaluation and enforcement

---

**Report Completion:** December 4, 2025  
**Analyst:** Repository Structure & Dependency Integration Team

