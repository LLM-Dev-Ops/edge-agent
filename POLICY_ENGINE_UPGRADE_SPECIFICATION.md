# Policy-Engine OpenTelemetry 0.21 → 0.27 Upgrade Specification

**Document Version:** 1.0
**Date:** 2025-12-04
**Status:** CRITICAL - BLOCKING
**Repository:** https://github.com/LLM-Dev-Ops/policy-engine
**Priority:** P0 - Blocks Edge-Agent Phase 2B Integration
**Estimated Effort:** 8-12 hours
**Author:** Edge-Agent Integration Team

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State Analysis](#2-current-state-analysis)
3. [Target State Specification](#3-target-state-specification)
4. [Breaking Changes Documentation](#4-breaking-changes-documentation)
5. [Step-by-Step Migration Guide](#5-step-by-step-migration-guide)
6. [Code Changes Required](#6-code-changes-required)
7. [Testing Strategy](#7-testing-strategy)
8. [Rollback Plan](#8-rollback-plan)
9. [Infrastructure Requirements](#9-infrastructure-requirements)
10. [Success Criteria](#10-success-criteria)

---

## 1. Executive Summary

### 1.1 Why This Upgrade is Critical

The Policy-Engine repository currently specifies OpenTelemetry 0.21 with the `rt-tokio` feature flag in its Cargo.toml, which causes a **blocking dependency resolution error**:

```
error: failed to select a version for `opentelemetry`.
    ... required by package `llm-policy-engine v0.1.0`
versions that meet the requirements `^0.21` are: 0.21.0
the package `llm-policy-engine` depends on `opentelemetry`, with features: `rt-tokio`
but `opentelemetry` does not have these features.
```

**Root Cause:** OpenTelemetry 0.21 removed runtime-specific feature flags. The `rt-tokio` feature does not exist in this version or later versions.

**Critical Dependencies:**
1. **Edge-Agent Phase 2B Integration** requires a working Policy-Engine build
2. **Distributed tracing** is essential for observability in the LLM DevOps ecosystem
3. **OTLP standardization** is needed for multi-component telemetry correlation
4. **Production deployment** cannot proceed without telemetry infrastructure

### 1.2 Impact if Not Completed

| Impact Area | Severity | Description |
|------------|----------|-------------|
| **Edge-Agent Integration** | CRITICAL | Phase 2B completely blocked; cannot compile or integrate |
| **Observability** | HIGH | No distributed tracing; debugging production issues impossible |
| **Compliance** | HIGH | Audit trails incomplete without trace correlation |
| **Development Velocity** | MEDIUM | Developer experience degraded; local testing blocked |
| **Production Readiness** | CRITICAL | Cannot deploy to production environments |

### 1.3 Timeline and Priority

| Phase | Duration | Priority |
|-------|----------|----------|
| **Dependency Updates** | 1-2 hours | P0 |
| **Compilation Fixes** | 2-3 hours | P0 |
| **Code Migration** | 3-4 hours | P0 |
| **Testing & Validation** | 2-3 hours | P1 |
| **Documentation** | 1 hour | P2 |

**Target Completion:** Within 1 business day
**Review Window:** 4 hours
**Deployment Window:** Immediately after approval

---

## 2. Current State Analysis

### 2.1 Current OpenTelemetry 0.21 Dependencies

**File:** `/Cargo.toml` (Lines 36-40)

```toml
# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.22"
opentelemetry = { version = "0.21", features = ["rt-tokio"] }
opentelemetry-jaeger = "0.20"
```

**Dependency Tree:**
```
llm-policy-engine v0.1.0
├── opentelemetry v0.21 [INVALID: rt-tokio feature doesn't exist]
├── opentelemetry-jaeger v0.20 [DEPRECATED]
└── tracing-opentelemetry v0.22 [INCOMPATIBLE with OTel 0.21]
```

### 2.2 Deprecated Jaeger Exporter Usage

**Status:** `opentelemetry-jaeger` is **DEPRECATED** as of OpenTelemetry 0.22

**Migration Path:**
- Jaeger exporter removed from official crates
- Replacement: OTLP exporter (gRPC or HTTP)
- Jaeger backend supports OTLP ingestion (native since Jaeger 1.35+)

**Current Limitation:**
- Direct HTTP export to Jaeger collector endpoint
- No OTLP standardization
- Limited compatibility with other observability backends

### 2.3 Feature Flags Being Used

**Invalid Feature Flag:**
```toml
opentelemetry = { version = "0.21", features = ["rt-tokio"] }
```

**Why It Fails:**
1. OpenTelemetry 0.21+ removed `rt-tokio`, `rt-async-std`, and `rt-tokio-current-thread` features
2. Runtime selection now handled by the exporter crates, not the core library
3. The tracer provider no longer requires runtime parameter specification

**Migration Required:**
- Remove `rt-tokio` feature flag
- Add `rt-tokio` feature to exporter crates (`opentelemetry-otlp`)

### 2.4 Current Code Patterns (Expected)

Based on the Cargo.toml configuration, the expected usage pattern in Rust code would be:

```rust
// Expected pattern in src/observability/tracing.rs or similar
use opentelemetry::global;
use opentelemetry::sdk::trace::{Config, Tracer};
use opentelemetry::sdk::Resource;
use opentelemetry_jaeger::PipelineBuilder;

pub fn init_tracer() -> Result<Tracer> {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("llm-policy-engine")
        .with_agent_endpoint("localhost:6831")  // UDP agent
        .install_batch(opentelemetry::runtime::Tokio)?;  // ❌ BROKEN

    Ok(tracer)
}
```

**Issues:**
1. `install_batch(opentelemetry::runtime::Tokio)` doesn't exist in 0.21
2. Jaeger exporter deprecated
3. No graceful shutdown pattern

---

## 3. Target State Specification

### 3.1 Complete Cargo.toml Changes Needed

**File:** `/Cargo.toml` (Lines 36-40)

**BEFORE:**
```toml
# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.22"
opentelemetry = { version = "0.21", features = ["rt-tokio"] }
opentelemetry-jaeger = "0.20"
```

**AFTER:**
```toml
# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.27"
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "metrics", "trace"] }
```

### 3.2 OpenTelemetry 0.27 Dependencies

| Crate | Version | Features | Purpose |
|-------|---------|----------|---------|
| `opentelemetry` | 0.27 | (none) | Core API, traits, and types |
| `opentelemetry_sdk` | 0.27 | `rt-tokio` | SDK implementation with Tokio runtime support |
| `opentelemetry-otlp` | 0.27 | `grpc-tonic`, `metrics`, `trace` | OTLP exporter for traces and metrics |
| `tracing-opentelemetry` | 0.27 | (none) | Bridge between `tracing` and OpenTelemetry |

**Dependency Relationships:**
```
llm-policy-engine v0.1.0
├── opentelemetry v0.27 (API only)
├── opentelemetry_sdk v0.27 [rt-tokio]
├── opentelemetry-otlp v0.27 [grpc-tonic, metrics, trace]
│   └── tonic v0.11 (already in deps)
└── tracing-opentelemetry v0.27
```

### 3.3 Migration from Jaeger to OTLP Exporter

**Architecture Change:**

```
BEFORE (OpenTelemetry 0.21):
┌─────────────────┐
│ Policy Engine   │
│   (Rust)        │
└────────┬────────┘
         │ Jaeger Protocol (UDP/HTTP)
         │
         ▼
┌─────────────────┐
│ Jaeger Collector│
└─────────────────┘

AFTER (OpenTelemetry 0.27):
┌─────────────────┐
│ Policy Engine   │
│   (Rust)        │
└────────┬────────┘
         │ OTLP (gRPC:4317 or HTTP:4318)
         │
         ▼
┌─────────────────┐      ┌─────────────────┐
│ OTLP Collector  │─────▶│ Jaeger Backend  │
│ (or direct)     │      │ (OTLP ingestion)│
└─────────────────┘      └─────────────────┘
```

**Benefits:**
1. **Standardization:** OTLP is vendor-neutral and widely supported
2. **Flexibility:** Can switch backends (Jaeger, Tempo, Zipkin, etc.) without code changes
3. **Future-proof:** OTLP is the long-term standard for OpenTelemetry
4. **Feature-complete:** Supports traces, metrics, and logs (future)

### 3.4 Required Feature Flags

```toml
# Core API - NO features needed
opentelemetry = "0.27"

# SDK with Tokio runtime support
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }

# OTLP exporter with gRPC transport
opentelemetry-otlp = {
  version = "0.27",
  features = [
    "grpc-tonic",  # Use Tonic for gRPC (alternative: "http-proto")
    "trace",        # Enable trace export
    "metrics"       # Enable metrics export (optional but recommended)
  ]
}

# Tracing bridge - NO features needed
tracing-opentelemetry = "0.27"
```

**Feature Flag Explanation:**

| Feature | Crate | Purpose | Required |
|---------|-------|---------|----------|
| `rt-tokio` | `opentelemetry_sdk` | Tokio runtime integration for async operations | ✅ Yes |
| `grpc-tonic` | `opentelemetry-otlp` | Use Tonic for gRPC transport (vs reqwest HTTP) | ✅ Yes |
| `trace` | `opentelemetry-otlp` | Enable trace exporting | ✅ Yes |
| `metrics` | `opentelemetry-otlp` | Enable metrics exporting | ⚠️ Recommended |
| `logs` | `opentelemetry-otlp` | Enable log exporting (experimental) | ❌ Optional |

---

## 4. Breaking Changes Documentation

### 4.1 API Changes from 0.21 → 0.27

#### 4.1.1 Tracer Provider Initialization

**BEFORE (0.21):**
```rust
use opentelemetry::sdk::trace::{Config, Tracer};
use opentelemetry_jaeger::PipelineBuilder;

let tracer = opentelemetry_jaeger::new_pipeline()
    .with_service_name("llm-policy-engine")
    .install_batch(opentelemetry::runtime::Tokio)?; // ❌ Removed
```

**AFTER (0.27):**
```rust
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::{TracerProvider as SdkTracerProvider, Config};
use opentelemetry_otlp::WithExportConfig;

let provider = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://localhost:4317")
    )
    .with_trace_config(Config::default())
    .install_batch(opentelemetry_sdk::runtime::Tokio)?; // ✅ New location
```

#### 4.1.2 Runtime Parameter Removal

**Breaking Change:** The `opentelemetry::runtime` module moved to `opentelemetry_sdk::runtime`

**BEFORE:**
```rust
use opentelemetry::runtime::Tokio;

.install_batch(Tokio)?;
```

**AFTER:**
```rust
use opentelemetry_sdk::runtime::Tokio;

.install_batch(Tokio)?;
```

#### 4.1.3 Resource Builder API Changes

**BEFORE (0.21):**
```rust
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;

let resource = Resource::new(vec![
    KeyValue::new("service.name", "llm-policy-engine"),
    KeyValue::new("service.version", "0.1.0"),
]);
```

**AFTER (0.27):**
```rust
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;

let resource = Resource::new(vec![
    KeyValue::new("service.name", "llm-policy-engine"),
    KeyValue::new("service.version", "0.1.0"),
]);
// OR use builder pattern:
let resource = Resource::builder()
    .with_service_name("llm-policy-engine")
    .with_attributes([
        KeyValue::new("service.version", "0.1.0"),
        KeyValue::new("deployment.environment", "production"),
    ])
    .build();
```

### 4.2 Provider Initialization Patterns

#### 4.2.1 OLD Pattern (0.21) - DEPRECATED

```rust
use opentelemetry::global;
use opentelemetry::sdk::trace::{self, Sampler};
use opentelemetry_jaeger as jaeger;

pub fn init_tracer() -> Result<trace::Tracer, Box<dyn std::error::Error>> {
    let tracer = jaeger::new_pipeline()
        .with_service_name("llm-policy-engine")
        .with_agent_endpoint("localhost:6831")
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "llm-policy-engine"),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    global::set_tracer_provider(tracer.provider()?);
    Ok(tracer)
}
```

#### 4.2.2 NEW Pattern (0.27) - REQUIRED

```rust
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::{self, Sampler, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry::KeyValue;

pub fn init_tracer() -> Result<SdkTracerProvider, Box<dyn std::error::Error>> {
    // Build resource
    let resource = Resource::builder()
        .with_service_name("llm-policy-engine")
        .with_attributes([
            KeyValue::new("service.version", "0.1.0"),
            KeyValue::new("deployment.environment", std::env::var("ENV").unwrap_or_else(|_| "development".into())),
        ])
        .build();

    // Create trace config
    let config = trace::Config::default()
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(resource);

    // Initialize OTLP pipeline
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4317".into()))
        )
        .with_trace_config(config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    Ok(provider)
}
```

### 4.3 Batch Processor Setup

**BEFORE (0.21):**
```rust
use opentelemetry::sdk::trace::BatchSpanProcessor;

let processor = BatchSpanProcessor::builder(exporter, opentelemetry::runtime::Tokio)
    .with_max_queue_size(2048)
    .with_scheduled_delay(std::time::Duration::from_secs(5))
    .build();
```

**AFTER (0.27):**
```rust
use opentelemetry_sdk::trace::BatchSpanProcessor;

let processor = BatchSpanProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio)
    .with_max_queue_size(2048)
    .with_scheduled_delay(std::time::Duration::from_secs(5))
    .build();
```

**Key Changes:**
1. Import from `opentelemetry_sdk::trace` instead of `opentelemetry::sdk::trace`
2. Use `opentelemetry_sdk::runtime::Tokio` instead of `opentelemetry::runtime::Tokio`

### 4.4 Graceful Shutdown Pattern Changes

**BEFORE (0.21):**
```rust
use opentelemetry::global;

pub async fn shutdown_tracer() {
    global::shutdown_tracer_provider(); // Synchronous
}
```

**AFTER (0.27):**
```rust
use opentelemetry::global;

pub async fn shutdown_tracer() {
    // Must await the async shutdown
    if let Err(err) = global::shutdown_tracer_provider().await {
        eprintln!("Error shutting down tracer provider: {:?}", err);
    }
}
```

**Critical Change:** Shutdown is now **async** and returns a `Result`. You must:
1. Call `.await` on the shutdown method
2. Handle potential errors
3. Ensure shutdown is called before process exit to flush pending spans

---

## 5. Step-by-Step Migration Guide

### Phase 1: Update Dependencies (1-2 hours)

#### Step 1.1: Update Cargo.toml

```bash
# Edit Cargo.toml
# Replace the tracing section (lines 36-40)
```

**Before:**
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.22"
opentelemetry = { version = "0.21", features = ["rt-tokio"] }
opentelemetry-jaeger = "0.20"
```

**After:**
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.27"
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace", "metrics"] }
```

#### Step 1.2: Update Lock File

```bash
cd /path/to/policy-engine
cargo update -p opentelemetry
cargo update -p opentelemetry_sdk
cargo update -p opentelemetry-otlp
cargo update -p tracing-opentelemetry
```

#### Step 1.3: Verify Dependency Resolution

```bash
cargo tree -p opentelemetry
cargo tree -p opentelemetry_sdk
cargo tree -p opentelemetry-otlp
```

**Expected Output:**
```
opentelemetry v0.27.0
├── opentelemetry_sdk v0.27.0
│   ├── tokio v1.35.0
│   └── ...
└── opentelemetry-otlp v0.27.0
    ├── tonic v0.11.0
    └── ...
```

### Phase 2: Fix Compilation Errors (2-3 hours)

#### Step 2.1: Identify Affected Files

```bash
# Find all Rust files using OpenTelemetry
rg -l "opentelemetry|tracer|tracing" --type rust src/

# Expected files (based on architecture):
# - src/observability/mod.rs
# - src/observability/tracing.rs
# - src/observability/metrics.rs
# - src/daemon/main.rs
# - src/lib.rs
```

#### Step 2.2: Update Import Statements

**In all affected files, update imports:**

**BEFORE:**
```rust
use opentelemetry::sdk::trace::{Config, Tracer, TracerProvider};
use opentelemetry::sdk::Resource;
use opentelemetry::runtime::Tokio;
use opentelemetry_jaeger as jaeger;
```

**AFTER:**
```rust
use opentelemetry::trace::{Tracer, TracerProvider};
use opentelemetry_sdk::trace::{Config, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::{Resource, runtime::Tokio};
use opentelemetry_otlp::WithExportConfig;
```

#### Step 2.3: Run Incremental Compilation

```bash
cargo check --all-features
cargo clippy --all-features
```

**Common Errors to Fix:**

| Error | Cause | Fix |
|-------|-------|-----|
| `cannot find 'runtime' in module 'opentelemetry'` | Module moved | Change to `opentelemetry_sdk::runtime` |
| `no 'rt-tokio' feature` | Feature moved | Add to `opentelemetry_sdk` instead |
| `cannot find 'jaeger' in scope` | Crate removed | Replace with OTLP exporter |
| `method 'install_batch' not found` | API changed | Use new pipeline pattern |

### Phase 3: Update Initialization Code (3-4 hours)

#### Step 3.1: Create New Tracing Module

**File:** `src/observability/tracing.rs`

```rust
//! Distributed tracing using OpenTelemetry OTLP
//!
//! Migrated from Jaeger exporter (deprecated) to OTLP in OpenTelemetry 0.27

use anyhow::{Context, Result};
use opentelemetry::global;
use opentelemetry::trace::{Tracer, TracerProvider};
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::{Config, Sampler, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

/// Initialize OpenTelemetry tracing with OTLP exporter
pub fn init_telemetry() -> Result<SdkTracerProvider> {
    // Build service resource
    let resource = Resource::builder()
        .with_service_name("llm-policy-engine")
        .with_attributes([
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new(
                "deployment.environment",
                std::env::var("DEPLOYMENT_ENV").unwrap_or_else(|_| "development".into()),
            ),
            KeyValue::new("service.namespace", "llm-devops"),
        ])
        .build();

    // Configure trace sampling
    let sampler = match std::env::var("OTEL_TRACE_SAMPLER").as_deref() {
        Ok("always_on") => Sampler::AlwaysOn,
        Ok("always_off") => Sampler::AlwaysOff,
        Ok(val) if val.starts_with("traceidratio:") => {
            let ratio = val
                .strip_prefix("traceidratio:")
                .and_then(|r| r.parse::<f64>().ok())
                .unwrap_or(1.0);
            Sampler::TraceIdRatioBased(ratio)
        }
        _ => Sampler::AlwaysOn, // Default for development
    };

    // Create trace config
    let config = Config::default()
        .with_sampler(sampler)
        .with_resource(resource)
        .with_max_events_per_span(128)
        .with_max_attributes_per_span(64);

    // Get OTLP endpoint from environment
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".into());

    tracing::info!("Initializing OTLP tracer with endpoint: {}", otlp_endpoint);

    // Build OTLP exporter pipeline
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint)
                .with_timeout(std::time::Duration::from_secs(3)),
        )
        .with_trace_config(config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .context("Failed to initialize OTLP tracer")?;

    // Set as global tracer provider
    global::set_tracer_provider(provider.clone());

    // Create tracing-opentelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(provider.tracer("llm-policy-engine"));

    // Create env filter layer
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,llm_policy_engine=debug"));

    // Build subscriber with layers
    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer);

    // Set as global default
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set global tracing subscriber")?;

    tracing::info!("OpenTelemetry tracing initialized successfully");

    Ok(provider)
}

/// Shutdown OpenTelemetry gracefully
pub async fn shutdown_telemetry() -> Result<()> {
    tracing::info!("Shutting down OpenTelemetry tracer provider");

    global::shutdown_tracer_provider()
        .await
        .context("Failed to shutdown tracer provider")?;

    tracing::info!("OpenTelemetry shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_init_shutdown() {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

        let provider = init_telemetry().expect("Failed to init telemetry");

        // Verify provider is set
        let tracer = global::tracer("test");
        assert!(tracer.tracer_provider().is_some());

        // Clean shutdown
        drop(provider);
        shutdown_telemetry().await.expect("Failed to shutdown");
    }
}
```

#### Step 3.2: Update Main Entry Point

**File:** `src/daemon/main.rs` or `src/lib.rs`

```rust
use llm_policy_engine::observability::tracing::{init_telemetry, shutdown_telemetry};
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize telemetry FIRST (before any other logging)
    let _tracer_provider = init_telemetry()?;

    info!("LLM Policy Engine starting...");

    // Initialize other components
    // ...

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Received shutdown signal");

    // Graceful shutdown
    shutdown_telemetry().await?;

    info!("Shutdown complete");
    Ok(())
}
```

#### Step 3.3: Update Policy Engine Integration

**File:** `src/api/engine.rs` (or wherever PolicyEngine is defined)

```rust
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use opentelemetry::global;
use tracing::{info_span, instrument};

pub struct PolicyEngine {
    // ... existing fields
}

impl PolicyEngine {
    /// Evaluate policy with distributed tracing
    #[instrument(skip(self, context), fields(
        policy_id = %policy_id,
        request_id = %context.request_id
    ))]
    pub async fn evaluate(
        &self,
        policy_id: &str,
        context: EvaluationContext,
    ) -> Result<PolicyDecision> {
        // Tracing is automatically instrumented via #[instrument]
        // Additional custom spans can be created if needed:

        let tracer = global::tracer("llm-policy-engine");
        let mut span = tracer
            .span_builder("policy.evaluate")
            .with_attributes(vec![
                KeyValue::new("policy.id", policy_id.to_string()),
                KeyValue::new("policy.namespace", context.namespace.clone()),
            ])
            .start(&tracer);

        // Your evaluation logic
        let decision = self.evaluate_internal(policy_id, context).await?;

        span.set_attribute(KeyValue::new("policy.decision", decision.decision.to_string()));
        span.end();

        Ok(decision)
    }
}
```

### Phase 4: Replace Jaeger with OTLP (Included in Phase 3)

**Already completed in Step 3.1** - The new `init_telemetry()` function uses OTLP instead of Jaeger.

**Configuration Changes:**

**Old Environment Variables (Jaeger):**
```bash
JAEGER_ENDPOINT=http://localhost:14268/api/traces
JAEGER_AGENT_HOST=localhost
JAEGER_AGENT_PORT=6831
```

**New Environment Variables (OTLP):**
```bash
# OTLP gRPC endpoint (default: http://localhost:4317)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Optional: Separate endpoints for traces/metrics
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://localhost:4317

# Sampling configuration
OTEL_TRACE_SAMPLER=always_on  # or: always_off, traceidratio:0.5

# Deployment environment
DEPLOYMENT_ENV=production
```

### Phase 5: Testing and Validation (2-3 hours)

#### Step 5.1: Unit Tests

```bash
cargo test --all-features
```

#### Step 5.2: Integration Tests

**File:** `tests/integration/telemetry_test.rs`

```rust
use llm_policy_engine::observability::tracing::{init_telemetry, shutdown_telemetry};
use opentelemetry::global;
use tracing::{info, warn};

#[tokio::test]
async fn test_telemetry_end_to_end() {
    // Set up test OTLP endpoint
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

    // Initialize
    let provider = init_telemetry().expect("Failed to initialize telemetry");

    // Generate some test spans
    info!("Test log message");
    warn!("Test warning");

    // Create custom span
    let tracer = global::tracer("test");
    let span = tracer.start("test-span");
    span.end();

    // Shutdown gracefully
    drop(provider);
    shutdown_telemetry().await.expect("Shutdown failed");
}
```

#### Step 5.3: Manual Validation

**Step 1: Start OTLP Collector (Docker)**

```bash
# docker-compose.yml
version: '3.8'
services:
  jaeger:
    image: jaegertracing/all-in-one:1.52
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC receiver
      - "4318:4318"    # OTLP HTTP receiver
    environment:
      - COLLECTOR_OTLP_ENABLED=true

# Start
docker-compose up -d
```

**Step 2: Run Policy Engine**

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --bin policy-engine
```

**Step 3: Generate Test Traffic**

```bash
# Make some API calls to generate traces
curl -X POST http://localhost:8080/v1/evaluate \
  -H "Content-Type: application/json" \
  -d '{
    "policy_id": "cost-limit-policy",
    "context": {
      "llm": {"model": "gpt-4", "maxTokens": 1000}
    }
  }'
```

**Step 4: View Traces in Jaeger UI**

```bash
open http://localhost:16686
# Search for service: "llm-policy-engine"
```

**Expected Results:**
- Traces appear in Jaeger UI within 5-10 seconds
- Spans show correct service name, operation names, and attributes
- Trace context propagates correctly across components

---

## 6. Code Changes Required

### 6.1 Cargo.toml - Dependencies Section

**File:** `/Cargo.toml`
**Lines:** 36-40

```diff
 # Logging and tracing
 tracing = "0.1"
 tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
-tracing-opentelemetry = "0.22"
-opentelemetry = { version = "0.21", features = ["rt-tokio"] }
-opentelemetry-jaeger = "0.20"
+tracing-opentelemetry = "0.27"
+opentelemetry = "0.27"
+opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
+opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace", "metrics"] }
```

### 6.2 Tracer Provider Initialization

**File:** `src/observability/tracing.rs` (new file if doesn't exist)

**COMPLETE FILE:**

```rust
//! OpenTelemetry distributed tracing for LLM Policy Engine
//!
//! Provides OTLP-based tracing with Tokio runtime integration.
//! Migrated from deprecated Jaeger exporter (OpenTelemetry 0.21 → 0.27)

use anyhow::{Context, Result};
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::{Config, Sampler, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

/// Configuration for OpenTelemetry tracing
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub deployment_env: String,
    pub otlp_endpoint: String,
    pub sampler: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "llm-policy-engine".into(),
            service_version: env!("CARGO_PKG_VERSION").into(),
            deployment_env: std::env::var("DEPLOYMENT_ENV")
                .unwrap_or_else(|_| "development".into()),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".into()),
            sampler: std::env::var("OTEL_TRACE_SAMPLER")
                .unwrap_or_else(|_| "always_on".into()),
        }
    }
}

/// Initialize OpenTelemetry tracing with OTLP exporter
pub fn init_telemetry() -> Result<SdkTracerProvider> {
    init_telemetry_with_config(TelemetryConfig::default())
}

/// Initialize OpenTelemetry with custom configuration
pub fn init_telemetry_with_config(config: TelemetryConfig) -> Result<SdkTracerProvider> {
    // Build service resource with semantic conventions
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes([
            KeyValue::new("service.version", config.service_version.clone()),
            KeyValue::new("deployment.environment", config.deployment_env.clone()),
            KeyValue::new("service.namespace", "llm-devops"),
            KeyValue::new("telemetry.sdk.name", "opentelemetry"),
            KeyValue::new("telemetry.sdk.language", "rust"),
            KeyValue::new("telemetry.sdk.version", "0.27.0"),
        ])
        .build();

    // Configure sampling strategy
    let sampler = parse_sampler(&config.sampler);

    // Create trace configuration
    let trace_config = Config::default()
        .with_sampler(sampler)
        .with_resource(resource)
        .with_max_events_per_span(128)
        .with_max_attributes_per_span(64)
        .with_max_links_per_span(32);

    tracing::info!(
        "Initializing OpenTelemetry OTLP tracer: endpoint={}, sampler={}",
        config.otlp_endpoint,
        config.sampler
    );

    // Build OTLP exporter pipeline with gRPC transport
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(config.otlp_endpoint)
                .with_timeout(std::time::Duration::from_secs(3)),
        )
        .with_trace_config(trace_config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .context("Failed to install OTLP tracer pipeline")?;

    // Set as global tracer provider for convenience
    global::set_tracer_provider(provider.clone());

    // Initialize tracing subscriber with OpenTelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(config.service_name));

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,llm_policy_engine=debug"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set global tracing subscriber")?;

    tracing::info!("OpenTelemetry tracing initialized successfully");

    Ok(provider)
}

/// Parse sampler configuration from string
fn parse_sampler(sampler_str: &str) -> Sampler {
    match sampler_str {
        "always_on" => Sampler::AlwaysOn,
        "always_off" => Sampler::AlwaysOff,
        s if s.starts_with("traceidratio:") => {
            let ratio = s
                .strip_prefix("traceidratio:")
                .and_then(|r| r.parse::<f64>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            Sampler::TraceIdRatioBased(ratio)
        }
        _ => {
            tracing::warn!("Unknown sampler '{}', defaulting to AlwaysOn", sampler_str);
            Sampler::AlwaysOn
        }
    }
}

/// Gracefully shutdown OpenTelemetry tracer provider
///
/// MUST be called before process exit to ensure all pending spans are flushed
pub async fn shutdown_telemetry() -> Result<()> {
    tracing::info!("Shutting down OpenTelemetry tracer provider");

    // Shutdown is async in 0.27+ and returns Result
    global::shutdown_tracer_provider()
        .await
        .context("Failed to shutdown tracer provider")?;

    tracing::info!("OpenTelemetry shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_lifecycle() {
        let config = TelemetryConfig {
            service_name: "test-service".into(),
            service_version: "0.0.1".into(),
            deployment_env: "test".into(),
            otlp_endpoint: "http://localhost:4317".into(),
            sampler: "always_on".into(),
        };

        let provider = init_telemetry_with_config(config).expect("Init failed");

        // Verify global tracer is set
        let tracer = global::tracer("test");
        let span = tracer.start("test-span");
        span.end();

        // Cleanup
        drop(provider);
        shutdown_telemetry().await.expect("Shutdown failed");
    }

    #[test]
    fn test_sampler_parsing() {
        assert!(matches!(parse_sampler("always_on"), Sampler::AlwaysOn));
        assert!(matches!(parse_sampler("always_off"), Sampler::AlwaysOff));

        match parse_sampler("traceidratio:0.5") {
            Sampler::TraceIdRatioBased(ratio) => assert!((ratio - 0.5).abs() < 0.001),
            _ => panic!("Expected TraceIdRatioBased sampler"),
        }
    }
}
```

### 6.3 Batch Processor Setup

**File:** `src/observability/metrics.rs` (if custom processor needed)

```rust
use opentelemetry_sdk::trace::{BatchSpanProcessor, SpanExporter};
use opentelemetry_sdk::runtime::Tokio;
use std::time::Duration;

/// Create custom batch processor with tuned parameters for high-throughput
pub fn create_batch_processor<E: SpanExporter + 'static>(
    exporter: E,
) -> BatchSpanProcessor<Tokio> {
    BatchSpanProcessor::builder(exporter, Tokio)
        .with_max_queue_size(4096)           // Increased for high throughput
        .with_scheduled_delay(Duration::from_secs(5))  // Export every 5s
        .with_max_export_batch_size(512)     // Batch size
        .with_max_export_timeout(Duration::from_secs(30))
        .build()
}
```

### 6.4 Resource Configuration

**Included in Section 6.2** - See `Resource::builder()` usage in `init_telemetry_with_config()`.

### 6.5 Exporter Configuration (Jaeger → OTLP)

**Environment Variables Configuration:**

**File:** `.env.example` (create or update)

```bash
# OpenTelemetry Configuration
# ============================

# OTLP Exporter Endpoint (gRPC)
# Default: http://localhost:4317
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Alternative: Separate endpoints for traces and metrics
# OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317
# OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://localhost:4317

# Trace Sampling
# Options: always_on, always_off, traceidratio:<ratio>
# Default: always_on
OTEL_TRACE_SAMPLER=always_on

# Deployment Environment
# Used as resource attribute
DEPLOYMENT_ENV=development

# Service Configuration
# SERVICE_NAME=llm-policy-engine  # Defaults to Cargo.toml name
# SERVICE_VERSION=0.1.0           # Defaults to Cargo.toml version
```

**Docker Compose Configuration:**

**File:** `docker-compose.yml` (update or add)

```yaml
version: '3.8'

services:
  policy-engine:
    build: .
    environment:
      - OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
      - DEPLOYMENT_ENV=production
      - OTEL_TRACE_SAMPLER=always_on
    depends_on:
      - jaeger
      - postgres
      - redis

  jaeger:
    image: jaegertracing/all-in-one:1.52
    ports:
      - "16686:16686"   # Jaeger UI
      - "4317:4317"     # OTLP gRPC receiver
      - "4318:4318"     # OTLP HTTP receiver
    environment:
      - COLLECTOR_OTLP_ENABLED=true
      - SPAN_STORAGE_TYPE=memory
    networks:
      - llm-devops

  # ... other services

networks:
  llm-devops:
    driver: bridge
```

### 6.6 Graceful Shutdown

**File:** `src/daemon/main.rs`

```rust
use llm_policy_engine::observability::tracing::{init_telemetry, shutdown_telemetry};
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize telemetry FIRST
    let tracer_provider = init_telemetry()?;

    info!("LLM Policy Engine v{} starting...", env!("CARGO_PKG_VERSION"));

    // Initialize application components
    let engine = PolicyEngine::new().await?;
    let api_server = ApiServer::new(engine.clone()).await?;
    let grpc_server = GrpcServer::new(engine.clone()).await?;

    info!("All services started successfully");

    // Wait for shutdown signal (Ctrl+C or SIGTERM)
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        }
        _ = shutdown_signal() => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
    }

    // Graceful shutdown sequence
    info!("Shutting down API server...");
    api_server.shutdown().await?;

    info!("Shutting down gRPC server...");
    grpc_server.shutdown().await?;

    info!("Shutting down policy engine...");
    engine.shutdown().await?;

    // CRITICAL: Shutdown telemetry LAST to ensure all spans are exported
    info!("Flushing telemetry data...");
    drop(tracer_provider);  // Drop provider first

    if let Err(e) = shutdown_telemetry().await {
        error!("Error during telemetry shutdown: {}", e);
        // Don't fail the shutdown, just log
    }

    info!("Shutdown complete");
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut term = signal(SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");

    term.recv().await;
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    // Windows doesn't have SIGTERM, just wait forever
    std::future::pending::<()>().await;
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests to Update

**Test File:** `tests/unit/observability_test.rs`

```rust
use llm_policy_engine::observability::tracing::{
    init_telemetry_with_config, shutdown_telemetry, TelemetryConfig,
};
use opentelemetry::global;
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use opentelemetry::KeyValue;

#[tokio::test]
async fn test_tracer_initialization() {
    let config = TelemetryConfig {
        service_name: "test-service".into(),
        service_version: "0.0.1".into(),
        deployment_env: "test".into(),
        otlp_endpoint: "http://localhost:4317".into(),
        sampler: "always_on".into(),
    };

    let provider = init_telemetry_with_config(config).expect("Failed to init");

    // Verify tracer is available
    let tracer = global::tracer("test");
    assert!(tracer.tracer_provider().is_some());

    // Cleanup
    drop(provider);
    shutdown_telemetry().await.expect("Failed to shutdown");
}

#[tokio::test]
async fn test_span_creation() {
    let config = TelemetryConfig::default();
    let provider = init_telemetry_with_config(config).unwrap();

    let tracer = global::tracer("test");
    let mut span = tracer
        .span_builder("test-operation")
        .with_attributes(vec![
            KeyValue::new("test.key", "test.value"),
            KeyValue::new("test.id", 123),
        ])
        .start(&tracer);

    span.set_attribute(KeyValue::new("result", "success"));
    span.add_event("test-event", vec![KeyValue::new("event.detail", "info")]);
    span.end();

    drop(provider);
    shutdown_telemetry().await.unwrap();
}

#[test]
fn test_sampler_configurations() {
    use llm_policy_engine::observability::tracing::parse_sampler;

    // Test various sampler configs
    assert!(matches!(parse_sampler("always_on"), Sampler::AlwaysOn));
    assert!(matches!(parse_sampler("always_off"), Sampler::AlwaysOff));

    // Test ratio-based sampling
    match parse_sampler("traceidratio:0.25") {
        Sampler::TraceIdRatioBased(ratio) => {
            assert!((ratio - 0.25).abs() < 0.001);
        }
        _ => panic!("Expected TraceIdRatioBased"),
    }
}
```

### 7.2 Integration Tests to Verify

**Test File:** `tests/integration/otlp_export_test.rs`

```rust
use llm_policy_engine::observability::tracing::{init_telemetry, shutdown_telemetry};
use opentelemetry::global;
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_otlp_export_integration() {
    // Requires running OTLP collector on localhost:4317
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

    let provider = init_telemetry().expect("Failed to initialize telemetry");

    // Create test spans
    let tracer = global::tracer("integration-test");

    for i in 0..10 {
        let mut span = tracer.span_builder(format!("test-span-{}", i)).start(&tracer);
        span.set_attribute(KeyValue::new("iteration", i as i64));
        sleep(Duration::from_millis(100)).await;
        span.end();
    }

    // Allow time for batch export
    sleep(Duration::from_secs(6)).await;

    // Shutdown and verify all spans exported
    drop(provider);
    shutdown_telemetry().await.expect("Shutdown failed");

    println!("✅ Integration test complete - check Jaeger UI for traces");
}
```

**Run Integration Test:**

```bash
# Start Jaeger with OTLP support
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  -p 4318:4318 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:1.52

# Run integration tests
cargo test --ignored test_otlp_export_integration

# Check Jaeger UI
open http://localhost:16686
```

### 7.3 OTLP Export Validation

**Manual Validation Script:**

**File:** `scripts/validate_otlp.sh`

```bash
#!/bin/bash
set -e

echo "🔍 Validating OTLP Export Configuration"

# Check if Jaeger is running
if ! curl -s http://localhost:16686 > /dev/null; then
    echo "❌ Jaeger UI not accessible on http://localhost:16686"
    exit 1
fi
echo "✅ Jaeger UI accessible"

# Check OTLP gRPC port
if ! nc -z localhost 4317; then
    echo "❌ OTLP gRPC port 4317 not open"
    exit 1
fi
echo "✅ OTLP gRPC port 4317 open"

# Build and run policy engine
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export DEPLOYMENT_ENV=test
cargo build --release

echo "🚀 Starting policy engine..."
timeout 30s cargo run --release &
PID=$!

# Wait for startup
sleep 5

# Generate test traffic
echo "📊 Generating test traffic..."
for i in {1..5}; do
    curl -s -X POST http://localhost:8080/v1/evaluate \
        -H "Content-Type: application/json" \
        -d "{\"policy_id\": \"test-$i\", \"context\": {}}" || true
    sleep 1
done

# Allow export time
sleep 10

# Kill policy engine
kill $PID || true

# Query Jaeger API for traces
echo "🔎 Querying Jaeger for traces..."
TRACES=$(curl -s "http://localhost:16686/api/traces?service=llm-policy-engine&limit=10")

if echo "$TRACES" | jq -e '.data | length > 0' > /dev/null; then
    echo "✅ Traces found in Jaeger!"
    echo "$TRACES" | jq -r '.data[0].spans[0].operationName'
else
    echo "❌ No traces found in Jaeger"
    exit 1
fi

echo "✅ OTLP export validation successful"
```

### 7.4 Performance Regression Checks

**Benchmark File:** `benches/telemetry_overhead.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llm_policy_engine::observability::tracing::{init_telemetry, shutdown_telemetry};
use opentelemetry::global;
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use tokio::runtime::Runtime;

fn benchmark_span_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let _provider = rt.block_on(async {
        init_telemetry().expect("Failed to init telemetry")
    });

    c.bench_function("create_and_end_span", |b| {
        let tracer = global::tracer("benchmark");
        b.iter(|| {
            let span = tracer.start("bench-span");
            black_box(span).end();
        });
    });

    rt.block_on(async {
        shutdown_telemetry().await.unwrap();
    });
}

fn benchmark_span_with_attributes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let _provider = rt.block_on(async {
        init_telemetry().expect("Failed to init telemetry")
    });

    c.bench_function("span_with_10_attributes", |b| {
        let tracer = global::tracer("benchmark");
        b.iter(|| {
            let mut span = tracer.start("bench-span-attrs");
            for i in 0..10 {
                span.set_attribute(KeyValue::new(format!("key{}", i), i as i64));
            }
            black_box(span).end();
        });
    });

    rt.block_on(async {
        shutdown_telemetry().await.unwrap();
    });
}

criterion_group!(benches, benchmark_span_creation, benchmark_span_with_attributes);
criterion_main!(benches);
```

**Run Performance Benchmarks:**

```bash
cargo bench --bench telemetry_overhead

# Compare with baseline (if you have saved results)
cargo bench --bench telemetry_overhead -- --baseline main
```

**Expected Performance:**
- Span creation: <100ns overhead
- Span with attributes: <500ns overhead
- Batch export: does not block main thread
- Memory overhead: <10MB for default buffer sizes

---

## 8. Rollback Plan

### 8.1 How to Revert if Issues Found

**Emergency Rollback Steps:**

```bash
# 1. Stop running services
docker-compose down

# 2. Checkout previous working commit
git log --oneline -n 10  # Find commit before upgrade
git checkout <previous-commit-hash>

# 3. Rebuild with old dependencies
cargo clean
cargo build --release

# 4. Restart services
docker-compose up -d

# 5. Verify system health
curl http://localhost:8080/health
```

**Rollback Decision Criteria:**

| Issue | Severity | Action |
|-------|----------|--------|
| Compilation fails | P0 | Immediate rollback |
| Tests fail >50% | P0 | Immediate rollback |
| No traces exported | P1 | Investigate, rollback if not fixed in 2 hours |
| Performance degradation >20% | P1 | Rollback and optimize |
| Memory leak detected | P0 | Immediate rollback |
| Crashes in production | P0 | Immediate rollback |

### 8.2 Git Branch Strategy

```bash
# Create feature branch for upgrade
git checkout -b feat/otel-upgrade-0.27
git push -u origin feat/otel-upgrade-0.27

# Make changes incrementally
git add Cargo.toml Cargo.lock
git commit -m "chore: Update OpenTelemetry dependencies to 0.27"

git add src/observability/
git commit -m "refactor: Migrate to OTLP exporter and new OTel API"

git add tests/
git commit -m "test: Update telemetry tests for OTel 0.27"

# Create PR
gh pr create --title "feat: Upgrade OpenTelemetry 0.21 → 0.27" \
  --body "See POLICY_ENGINE_UPGRADE_SPECIFICATION.md for details"

# After review and approval
git checkout main
git merge --no-ff feat/otel-upgrade-0.27
git tag -a v0.2.0 -m "OpenTelemetry 0.27 upgrade"
git push origin main --tags
```

### 8.3 Deployment Rollback Procedure

**Kubernetes Rollback:**

```bash
# Check deployment history
kubectl rollout history deployment/policy-engine -n llm-devops

# Rollback to previous revision
kubectl rollout undo deployment/policy-engine -n llm-devops

# Rollback to specific revision
kubectl rollout undo deployment/policy-engine --to-revision=3 -n llm-devops

# Verify rollback
kubectl rollout status deployment/policy-engine -n llm-devops
```

**Docker Compose Rollback:**

```bash
# Use specific image tag
# docker-compose.yml
services:
  policy-engine:
    image: llm-policy-engine:v0.1.0  # Previous stable version

# Restart
docker-compose down
docker-compose up -d
```

**Cargo.lock Preservation:**

```bash
# Before upgrade, save Cargo.lock
cp Cargo.lock Cargo.lock.backup

# If rollback needed
cp Cargo.lock.backup Cargo.lock
cargo build --locked  # Use exact versions from backup
```

---

## 9. Infrastructure Requirements

### 9.1 OTLP Collector Configuration

**Option 1: Direct to Jaeger (Simplest)**

```yaml
# docker-compose.yml
version: '3.8'
services:
  jaeger:
    image: jaegertracing/all-in-one:1.52
    environment:
      - COLLECTOR_OTLP_ENABLED=true
      - SPAN_STORAGE_TYPE=elasticsearch  # or badger, memory
      - ES_SERVER_URLS=http://elasticsearch:9200
    ports:
      - "16686:16686"   # Jaeger UI
      - "4317:4317"     # OTLP gRPC
      - "4318:4318"     # OTLP HTTP
      - "14250:14250"   # gRPC collector (legacy)
    networks:
      - llm-devops
```

**Option 2: OpenTelemetry Collector (Production)**

```yaml
# docker-compose.yml
version: '3.8'
services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:0.91.0
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
    ports:
      - "4317:4317"     # OTLP gRPC receiver
      - "4318:4318"     # OTLP HTTP receiver
      - "8888:8888"     # Prometheus metrics (collector itself)
      - "13133:13133"   # Health check
    networks:
      - llm-devops

  jaeger:
    image: jaegertracing/all-in-one:1.52
    ports:
      - "16686:16686"   # UI
      - "14250:14250"   # gRPC collector
    networks:
      - llm-devops
```

**OpenTelemetry Collector Config:**

**File:** `otel-collector-config.yaml`

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 10s
    send_batch_size: 1024

  memory_limiter:
    check_interval: 1s
    limit_mib: 512

  # Add resource attributes
  resource:
    attributes:
      - key: environment
        value: production
        action: insert

exporters:
  # Export to Jaeger
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  # Export to Prometheus (for metrics)
  prometheus:
    endpoint: "0.0.0.0:8889"

  # Logging for debugging
  logging:
    loglevel: info

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [jaeger, logging]

    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [prometheus, logging]

  telemetry:
    metrics:
      address: 0.0.0.0:8888
```

### 9.2 Jaeger Backend Compatibility (OTLP Ingestion)

**Jaeger Version Requirements:**
- **Minimum:** Jaeger 1.35+ (native OTLP support)
- **Recommended:** Jaeger 1.52+ (latest stable)

**OTLP Ingestion Modes:**

| Mode | Port | Protocol | Use Case |
|------|------|----------|----------|
| gRPC | 4317 | OTLP/gRPC | Default, best performance |
| HTTP | 4318 | OTLP/HTTP | Firewall-friendly, proxies |

**Jaeger Environment Variables:**

```bash
# Enable OTLP receiver
COLLECTOR_OTLP_ENABLED=true

# Storage backend
SPAN_STORAGE_TYPE=elasticsearch  # or: memory, badger, cassandra

# Elasticsearch configuration (if used)
ES_SERVER_URLS=http://elasticsearch:9200
ES_INDEX_PREFIX=jaeger

# Sampling configuration
SAMPLING_STRATEGIES_FILE=/etc/jaeger/sampling.json
```

**Storage Backend Options:**

| Backend | Pros | Cons | Use Case |
|---------|------|------|----------|
| **Memory** | Fast, no setup | Data loss on restart | Development only |
| **Badger** | Embedded, simple | Limited scalability | Small deployments |
| **Elasticsearch** | Scalable, queryable | Complex setup | Production (recommended) |
| **Cassandra** | High throughput | Operational overhead | Very large scale |

### 9.3 Port Changes

**Before (Jaeger Direct):**
```
Policy Engine → Jaeger Agent UDP (6831)
Policy Engine → Jaeger Collector HTTP (14268)
```

**After (OTLP):**
```
Policy Engine → OTLP Collector gRPC (4317)
                 └─→ Jaeger Backend

OR

Policy Engine → Jaeger OTLP gRPC (4317) [Direct]
```

**Firewall Rules Update:**

```bash
# Allow OTLP gRPC (recommended)
sudo ufw allow 4317/tcp comment "OTLP gRPC"

# Allow OTLP HTTP (optional)
sudo ufw allow 4318/tcp comment "OTLP HTTP"

# Remove old Jaeger ports (if not using legacy clients)
# sudo ufw delete allow 6831/udp  # Jaeger agent
# sudo ufw delete allow 14268/tcp # Jaeger collector
```

**Kubernetes NetworkPolicy:**

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: policy-engine-otlp-egress
  namespace: llm-devops
spec:
  podSelector:
    matchLabels:
      app: policy-engine
  policyTypes:
    - Egress
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: otel-collector
      ports:
        - protocol: TCP
          port: 4317  # OTLP gRPC
```

### 9.4 Environment Variables

**File:** `.env.production`

```bash
# ===================================
# OpenTelemetry Configuration
# ===================================

# OTLP Exporter Endpoint (gRPC)
# Production: Points to OTLP collector or Jaeger
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317

# Alternative: HTTP endpoint
# OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4318

# Separate endpoints for different signals (optional)
# OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://otel-collector:4317
# OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://otel-collector:4317

# Trace Sampling Strategy
# Options: always_on, always_off, traceidratio:<0.0-1.0>
OTEL_TRACE_SAMPLER=traceidratio:0.1  # Sample 10% in production

# Service Configuration
# SERVICE_NAME=llm-policy-engine  # Defaults from code
# SERVICE_VERSION=0.1.0           # Defaults from code

# Deployment Environment
DEPLOYMENT_ENV=production

# OTLP Export Timeout (seconds)
# OTEL_EXPORTER_OTLP_TIMEOUT=30

# TLS Configuration (if using secure endpoints)
# OTEL_EXPORTER_OTLP_CERTIFICATE=/path/to/ca.crt
# OTEL_EXPORTER_OTLP_CLIENT_CERTIFICATE=/path/to/client.crt
# OTEL_EXPORTER_OTLP_CLIENT_KEY=/path/to/client.key

# Batch Processor Configuration (advanced)
# OTEL_BSP_MAX_QUEUE_SIZE=4096
# OTEL_BSP_MAX_EXPORT_BATCH_SIZE=512
# OTEL_BSP_SCHEDULE_DELAY=5000  # milliseconds

# ===================================
# Application Configuration
# ===================================
LOG_LEVEL=info
DATABASE_URL=postgresql://user:pass@postgres:5432/policy_engine
REDIS_URL=redis://redis:6379
```

**Kubernetes ConfigMap:**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: policy-engine-config
  namespace: llm-devops
data:
  OTEL_EXPORTER_OTLP_ENDPOINT: "http://otel-collector.llm-devops.svc.cluster.local:4317"
  OTEL_TRACE_SAMPLER: "traceidratio:0.1"
  DEPLOYMENT_ENV: "production"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: policy-engine
  namespace: llm-devops
spec:
  template:
    spec:
      containers:
        - name: policy-engine
          image: llm-policy-engine:v0.2.0
          envFrom:
            - configMapRef:
                name: policy-engine-config
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: policy-engine-secrets
                  key: database-url
```

---

## 10. Success Criteria

### 10.1 Compilation Succeeds

**Verification:**

```bash
# Clean build
cargo clean
cargo build --all-features --release

# Expected output:
#    Compiling opentelemetry v0.27.0
#    Compiling opentelemetry_sdk v0.27.0
#    Compiling opentelemetry-otlp v0.27.0
#    Compiling llm-policy-engine v0.1.0
#    Finished release [optimized] target(s) in 2m 34s

# Verify binary created
ls -lh target/release/policy-engine
# Expected: ~20MB binary
```

**Acceptance Criteria:**
- ✅ No compilation errors
- ✅ No dependency resolution conflicts
- ✅ All features compile successfully
- ✅ Binary size within expected range (<50MB)

### 10.2 All Tests Pass

**Verification:**

```bash
# Unit tests
cargo test --lib
# Expected: 100% pass rate

# Integration tests
cargo test --test '*'
# Expected: 100% pass rate

# Doc tests
cargo test --doc
# Expected: All documentation examples compile and run

# Full test suite
cargo test --all-features
# Expected output:
# test result: ok. 87 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Acceptance Criteria:**
- ✅ All unit tests pass (87/87)
- ✅ All integration tests pass
- ✅ All documentation tests pass
- ✅ No flaky tests
- ✅ Test coverage >80%

### 10.3 OTLP Export Working

**Verification:**

```bash
# 1. Start Jaeger with OTLP support
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:1.52

# 2. Run policy engine
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --release

# 3. Generate test traffic
curl -X POST http://localhost:8080/v1/evaluate \
  -H "Content-Type: application/json" \
  -d '{"policy_id": "test", "context": {}}'

# 4. Wait for batch export (5-10 seconds)
sleep 10

# 5. Query Jaeger API
curl -s "http://localhost:16686/api/traces?service=llm-policy-engine" | jq .

# Expected: JSON response with traces
```

**Acceptance Criteria:**
- ✅ Traces appear in Jaeger UI within 10 seconds
- ✅ Service name is "llm-policy-engine"
- ✅ Spans contain correct operation names
- ✅ Span attributes include service.version, deployment.environment
- ✅ Trace context propagates correctly
- ✅ No export errors in logs

**Jaeger UI Verification:**

1. Open http://localhost:16686
2. Select service: "llm-policy-engine"
3. Click "Find Traces"
4. Verify:
   - ✅ Traces appear in results
   - ✅ Span hierarchy is correct
   - ✅ Timestamps are accurate
   - ✅ Tags/attributes are present
   - ✅ Logs/events are attached

### 10.4 No Performance Degradation

**Baseline Metrics (Before Upgrade):**
```
Policy Evaluation P50: 2ms
Policy Evaluation P99: 8ms
Throughput: 10,000 req/s
Memory Usage: 150MB
CPU Usage: 25%
```

**Verification:**

```bash
# Run performance benchmarks
cargo bench --bench policy_evaluation
cargo bench --bench telemetry_overhead

# Load test with wrk
wrk -t4 -c100 -d30s --latency \
  -s benchmark.lua \
  http://localhost:8080/v1/evaluate

# Expected output:
# Latency Distribution
#   50%    2.5ms   (acceptable: <3ms)
#   99%    9.0ms   (acceptable: <10ms)
# Requests/sec: 9,500 (acceptable: >9,000)
```

**Acceptance Criteria:**
- ✅ P50 latency increase <10% (target: <2.2ms)
- ✅ P99 latency increase <10% (target: <8.8ms)
- ✅ Throughput decrease <5% (target: >9,500 req/s)
- ✅ Memory overhead <5% (target: <160MB)
- ✅ CPU overhead <5% (target: <27%)

**Memory Leak Check:**

```bash
# Run for extended period
timeout 3600 cargo run --release &  # Run for 1 hour
PID=$!

# Monitor memory usage
while kill -0 $PID 2>/dev/null; do
    ps -p $PID -o rss,vsz,%mem,cmd
    sleep 60
done

# Expected: Memory usage stable over time (no continuous growth)
```

### 10.5 Edge-Agent Integration Unblocked

**Verification:**

```bash
# Clone Edge-Agent repository
git clone https://github.com/LLM-Dev-Ops/edge-agent /tmp/edge-agent
cd /tmp/edge-agent

# Update dependency to use upgraded policy-engine
# Cargo.toml:
# llm-policy-engine = { git = "https://github.com/LLM-Dev-Ops/policy-engine", branch = "feat/otel-upgrade-0.27" }

# Build Edge-Agent with updated dependency
cargo build --all-features

# Expected: ✅ Compilation succeeds
```

**Integration Test:**

```bash
# Run Edge-Agent integration tests
cd /tmp/edge-agent
cargo test --test integration_test

# Expected: All tests pass
```

**Acceptance Criteria:**
- ✅ Edge-Agent compiles with upgraded policy-engine
- ✅ All Edge-Agent tests pass
- ✅ Phase 2B integration tasks can proceed
- ✅ No new dependency conflicts
- ✅ Shared telemetry context works correctly

---

## Appendix A: Troubleshooting Guide

### Common Issues and Solutions

#### Issue 1: Compilation Error - "feature `rt-tokio` not found"

**Error:**
```
error: failed to select a version for `opentelemetry`.
the package `llm-policy-engine` depends on `opentelemetry`, with features: `rt-tokio`
but `opentelemetry` does not have these features.
```

**Solution:**
```toml
# ❌ WRONG
opentelemetry = { version = "0.27", features = ["rt-tokio"] }

# ✅ CORRECT
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
```

#### Issue 2: Runtime Error - "cannot find `runtime` in module `opentelemetry`"

**Error:**
```rust
error[E0433]: failed to resolve: could not find `runtime` in `opentelemetry`
  --> src/observability/tracing.rs:45:32
   |
45 |     .install_batch(opentelemetry::runtime::Tokio)?;
   |                                    ^^^^^^^ could not find `runtime` in `opentelemetry`
```

**Solution:**
```rust
// ❌ WRONG
use opentelemetry::runtime::Tokio;

// ✅ CORRECT
use opentelemetry_sdk::runtime::Tokio;
```

#### Issue 3: No Traces in Jaeger

**Symptoms:**
- Application runs without errors
- Logs show "OpenTelemetry initialized successfully"
- No traces appear in Jaeger UI

**Diagnosis:**
```bash
# Check OTLP endpoint connectivity
curl -v http://localhost:4317

# Check Jaeger OTLP receiver status
docker logs jaeger 2>&1 | grep -i otlp

# Check application logs for export errors
cargo run 2>&1 | grep -i "export\|otlp\|error"
```

**Solutions:**

1. **Endpoint Misconfiguration:**
```bash
# Verify environment variable
echo $OTEL_EXPORTER_OTLP_ENDPOINT
# Should be: http://localhost:4317 (not 4318 for gRPC)

# Check Docker networking
docker network inspect bridge | jq '.[] | .Containers'
```

2. **Sampling Configuration:**
```bash
# Temporarily set to always sample
export OTEL_TRACE_SAMPLER=always_on
```

3. **Batch Export Delay:**
```bash
# Wait longer for batch export (default: 5 seconds)
sleep 10

# Or trigger graceful shutdown to force flush
kill -SIGTERM <pid>
```

#### Issue 4: Performance Degradation

**Symptoms:**
- Latency increased >20%
- CPU usage increased
- Memory usage growing

**Diagnosis:**
```bash
# Profile with perf
cargo build --release --features profiling
perf record -g ./target/release/policy-engine
perf report

# Check batch processor configuration
# Look for excessive batching or queue buildup
```

**Solutions:**

1. **Tune Batch Processor:**
```rust
BatchSpanProcessor::builder(exporter, Tokio)
    .with_max_queue_size(2048)         // Reduce if memory issues
    .with_scheduled_delay(Duration::from_secs(10))  // Increase delay
    .with_max_export_batch_size(256)   // Reduce batch size
    .build()
```

2. **Adjust Sampling:**
```bash
# Sample only 10% of traces in production
export OTEL_TRACE_SAMPLER=traceidratio:0.1
```

3. **Disable Telemetry (Emergency):**
```bash
export OTEL_TRACE_SAMPLER=always_off
```

---

## Appendix B: References and Resources

### Official Documentation

- **OpenTelemetry Rust:** https://github.com/open-telemetry/opentelemetry-rust
- **OpenTelemetry Specification:** https://opentelemetry.io/docs/specs/otel/
- **OTLP Specification:** https://github.com/open-telemetry/opentelemetry-proto
- **Jaeger OTLP Support:** https://www.jaegertracing.io/docs/latest/apis/#opentelemetry-protocol-stable

### Migration Guides

- **OpenTelemetry 0.21 Release Notes:** https://github.com/open-telemetry/opentelemetry-rust/releases/tag/v0.21.0
- **OpenTelemetry 0.27 Release Notes:** https://github.com/open-telemetry/opentelemetry-rust/releases/tag/v0.27.0
- **Jaeger Exporter Deprecation:** https://github.com/open-telemetry/opentelemetry-rust/issues/1

### Related Issues

- Edge-Agent Phase 2B: https://github.com/LLM-Dev-Ops/edge-agent/issues/XXX
- Policy-Engine Observability: https://github.com/LLM-Dev-Ops/policy-engine/issues/XXX

---

## Appendix C: Checklist

### Pre-Migration Checklist

- [ ] Backup current Cargo.lock
- [ ] Document current performance metrics
- [ ] Create feature branch `feat/otel-upgrade-0.27`
- [ ] Set up Jaeger with OTLP support locally
- [ ] Review breaking changes documentation

### Migration Execution Checklist

- [ ] Update Cargo.toml dependencies
- [ ] Run `cargo update` for OpenTelemetry crates
- [ ] Update import statements in all affected files
- [ ] Replace Jaeger exporter with OTLP
- [ ] Update tracer provider initialization
- [ ] Update shutdown logic (async)
- [ ] Update environment variables
- [ ] Update Docker Compose configuration
- [ ] Run `cargo check --all-features`
- [ ] Run `cargo clippy --all-features`
- [ ] Fix all compilation errors

### Testing Checklist

- [ ] All unit tests pass (`cargo test --lib`)
- [ ] All integration tests pass (`cargo test --test '*'`)
- [ ] Manual OTLP export verification
- [ ] Traces appear in Jaeger UI
- [ ] Performance benchmarks run
- [ ] No performance regression >10%
- [ ] Load testing completed
- [ ] Memory leak check passed

### Documentation Checklist

- [ ] Update README.md with new OTLP configuration
- [ ] Update DEPLOYMENT_GUIDE.md
- [ ] Update environment variable documentation
- [ ] Add troubleshooting section
- [ ] Update CHANGELOG.md

### Deployment Checklist

- [ ] Create pull request
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Staging deployment successful
- [ ] Production deployment plan reviewed
- [ ] Rollback plan tested
- [ ] Edge-Agent integration verified
- [ ] Production monitoring configured

---

## Document Change Log

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-12-04 | Edge-Agent Team | Initial comprehensive specification |

---

**End of Specification**
