# Edge-Agent Code Locations Reference

## Critical Components (DO NOT MODIFY WITHOUT REVIEW)

This document maps the locations of proxy, routing, and caching implementations that should remain stable during upstream integration.

## 1. HTTP Proxy Implementation

### Location
`/workspaces/edge-agent/crates/llm-edge-proxy/`

### Module Structure
```
llm-edge-proxy/
├── Cargo.toml                    [Proxy crate manifest]
├── src/
│   ├── lib.rs                    [Entry point: exports Config, ProxyError, ProxyResult]
│   ├── server.rs                 [Server initialization - serve(), create_router(), build_app()]
│   ├── error.rs                  [Error types: ProxyError, ProxyResult]
│   ├── middleware.rs             [Middleware registry]
│   ├── server/
│   │   ├── routes.rs             [Route definitions and handlers]
│   │   ├── tracing.rs            [OpenTelemetry tracing setup]
│   │   └── tls.rs                [TLS/Rustls configuration]
│   ├── middleware/
│   │   ├── auth.rs               [Authentication middleware]
│   │   ├── rate_limit.rs         [Rate limiting via tower_governor]
│   │   └── timeout.rs            [Request timeout handling]
│   └── config/
│       └── mod.rs                [Configuration management - Config struct]
└── tests/                        [Integration tests]
```

### Key Public API
```rust
// Main exports from lib.rs
pub use config::Config;
pub use error::{ProxyError, ProxyResult};
pub use server::{build_app, create_router, serve};
```

### Critical Functions
- `serve(config: Config)` - Start proxy server
- `create_router()` - Build Axum router with middleware
- `build_app()` - Full application setup

### Middleware Integration Points
- `middleware/auth.rs` - NEW: Add upstream auth middleware here
- `middleware/rate_limit.rs` - EXTEND: Add upstream rate limiting here
- `server/routes.rs` - NEW: Add middleware to request pipeline

### TLS Configuration
- File: `server/tls.rs`
- Uses: rustls 0.23, tokio-rustls 0.26
- Current: Rustls with PEM file loading

### Dependencies in Cargo.toml
- axum 0.8, hyper 1.0, tower 0.5, tower-http 0.6
- rustls 0.23, tokio-rustls 0.26, rustls-pemfile 2.0
- tracing, opentelemetry (from workspace)

---

## 2. Routing Engine Implementation

### Location
`/workspaces/edge-agent/crates/llm-edge-routing/`

### Module Structure
```
llm-edge-routing/
├── Cargo.toml                    [Routing crate manifest]
├── src/
│   ├── lib.rs                    [Entry point: exports RoutingError, RoutingResult, strategy module]
│   ├── error.rs                  [Error types: RoutingError, RoutingResult]
│   ├── strategy.rs               [RoutingStrategy trait, RoutingDecision enum]
│   └── circuit_breaker.rs        [Circuit breaker pattern implementation]
└── tests/                        [Integration tests]
```

### Key Public API
```rust
// Main exports from lib.rs
pub use error::{RoutingError, RoutingResult};
pub use strategy::{RoutingDecision, RoutingStrategy};

// Circuit breaker for resilience
pub mod circuit_breaker;
```

### Core Traits & Types
```rust
// In strategy.rs
pub trait RoutingStrategy {
    fn route(&self, request: &Request) -> Result<RoutingDecision>;
}

pub enum RoutingDecision {
    Route(Provider),
    Fallback(Vec<Provider>),
    Error(RoutingError),
}
```

### Circuit Breaker (circuit_breaker.rs)
- Uses: failsafe 1.3 crate
- Pattern: Detects provider failures
- States: Closed -> Open -> Half-Open -> Closed
- Integration: Used in routing strategy selection

### Provider Integration
- Imports: `llm-edge-providers` (internal)
- Tracks: Provider health metrics
- Features: Failover chains, provider selection

### Dependencies in Cargo.toml
- llm-edge-providers (path: ../llm-edge-providers)
- failsafe 1.3 (circuit breaker)
- tokio, futures, async-trait (async)
- parking_lot 0.12.5 (synchronization)

### Scoring Logic (Strategy Implementation)
**Cost-based Routing:**
- Calculates: Cost per request × expected tokens
- Score: Lower cost = higher priority

**Latency-based Routing:**
- Measures: Response time metrics
- Score: Lower latency = higher priority

**Hybrid Routing:**
- Combines: Cost weight × latency weight
- Score: Weighted multi-factor decision

---

## 3. Multi-Tier Caching Implementation

### Location
`/workspaces/edge-agent/crates/llm-edge-cache/`

### Module Structure
```
llm-edge-cache/
├── Cargo.toml                    [Cache crate manifest]
├── src/
│   ├── lib.rs                    [Entry point: MultiTierCache, CacheLookupResult]
│   ├── key.rs                    [Cache key generation, CacheableRequest trait]
│   ├── l1.rs                     [L1 in-memory cache (Moka)]
│   ├── l2.rs                     [L2 distributed cache (Redis)]
│   ├── types.rs                  [Type definitions]
│   ├── metrics.rs                [Cache metrics and monitoring]
│   └── error.rs                  [Error types: CacheError, CacheResult]
└── tests/                        [Unit and integration tests]
```

### Key Public API
```rust
// Main exports from lib.rs
pub enum CacheLookupResult {
    L1Hit(Arc<CachedResponse>),
    L2Hit(Arc<CachedResponse>),
    Miss,
}

pub struct MultiTierCache {
    // L1 + L2 management
}

pub trait CacheableRequest {
    fn to_cache_key(&self) -> CacheKey;
}
```

### L1 Cache Implementation (l1.rs)
**Engine:** Moka 0.12 with future features
**Characteristics:**
- Eviction: TinyLFU algorithm
- Latency: <1ms (typically <100μs)
- TTL: 5 minutes default
- Scope: Per-instance (local memory)

**CachedResponse Type:**
```rust
pub struct CachedResponse {
    pub data: Vec<u8>,
    pub content_type: String,
    pub cache_time: Instant,
    pub ttl: Duration,
}
```

### L2 Cache Implementation (l2.rs)
**Engine:** Redis 0.27 with cluster support
**Characteristics:**
- Connection: tokio-comp, cluster-async features
- Latency: 1-2ms (network dependent)
- TTL: 1 hour default
- Scope: Shared across all instances
- Persistence: Data survives restarts

**Configuration (L2Config):**
```rust
pub struct L2Config {
    pub redis_url: String,
    pub default_ttl: Duration,
    pub cluster_enabled: bool,
}
```

### Cache Key Generation (key.rs)
**Algorithm:**
```
cache_key = SHA256(
    request_method +
    request_path +
    request_body_hash +
    user_id (if applicable)
)
```

**CacheableRequest Trait:**
- Identifies: Request fields affecting response
- Excludes: Timestamps, session tokens
- Implementation: Custom for each provider

### Metrics (metrics.rs)
**Tracked Metrics:**
- L1 hits/misses/evictions
- L2 hits/misses/evictions
- Cache size (bytes)
- Hit rate percentage
- Eviction rate

**Integration:**
- Exports: metrics 0.23 crate
- Format: Prometheus-compatible

### Performance Targets
- **L1 Latency:** <1ms (MVP)
- **L2 Latency:** 1-2ms (network)
- **Hit Rate:** >50% MVP, >70% Beta
- **Memory:** Configurable L1 size limit

### Dependencies in Cargo.toml
- moka 0.12 (future features for L1)
- redis 0.27 (tokio-comp, cluster-async for L2)
- sha2 0.10, hex 0.4 (key generation)
- tokio, futures (async runtime)
- tracing, metrics (observability)

### Integration with Request Flow
```
Request arrives
    ↓
[Generate cache key from request]
    ↓
[L1 lookup - in-memory Moka]
    ├─ HIT → Return response (~0.1ms)
    └─ MISS ↓
[L2 lookup - Redis]
    ├─ HIT → Populate L1 + return (~2ms)
    └─ MISS ↓
[Execute provider request]
    ↓
[Write result to L1 + L2 async]
```

---

## 4. Provider Adapters

### Location
`/workspaces/edge-agent/crates/llm-edge-providers/`

### Module Structure
```
llm-edge-providers/
├── Cargo.toml                    [Providers crate manifest]
├── src/
│   ├── lib.rs                    [Entry point: adapter module]
│   ├── adapter.rs                [Provider adapter trait and registry]
│   ├── openai.rs                 [OpenAI provider implementation]
│   ├── anthropic.rs              [Anthropic provider implementation]
│   ├── types.rs                  [Shared types: LLMRequest, LLMResponse]
│   └── error.rs                  [Error types: ProviderError, ProviderResult]
└── tests/                        [Provider integration tests]
```

### Key Public API
```rust
// Provider adapter trait
pub trait LLMProvider {
    async fn execute(&self, request: LLMRequest) -> ProviderResult<LLMResponse>;
    fn name(&self) -> &str;
}

// Provider registry
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn LLMProvider>>,
}
```

### Supported Providers (Current)
1. **OpenAI**
   - Models: GPT-3.5, GPT-4, GPT-4 Turbo
   - File: `openai.rs`

2. **Anthropic**
   - Models: Claude v1, Claude 2
   - File: `anthropic.rs`

3. **Extensible Pattern**
   - Add: New providers implementing LLMProvider trait
   - Register: In ProviderRegistry

### Request/Response Types (types.rs)
```rust
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: u32,
}

pub struct LLMResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
    pub timestamp: Instant,
}
```

### Dependencies in Cargo.toml
- reqwest 0.12 (HTTP client)
- reqwest-middleware 0.4 (middleware support)
- reqwest-retry 0.7 (retry logic)
- tokio, futures, async-trait (async)
- secrecy 0.8 (API key handling)

### API Key Management
- Uses: secrecy 0.8 crate
- Storage: Environment variables
- Format: Zeroized on drop (secure)

---

## 5. Security Layer

### Location
`/workspaces/edge-agent/crates/llm-edge-security/`

### Module Structure
```
llm-edge-security/
├── Cargo.toml
├── src/
│   ├── lib.rs                    [Entry point]
│   ├── auth.rs                   [Authentication - JWT, API keys]
│   ├── pii.rs                    [PII detection and redaction]
│   ├── validation.rs             [Input/output validation]
│   └── error.rs                  [Security errors]
└── tests/
```

### Integration Points for Upstream
**llm-shield-core:** Input/output validation
**llm-policy-engine:** Policy enforcement
**llm-sentinel:** Anomaly detection

### Current Implementation
- JWT validation (jsonwebtoken 9)
- API key validation
- PII pattern matching (regex 1.10)
- Field validation (validator 0.20)

---

## 6. Monitoring & Observability

### Location
`/workspaces/edge-agent/crates/llm-edge-monitoring/`

### Module Structure
```
llm-edge-monitoring/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── metrics.rs                [Metric definitions]
│   ├── tracing.rs                [Distributed tracing setup]
│   └── error.rs
└── tests/
```

### Metric Collection
- Uses: metrics 0.23 crate
- Export: metrics-exporter-prometheus 0.15
- Endpoint: /metrics (Prometheus format)

### Distributed Tracing
- Uses: opentelemetry 0.26, opentelemetry-otlp 0.26
- Exporter: OTLP (OpenTelemetry Protocol)
- Spans: Per-request tracing

### Integration Point for Upstream
**llm-observatory-core:** Observability enhancement

---

## 7. Workspace Root Configuration

### Location
`/workspaces/edge-agent/Cargo.toml`

### Workspace Members
```toml
[workspace]
members = [
    "crates/llm-edge-agent",
    "crates/llm-edge-proxy",
    "crates/llm-edge-cache",
    "crates/llm-edge-routing",
    "crates/llm-edge-providers",
    "crates/llm-edge-security",
    "crates/llm-edge-monitoring",
]
resolver = "2"
```

### Shared Dependencies
- All workspace members inherit from `[workspace.dependencies]`
- Version pinning: Central management at root level
- Features: Coordinated across crates

### Build Profiles
**Release:**
- opt-level = 3 (maximum optimization)
- lto = "fat" (link-time optimization)
- codegen-units = 1 (optimal compilation)
- strip = true (stripped binary)
- panic = "abort" (minimal overhead)

**Dev:**
- opt-level = 0 (fastest compilation)
- debug = true (debugging symbols)

---

## 8. Source Code Integrity

### Files NOT to Modify During Integration
- Any `.rs` file in existing crates without specific integration task
- `Cargo.toml` manifest files (use git for changes)
- Profile configurations in root `Cargo.toml`
- Lock file `Cargo.lock` (auto-generated)

### Files for Integration Changes
1. **Root Cargo.toml** - Add upstream dependencies
2. **Crate-specific Cargo.toml** - Add feature-gated deps
3. **Crate lib.rs files** - Import and expose upstream APIs
4. **Middleware files** - Add upstream middleware to pipeline
5. **Config files** - Add upstream configuration options

### Validation Checklist
- [ ] `cargo check` passes
- [ ] `cargo test` passes (all crates)
- [ ] `cargo clippy` passes (no warnings)
- [ ] `cargo fmt` compliant (formatting)
- [ ] Workspace dependencies resolve without conflicts
- [ ] No circular dependencies introduced

---

## 9. Integration Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│            llm-edge-agent (Main Binary)             │
│  Orchestrates all crates, configuration, startup    │
└────────────────────┬────────────────────────────────┘
                     │
    ┌────────────────┼────────────────┬──────────────┐
    │                │                │              │
┌───▼────────┐  ┌───▼────────┐  ┌───▼───────┐  ┌──▼──────────┐
│ Proxy      │  │ Routing    │  │ Cache     │  │Security     │
│ ---------- │  │ -----      │  │ -----     │  │-------      │
│ Request    │  │ Cost-based │  │ L1 (Moka) │  │ Auth        │
│ Routing    │  │ Latency    │  │ L2 (Redis)│  │ Validation  │
│ TLS/HTTPS  │  │ Failover   │  │ Metrics   │  │ PII         │
└────┬───────┘  └────┬───────┘  └─┬────────┘  └──┬──────────┘
     │               │            │              │
     │         ┌─────▼────────────▼──────────────▼──┐
     │         │  Providers (OpenAI, Anthropic)    │
     │         │  + Connector-Hub integration      │
     │         └─────┬─────────────────────────────┘
     │               │
     └───────────────┼──────────────┐
                     │              │
              ┌──────▼──────┐  ┌───▼──────────┐
              │ Monitoring  │  │ Upstream     │
              │ - Metrics   │  │ Integrations │
              │ - Tracing   │  │ - Shield     │
              └─────────────┘  │ - Sentinel   │
                               │ - PolicyEng  │
                               │ - CostOps    │
                               │ - Observatory│
                               └──────────────┘
```

---

## 10. Build & Compilation Notes

### Compilation Time
- Current: ~2-3 minutes (clean build)
- With upstreams: ~4-5 minutes (estimated)
- Incremental: <30 seconds

### Binary Size
- Release: ~15-20MB (current estimate)
- With upstreams: ~25-30MB (estimated)
- Optimization: -ffunction-sections, -fdata-sections

### Docker Build
- Multi-stage build (builder + runtime)
- Cache layers: Dependency caching enabled
- Test execution: Optional in build process

---

## Quick Reference

| Component | Location | Main File | Key Trait |
|-----------|----------|-----------|-----------|
| Proxy | `crates/llm-edge-proxy` | `server.rs` | N/A (Server) |
| Routing | `crates/llm-edge-routing` | `strategy.rs` | RoutingStrategy |
| Cache | `crates/llm-edge-cache` | `lib.rs` | CacheableRequest |
| Providers | `crates/llm-edge-providers` | `adapter.rs` | LLMProvider |
| Security | `crates/llm-edge-security` | `auth.rs` | N/A (utility) |
| Monitoring | `crates/llm-edge-monitoring` | `metrics.rs` | N/A (utility) |

---

**Document Version:** 1.0  
**Last Updated:** December 4, 2025  
**Status:** Reference (Do not modify)

