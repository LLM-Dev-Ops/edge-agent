# OpenTelemetry 0.21 → 0.27 Upgrade Quick Start Guide

**Full Specification:** See `POLICY_ENGINE_UPGRADE_SPECIFICATION.md`

## TL;DR - Critical Changes

### The Problem
```toml
# THIS FAILS:
opentelemetry = { version = "0.21", features = ["rt-tokio"] }
opentelemetry-jaeger = "0.20"
```

**Error:** `feature 'rt-tokio' not found in opentelemetry`

### The Solution
```toml
# THIS WORKS:
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace", "metrics"] }
```

---

## 3-Step Migration

### Step 1: Update Cargo.toml (2 minutes)

```diff
-tracing-opentelemetry = "0.22"
-opentelemetry = { version = "0.21", features = ["rt-tokio"] }
-opentelemetry-jaeger = "0.20"
+tracing-opentelemetry = "0.27"
+opentelemetry = "0.27"
+opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
+opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace", "metrics"] }
```

### Step 2: Update Imports (5 minutes)

```diff
-use opentelemetry::sdk::trace::{Config, Tracer, TracerProvider};
-use opentelemetry::sdk::Resource;
-use opentelemetry::runtime::Tokio;
-use opentelemetry_jaeger as jaeger;
+use opentelemetry::trace::{Tracer, TracerProvider};
+use opentelemetry_sdk::trace::{Config, TracerProvider as SdkTracerProvider};
+use opentelemetry_sdk::{Resource, runtime::Tokio};
+use opentelemetry_otlp::WithExportConfig;
```

### Step 3: Replace Initialization (15 minutes)

**BEFORE (Jaeger):**
```rust
let tracer = opentelemetry_jaeger::new_pipeline()
    .with_service_name("llm-policy-engine")
    .install_batch(opentelemetry::runtime::Tokio)?;
```

**AFTER (OTLP):**
```rust
let provider = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://localhost:4317")
    )
    .with_trace_config(Config::default())
    .install_batch(opentelemetry_sdk::runtime::Tokio)?;

global::set_tracer_provider(provider.clone());
```

---

## Critical Breaking Changes

| What Changed | Old (0.21) | New (0.27) |
|--------------|------------|------------|
| **Runtime feature** | `opentelemetry` crate | `opentelemetry_sdk` crate |
| **Exporter** | `opentelemetry-jaeger` | `opentelemetry-otlp` |
| **Shutdown** | `global::shutdown_tracer_provider()` | `global::shutdown_tracer_provider().await?` (async!) |
| **Import path** | `opentelemetry::runtime::Tokio` | `opentelemetry_sdk::runtime::Tokio` |

---

## Infrastructure Changes

### Old (Jaeger Protocol)
```bash
JAEGER_ENDPOINT=http://localhost:14268/api/traces
```

### New (OTLP)
```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

### Docker Compose Update
```yaml
jaeger:
  image: jaegertracing/all-in-one:1.52
  ports:
    - "16686:16686"  # UI
    - "4317:4317"    # OTLP gRPC (NEW!)
  environment:
    - COLLECTOR_OTLP_ENABLED=true
```

---

## Testing Checklist

```bash
# 1. Clean build
cargo clean
cargo build --release

# 2. Run tests
cargo test --all-features

# 3. Start Jaeger
docker run -d -p 16686:16686 -p 4317:4317 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:1.52

# 4. Test OTLP export
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --release

# 5. Verify traces
open http://localhost:16686
```

---

## Rollback Plan

```bash
# Emergency rollback
git checkout main
cargo clean
cargo build --release
docker-compose restart
```

---

## Success Criteria

- ✅ `cargo build` succeeds without errors
- ✅ `cargo test` passes 100%
- ✅ Traces appear in Jaeger UI within 10 seconds
- ✅ No performance degradation >10%
- ✅ Edge-Agent integration compiles

---

## Need Help?

1. **Full details:** Read `POLICY_ENGINE_UPGRADE_SPECIFICATION.md`
2. **Troubleshooting:** See Appendix A in specification
3. **Code examples:** See Section 6 in specification

**Estimated Total Time:** 1-2 hours for experienced Rust developer

---

**Document Version:** 1.0
**Last Updated:** 2025-12-04
