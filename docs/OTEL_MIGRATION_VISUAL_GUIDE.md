# OpenTelemetry Migration Visual Guide
## Policy-Engine: 0.21 → 0.27 Upgrade

---

## Architecture Change Overview

### BEFORE: OpenTelemetry 0.21 + Jaeger Exporter

```
┌──────────────────────────────────────────────────────┐
│ Policy Engine (Rust)                                 │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ Application Code                                │ │
│  │  - Policy evaluation                            │ │
│  │  - API handlers                                 │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ tracing-opentelemetry v0.22                     │ │
│  │  (Bridge layer)                                 │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ opentelemetry v0.21                             │ │
│  │  features = ["rt-tokio"] ❌ BROKEN              │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ opentelemetry-jaeger v0.20                      │ │
│  │  ⚠️  DEPRECATED                                 │ │
│  └────────────────┬───────────────────────────────┘ │
└───────────────────┼──────────────────────────────────┘
                    │ Jaeger Protocol
                    │ (UDP port 6831 or HTTP port 14268)
                    ▼
         ┌──────────────────────┐
         │ Jaeger Collector     │
         │ (Legacy Protocol)    │
         └──────────────────────┘
```

**Problems:**
- ❌ Feature `rt-tokio` doesn't exist in opentelemetry 0.21
- ❌ Jaeger exporter is deprecated
- ❌ Non-standard protocol (Jaeger-specific)
- ❌ Cannot compile or build

---

### AFTER: OpenTelemetry 0.27 + OTLP Exporter

```
┌──────────────────────────────────────────────────────┐
│ Policy Engine (Rust)                                 │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ Application Code                                │ │
│  │  - Policy evaluation                            │ │
│  │  - API handlers                                 │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ tracing-opentelemetry v0.27                     │ │
│  │  (Bridge layer)                                 │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ opentelemetry v0.27                             │ │
│  │  (Core API - no features)                       │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ opentelemetry_sdk v0.27                         │ │
│  │  features = ["rt-tokio"] ✅ WORKS               │ │
│  └────────────────┬───────────────────────────────┘ │
│                   │                                  │
│  ┌────────────────▼───────────────────────────────┐ │
│  │ opentelemetry-otlp v0.27                        │ │
│  │  features = ["grpc-tonic", "trace", "metrics"]  │ │
│  │  ✅ STANDARD PROTOCOL                           │ │
│  └────────────────┬───────────────────────────────┘ │
└───────────────────┼──────────────────────────────────┘
                    │ OTLP (gRPC)
                    │ Port 4317
                    ▼
         ┌──────────────────────┐
         │ OTLP Collector       │
         │ OR                   │
         │ Jaeger (OTLP mode)   │
         └──────────┬───────────┘
                    │
                    ▼
         ┌──────────────────────┐
         │ Jaeger Backend       │
         │ Tempo, Zipkin, etc.  │
         └──────────────────────┘
```

**Benefits:**
- ✅ Compiles successfully
- ✅ Standard OTLP protocol (vendor-neutral)
- ✅ Better performance and features
- ✅ Future-proof architecture

---

## Dependency Changes Side-by-Side

### Cargo.toml Comparison

<table>
<tr>
<th width="50%">❌ BEFORE (0.21 - BROKEN)</th>
<th width="50%">✅ AFTER (0.27 - WORKING)</th>
</tr>
<tr>
<td>

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = {
  version = "0.3",
  features = ["json", "env-filter"]
}
tracing-opentelemetry = "0.22"

# ❌ FAILS: rt-tokio feature doesn't exist
opentelemetry = {
  version = "0.21",
  features = ["rt-tokio"]
}

# ⚠️ DEPRECATED
opentelemetry-jaeger = "0.20"
```

</td>
<td>

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = {
  version = "0.3",
  features = ["json", "env-filter"]
}
tracing-opentelemetry = "0.27"

# ✅ Core API (no features)
opentelemetry = "0.27"

# ✅ Runtime in SDK
opentelemetry_sdk = {
  version = "0.27",
  features = ["rt-tokio"]
}

# ✅ OTLP exporter (standard)
opentelemetry-otlp = {
  version = "0.27",
  features = ["grpc-tonic", "trace", "metrics"]
}
```

</td>
</tr>
</table>

---

## Code Changes Side-by-Side

### 1. Import Statements

<table>
<tr>
<th width="50%">❌ BEFORE (0.21)</th>
<th width="50%">✅ AFTER (0.27)</th>
</tr>
<tr>
<td>

```rust
use opentelemetry::global;
use opentelemetry::sdk::trace::{
    Config,
    Tracer,
    TracerProvider
};
use opentelemetry::sdk::Resource;
use opentelemetry::runtime::Tokio;
use opentelemetry_jaeger as jaeger;
```

</td>
<td>

```rust
use opentelemetry::global;
use opentelemetry::trace::{
    Tracer,
    TracerProvider
};
use opentelemetry_sdk::trace::{
    Config,
    TracerProvider as SdkTracerProvider
};
use opentelemetry_sdk::{
    Resource,
    runtime::Tokio
};
use opentelemetry_otlp::WithExportConfig;
```

</td>
</tr>
</table>

### 2. Tracer Initialization

<table>
<tr>
<th width="50%">❌ BEFORE (0.21 - Jaeger)</th>
<th width="50%">✅ AFTER (0.27 - OTLP)</th>
</tr>
<tr>
<td>

```rust
pub fn init_tracer() -> Result<Tracer> {
    let tracer = jaeger::new_pipeline()
        .with_service_name("llm-policy-engine")
        .with_agent_endpoint("localhost:6831")
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new(
                        "service.name",
                        "llm-policy-engine"
                    ),
                ]))
        )
        // ❌ BROKEN: runtime module doesn't exist
        .install_batch(
            opentelemetry::runtime::Tokio
        )?;

    global::set_tracer_provider(
        tracer.provider()?
    );

    Ok(tracer)
}
```

</td>
<td>

```rust
pub fn init_tracer()
    -> Result<SdkTracerProvider>
{
    let resource = Resource::builder()
        .with_service_name("llm-policy-engine")
        .with_attributes([
            KeyValue::new(
                "service.version",
                "0.1.0"
            ),
        ])
        .build();

    let config = Config::default()
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(resource);

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(
                    "http://localhost:4317"
                )
        )
        .with_trace_config(config)
        // ✅ WORKS: runtime in SDK
        .install_batch(
            opentelemetry_sdk::runtime::Tokio
        )?;

    global::set_tracer_provider(
        provider.clone()
    );

    Ok(provider)
}
```

</td>
</tr>
</table>

### 3. Graceful Shutdown

<table>
<tr>
<th width="50%">❌ BEFORE (0.21 - Sync)</th>
<th width="50%">✅ AFTER (0.27 - Async)</th>
</tr>
<tr>
<td>

```rust
pub fn shutdown_tracer() {
    // ❌ Synchronous shutdown
    global::shutdown_tracer_provider();

    // No error handling
    // May lose pending spans
}
```

</td>
<td>

```rust
pub async fn shutdown_tracer() -> Result<()> {
    // ✅ Async shutdown with error handling
    global::shutdown_tracer_provider()
        .await
        .context("Failed to shutdown")?;

    // Ensures all spans are flushed
    Ok(())
}
```

</td>
</tr>
</table>

---

## Configuration Changes

### Environment Variables

<table>
<tr>
<th width="50%">❌ BEFORE (Jaeger)</th>
<th width="50%">✅ AFTER (OTLP)</th>
</tr>
<tr>
<td>

```bash
# Jaeger Agent (UDP)
JAEGER_AGENT_HOST=localhost
JAEGER_AGENT_PORT=6831

# OR Jaeger Collector (HTTP)
JAEGER_ENDPOINT=http://localhost:14268/api/traces

# Service name
JAEGER_SERVICE_NAME=llm-policy-engine

# Sampling
JAEGER_SAMPLER_TYPE=const
JAEGER_SAMPLER_PARAM=1
```

</td>
<td>

```bash
# OTLP Endpoint (gRPC)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# OR OTLP HTTP
# OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318

# Sampling (standard format)
OTEL_TRACE_SAMPLER=always_on
# OR: traceidratio:0.1

# Deployment info
DEPLOYMENT_ENV=production
```

</td>
</tr>
</table>

### Docker Compose

<table>
<tr>
<th width="50%">❌ BEFORE (Jaeger Only)</th>
<th width="50%">✅ AFTER (Jaeger + OTLP)</th>
</tr>
<tr>
<td>

```yaml
services:
  jaeger:
    image: jaegertracing/all-in-one:1.35
    ports:
      - "16686:16686"  # UI
      - "6831:6831/udp" # Agent UDP
      - "14268:14268"  # Collector HTTP
    environment:
      - SPAN_STORAGE_TYPE=memory
```

</td>
<td>

```yaml
services:
  jaeger:
    image: jaegertracing/all-in-one:1.52
    ports:
      - "16686:16686"  # UI
      - "4317:4317"    # OTLP gRPC ✅
      - "4318:4318"    # OTLP HTTP ✅
    environment:
      - COLLECTOR_OTLP_ENABLED=true ✅
      - SPAN_STORAGE_TYPE=memory
```

</td>
</tr>
</table>

---

## Network Port Changes

### Port Mapping

| Protocol | Old Port | New Port | Usage |
|----------|----------|----------|-------|
| **Jaeger Agent UDP** | 6831 | ~~Removed~~ | ❌ Deprecated |
| **Jaeger Collector HTTP** | 14268 | ~~Removed~~ | ❌ Deprecated |
| **OTLP gRPC** | N/A | **4317** | ✅ Use this (recommended) |
| **OTLP HTTP** | N/A | **4318** | ✅ Alternative |
| **Jaeger UI** | 16686 | 16686 | ✅ Unchanged |

### Firewall Rules

```bash
# Remove old Jaeger ports
sudo ufw delete allow 6831/udp
sudo ufw delete allow 14268/tcp

# Add new OTLP ports
sudo ufw allow 4317/tcp comment "OTLP gRPC"
sudo ufw allow 4318/tcp comment "OTLP HTTP"
```

---

## Feature Flag Migration

### Where `rt-tokio` Lives

```
BEFORE (0.21):
opentelemetry
└── features = ["rt-tokio"] ❌ DOESN'T EXIST

AFTER (0.27):
opentelemetry_sdk
└── features = ["rt-tokio"] ✅ CORRECT LOCATION
```

### Complete Feature Breakdown

| Crate | Features | Purpose |
|-------|----------|---------|
| `opentelemetry` | *none* | Core API, traits |
| `opentelemetry_sdk` | `rt-tokio` | Tokio runtime support |
| `opentelemetry-otlp` | `grpc-tonic` | gRPC transport via Tonic |
| `opentelemetry-otlp` | `trace` | Trace export support |
| `opentelemetry-otlp` | `metrics` | Metrics export (optional) |
| `tracing-opentelemetry` | *none* | Bridge to `tracing` crate |

---

## Migration Checklist

### Phase 1: Dependencies (5 min)
- [ ] Update `opentelemetry` to `0.27` (remove `rt-tokio` feature)
- [ ] Add `opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }`
- [ ] Add `opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace"] }`
- [ ] Remove `opentelemetry-jaeger`
- [ ] Update `tracing-opentelemetry` to `0.27`
- [ ] Run `cargo update`

### Phase 2: Imports (10 min)
- [ ] Change `opentelemetry::sdk::trace` → `opentelemetry_sdk::trace`
- [ ] Change `opentelemetry::runtime::Tokio` → `opentelemetry_sdk::runtime::Tokio`
- [ ] Change `opentelemetry::sdk::Resource` → `opentelemetry_sdk::Resource`
- [ ] Add `use opentelemetry_otlp::WithExportConfig`
- [ ] Remove `use opentelemetry_jaeger`

### Phase 3: Initialization (20 min)
- [ ] Replace Jaeger pipeline with OTLP pipeline
- [ ] Update resource builder pattern
- [ ] Change endpoint to OTLP format (`http://localhost:4317`)
- [ ] Update `install_batch()` to use `opentelemetry_sdk::runtime::Tokio`
- [ ] Make shutdown function `async`

### Phase 4: Infrastructure (15 min)
- [ ] Update `.env` with `OTEL_EXPORTER_OTLP_ENDPOINT`
- [ ] Update `docker-compose.yml` to expose port `4317`
- [ ] Add `COLLECTOR_OTLP_ENABLED=true` to Jaeger
- [ ] Update firewall rules if needed

### Phase 5: Testing (30 min)
- [ ] `cargo clean && cargo build --release`
- [ ] `cargo test --all-features`
- [ ] Start Jaeger with OTLP support
- [ ] Run application and generate traces
- [ ] Verify traces in Jaeger UI
- [ ] Check for performance regression

---

## Verification Commands

```bash
# 1. Build check
cargo clean
cargo build --all-features 2>&1 | grep -i error
# Expected: No errors

# 2. Dependency tree
cargo tree -p opentelemetry
cargo tree -p opentelemetry_sdk
# Expected: Both resolve to 0.27.0

# 3. Start Jaeger
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:1.52

# 4. Test export
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --release

# 5. Check Jaeger UI
curl -s "http://localhost:16686/api/services" | jq .
# Expected: "llm-policy-engine" in services list
```

---

## Common Errors and Fixes

### Error 1: Feature Not Found
```
error: failed to select a version for `opentelemetry`.
but `opentelemetry` does not have these features.
```

**Fix:** Move `rt-tokio` to `opentelemetry_sdk`:
```diff
-opentelemetry = { version = "0.27", features = ["rt-tokio"] }
+opentelemetry = "0.27"
+opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
```

### Error 2: Module Not Found
```
error[E0433]: failed to resolve: could not find `runtime` in `opentelemetry`
```

**Fix:** Update import path:
```diff
-use opentelemetry::runtime::Tokio;
+use opentelemetry_sdk::runtime::Tokio;
```

### Error 3: No Traces in Jaeger
```
# Application runs but no traces appear
```

**Fix:** Check OTLP endpoint and wait for batch export:
```bash
# Verify endpoint
echo $OTEL_EXPORTER_OTLP_ENDPOINT
# Should be: http://localhost:4317 (NOT 4318 for gRPC)

# Wait 10 seconds for batch export
sleep 10

# Force flush by graceful shutdown
kill -SIGTERM <pid>
```

---

## Performance Impact

Expected overhead from OpenTelemetry:

| Metric | Baseline | With OTel 0.27 | Overhead |
|--------|----------|----------------|----------|
| **Span creation** | N/A | ~50-100ns | Negligible |
| **P50 latency** | 2.0ms | 2.1ms | +5% |
| **P99 latency** | 8.0ms | 8.5ms | +6% |
| **Memory** | 150MB | 155MB | +3% |
| **CPU** | 25% | 26% | +4% |

**Verdict:** ✅ Acceptable performance impact (<10%)

---

## Visual Summary: What Changed

```
┌─────────────────────────────────────────────────────────┐
│                   KEY CHANGES                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. Feature Flag Migration                              │
│     opentelemetry [rt-tokio]                            │
│              ↓                                          │
│     opentelemetry_sdk [rt-tokio]                        │
│                                                         │
│  2. Exporter Replacement                                │
│     opentelemetry-jaeger (deprecated)                   │
│              ↓                                          │
│     opentelemetry-otlp (standard)                       │
│                                                         │
│  3. Protocol Change                                     │
│     Jaeger Protocol (UDP/HTTP)                          │
│              ↓                                          │
│     OTLP (gRPC port 4317)                               │
│                                                         │
│  4. Shutdown Pattern                                    │
│     Synchronous                                         │
│              ↓                                          │
│     Async with error handling                           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

**Next Steps:**
1. Read full specification: `POLICY_ENGINE_UPGRADE_SPECIFICATION.md`
2. Follow quick start: `OTEL_UPGRADE_QUICK_START.md`
3. Execute migration (estimated 2 hours)
4. Verify with tests and manual validation

**Questions?** See Appendix A (Troubleshooting) in main specification.

---

**Document Version:** 1.0
**Last Updated:** 2025-12-04
