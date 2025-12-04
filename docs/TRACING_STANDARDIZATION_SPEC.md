# LLM DevOps Unified Tracing Configuration Standard

**Version:** 1.0.0
**Date:** December 4, 2025
**Status:** Active Standard
**Scope:** All 6 LLM DevOps repositories

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Service Naming Convention](#service-naming-convention)
3. [Environment Metadata Standard](#environment-metadata-standard)
4. [OpenTelemetry Schema Assumptions](#opentelemetry-schema-assumptions)
5. [Exporter Configuration Standard](#exporter-configuration-standard)
6. [Tracing Initialization Pattern](#tracing-initialization-pattern)
7. [Context Propagation Standard](#context-propagation-standard)
8. [Per-Repository Configuration](#per-repository-configuration)
9. [Environment Variables Standard](#environment-variables-standard)
10. [Migration Guide](#migration-guide)
11. [Testing and Validation](#testing-and-validation)

---

## Executive Summary

This document defines the unified tracing configuration standard for all LLM DevOps repositories:

1. **edge-agent** - Main intercepting proxy
2. **shield** - Security scanning (15 crates)
3. **sentinel** - Threat detection (6 crates)
4. **connector-hub** - Provider integration
5. **observatory** - Observability platform (10 crates)
6. **cost-ops** - Cost management (5 crates)
7. **policy-engine** - Policy enforcement

### Core Principles

- **Consistency**: All services use identical attribute names and conventions
- **Interoperability**: Traces flow seamlessly across service boundaries
- **Observability**: Rich context propagation for debugging and analysis
- **Performance**: Minimal overhead with configurable sampling
- **Flexibility**: Support for multiple backend exporters

---

## Service Naming Convention

### Standard Pattern

All services MUST use the following naming convention:

```
llm.{component}[.{subcomponent}]
```

### Naming Rules

1. **Namespace**: Always start with `llm.`
2. **Component**: Primary repository/service name (lowercase, hyphen-separated)
3. **Subcomponent**: Optional for multi-crate workspaces
4. **Separators**: Use dots (`.`) only, never hyphens in service names

### Repository Service Names

| Repository | Service Name | Subcomponents (if any) |
|-----------|-------------|------------------------|
| edge-agent | `llm.edge-agent` | `llm.edge-agent.proxy`, `llm.edge-agent.cache`, `llm.edge-agent.routing` |
| shield | `llm.shield` | `llm.shield.core`, `llm.shield.scanner`, `llm.shield.detector` |
| sentinel | `llm.sentinel` | `llm.sentinel.core`, `llm.sentinel.processor`, `llm.sentinel.alerting` |
| connector-hub | `llm.connector-hub` | `llm.connector-hub.registry`, `llm.connector-hub.adapters` |
| observatory | `llm.observatory` | `llm.observatory.collector`, `llm.observatory.api`, `llm.observatory.storage` |
| cost-ops | `llm.cost-ops` | `llm.cost-ops.core`, `llm.cost-ops.api`, `llm.cost-ops.compliance` |
| policy-engine | `llm.policy-engine` | `llm.policy-engine.cel`, `llm.policy-engine.wasm` |

### Examples

```rust
// Main edge-agent service
KeyValue::new("service.name", "llm.edge-agent")

// Edge-agent proxy subcomponent
KeyValue::new("service.name", "llm.edge-agent.proxy")

// Shield scanner subcomponent
KeyValue::new("service.name", "llm.shield.scanner")

// Observatory collector
KeyValue::new("service.name", "llm.observatory.collector")
```

### Anti-Patterns (DO NOT USE)

```rust
// ❌ Wrong: Using old namespace
KeyValue::new("service.name", "llm-edge-agent")

// ❌ Wrong: Inconsistent format
KeyValue::new("service.name", "llm-devops.edge-agent")

// ❌ Wrong: Using hyphens in service name
KeyValue::new("service.name", "llm.edge-agent-proxy")

// ❌ Wrong: Missing namespace
KeyValue::new("service.name", "edge-agent")
```

---

## Environment Metadata Standard

### Required Attributes

All services MUST include these OpenTelemetry resource attributes:

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `service.name` | String | Service identifier (see naming convention) | `llm.edge-agent` |
| `service.version` | String | Semantic version from Cargo.toml | `0.1.0` |
| `deployment.environment` | String | Deployment environment | `production`, `staging`, `development` |

### Recommended Attributes

All services SHOULD include these attributes when available:

| Attribute | Type | Description | Example | Environment Variable |
|-----------|------|-------------|---------|---------------------|
| `service.namespace` | String | Logical namespace | `llm-devops` | `SERVICE_NAMESPACE` |
| `service.instance.id` | String | Unique instance identifier | `edge-agent-pod-a1b2c3` | `SERVICE_INSTANCE_ID` |
| `host.name` | String | Hostname | `edge-agent-01.prod` | `HOSTNAME` |
| `host.id` | String | Host unique identifier | `i-0a1b2c3d4e5f6` | N/A (auto-detected) |

### Kubernetes-Specific Attributes

For Kubernetes deployments, include:

| Attribute | Type | Description | Example | Environment Variable |
|-----------|------|-------------|---------|---------------------|
| `k8s.namespace.name` | String | Kubernetes namespace | `llm-prod` | `K8S_NAMESPACE` |
| `k8s.pod.name` | String | Pod name | `edge-agent-7f8c9d-abc12` | `K8S_POD_NAME` |
| `k8s.pod.uid` | String | Pod UID | `a1b2c3d4-e5f6-7890-abcd-ef1234567890` | `K8S_POD_UID` |
| `k8s.deployment.name` | String | Deployment name | `edge-agent` | `K8S_DEPLOYMENT_NAME` |
| `k8s.cluster.name` | String | Cluster name | `llm-prod-us-east-1` | `K8S_CLUSTER_NAME` |

### Cloud Provider Attributes

For cloud deployments, include:

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `cloud.provider` | String | Cloud provider | `aws`, `gcp`, `azure` |
| `cloud.region` | String | Cloud region | `us-east-1` |
| `cloud.availability_zone` | String | Availability zone | `us-east-1a` |
| `cloud.account.id` | String | Cloud account ID | `123456789012` |

### Code Example

```rust
use opentelemetry::{KeyValue, Resource};
use std::env;

fn create_resource(service_name: &str) -> Resource {
    let mut attributes = vec![
        // Required attributes
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION").to_string()),
        KeyValue::new(
            "deployment.environment",
            env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        ),

        // Recommended attributes
        KeyValue::new("service.namespace", "llm-devops"),
    ];

    // Add optional attributes if available
    if let Ok(instance_id) = env::var("SERVICE_INSTANCE_ID") {
        attributes.push(KeyValue::new("service.instance.id", instance_id));
    }

    if let Ok(hostname) = env::var("HOSTNAME") {
        attributes.push(KeyValue::new("host.name", hostname));
    }

    // Kubernetes attributes (if running in k8s)
    if let Ok(pod_name) = env::var("K8S_POD_NAME") {
        attributes.push(KeyValue::new("k8s.pod.name", pod_name));
    }

    if let Ok(namespace) = env::var("K8S_NAMESPACE") {
        attributes.push(KeyValue::new("k8s.namespace.name", namespace));
    }

    if let Ok(cluster) = env::var("K8S_CLUSTER_NAME") {
        attributes.push(KeyValue::new("k8s.cluster.name", cluster));
    }

    Resource::new(attributes)
}
```

---

## OpenTelemetry Schema Assumptions

### OpenTelemetry Version Standard

- **OpenTelemetry API**: `0.26.x` (minimum)
- **OpenTelemetry SDK**: `0.26.x` (minimum)
- **Semantic Conventions**: OpenTelemetry Semantic Conventions v1.27.0+
- **Protocol**: OTLP/gRPC (primary), OTLP/HTTP (fallback)

### Dependency Alignment

All repositories MUST align on these OpenTelemetry crate versions:

```toml
[workspace.dependencies]
opentelemetry = "0.26"
opentelemetry-sdk = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["trace", "grpc-tonic"] }
opentelemetry-semantic-conventions = "0.26"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.26"
```

### Span Naming Conventions

#### HTTP Spans

Follow OpenTelemetry HTTP semantic conventions:

```rust
// Format: {http.request.method} {http.route}
span_name = format!("{} {}", method, route);

// Examples:
"GET /v1/chat/completions"
"POST /v1/embeddings"
"GET /health"

// Required attributes:
KeyValue::new("http.request.method", "GET"),
KeyValue::new("http.route", "/v1/chat/completions"),
KeyValue::new("http.response.status_code", 200),
KeyValue::new("url.scheme", "https"),
KeyValue::new("server.address", "api.openai.com"),
KeyValue::new("server.port", 443),
```

#### gRPC Spans

Follow OpenTelemetry gRPC semantic conventions:

```rust
// Format: {rpc.service}/{rpc.method}
span_name = format!("{}/{}", service, method);

// Example:
"llm.shield.Scanner/Scan"

// Required attributes:
KeyValue::new("rpc.system", "grpc"),
KeyValue::new("rpc.service", "llm.shield.Scanner"),
KeyValue::new("rpc.method", "Scan"),
KeyValue::new("rpc.grpc.status_code", 0),
```

#### Database Spans

Follow OpenTelemetry database semantic conventions:

```rust
// Format: {db.operation} {db.name}.{db.collection.name}
span_name = format!("{} {}.{}", operation, db_name, collection);

// Examples:
"SELECT cache.responses"
"INSERT cost_ops.transactions"

// Required attributes:
KeyValue::new("db.system", "redis"),
KeyValue::new("db.operation", "GET"),
KeyValue::new("db.namespace", "cache"),
KeyValue::new("db.query.text", "GET prompt:abc123"),
```

#### LLM-Specific Spans

Custom semantic conventions for LLM operations:

```rust
// Span names
"llm.completion"
"llm.embedding"
"llm.cache.lookup"
"llm.provider.request"
"llm.security.scan"
"llm.cost.calculate"

// LLM-specific attributes (following gen_ai.* convention)
KeyValue::new("gen_ai.system", "openai"),
KeyValue::new("gen_ai.request.model", "gpt-4"),
KeyValue::new("gen_ai.request.max_tokens", 1000),
KeyValue::new("gen_ai.request.temperature", 0.7),
KeyValue::new("gen_ai.response.finish_reason", "stop"),
KeyValue::new("gen_ai.usage.input_tokens", 50),
KeyValue::new("gen_ai.usage.output_tokens", 120),

// Custom LLM attributes (llm.* namespace)
KeyValue::new("llm.provider.name", "openai"),
KeyValue::new("llm.provider.endpoint", "https://api.openai.com"),
KeyValue::new("llm.request_id", "req-abc123"),
KeyValue::new("llm.cache.hit", true),
KeyValue::new("llm.cache.tier", "l1"),
KeyValue::new("llm.cost.input_cost_usd", 0.0015),
KeyValue::new("llm.cost.output_cost_usd", 0.0036),
```

### Metric Naming Conventions

Follow OpenTelemetry metric naming best practices:

#### HTTP Metrics

```rust
// Request duration histogram
"http.server.request.duration" // unit: seconds
"http.client.request.duration" // unit: seconds

// Request size distribution
"http.server.request.body.size" // unit: bytes
"http.server.response.body.size" // unit: bytes

// Active requests gauge
"http.server.active_requests" // unit: {request}
```

#### LLM-Specific Metrics

```rust
// Token usage
"llm.tokens.usage" // unit: {token}
  // attributes: type={input|output}, model, provider

// Request latency
"llm.request.duration" // unit: seconds
  // attributes: provider, model, status

// Cache metrics
"llm.cache.hits" // unit: {hit}
"llm.cache.misses" // unit: {miss}
"llm.cache.hit_ratio" // unit: 1 (ratio)

// Cost metrics
"llm.cost.total" // unit: USD
  // attributes: provider, model, cost_type={input|output}

// Provider health
"llm.provider.availability" // unit: 1 (ratio)
  // attributes: provider, endpoint
```

---

## Exporter Configuration Standard

### Default OTLP Endpoint

All services MUST use this default OTLP endpoint configuration:

```
OTLP_ENDPOINT=http://localhost:4317 (gRPC)
OTLP_ENDPOINT_HTTP=http://localhost:4318 (HTTP)
```

### Supported Exporters

#### Primary: OTLP (OpenTelemetry Protocol)

**Configuration:**

```rust
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime;

let exporter = opentelemetry_otlp::new_exporter()
    .tonic() // gRPC transport
    .with_endpoint(endpoint) // from OTLP_ENDPOINT env var
    .with_timeout(std::time::Duration::from_secs(10));

let trace_provider = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(exporter)
    .with_trace_config(trace_config)
    .install_batch(runtime::Tokio)?;
```

**Features:**

- gRPC protocol (primary)
- HTTP protocol (fallback)
- Batch export (default)
- Compression: gzip (optional)
- Retry: exponential backoff

#### Fallback: Stdout (Development)

For local development and debugging:

```rust
use opentelemetry_stdout::SpanExporter;

let exporter = SpanExporter::default();
```

#### Optional: Jaeger (Direct)

For Jaeger-specific deployments:

```rust
use opentelemetry_jaeger::Propagator;

// Use OTLP to Jaeger instead (recommended)
// Jaeger native exporter is deprecated
```

### Export Strategy

#### Batch Export (Production)

**Default configuration:**

```rust
use opentelemetry_sdk::trace::{BatchConfig, BatchSpanProcessor};

let batch_config = BatchConfig::default()
    .with_max_queue_size(2048)
    .with_max_export_batch_size(512)
    .with_scheduled_delay(std::time::Duration::from_millis(5000))
    .with_max_export_timeout(std::time::Duration::from_secs(30));

let batch_processor = BatchSpanProcessor::builder(exporter, runtime::Tokio)
    .with_batch_config(batch_config)
    .build();
```

**Tuning parameters:**

| Parameter | Default | Production | Low-Latency | High-Throughput |
|-----------|---------|-----------|-------------|-----------------|
| `max_queue_size` | 2048 | 2048 | 1024 | 4096 |
| `max_export_batch_size` | 512 | 512 | 256 | 1024 |
| `scheduled_delay` | 5000ms | 5000ms | 1000ms | 10000ms |
| `max_export_timeout` | 30s | 30s | 10s | 60s |

#### Simple Export (Development)

For debugging and development:

```rust
use opentelemetry_sdk::trace::SimpleSpanProcessor;

let simple_processor = SimpleSpanProcessor::new(Box::new(exporter));
```

⚠️ **Warning**: Simple export adds significant latency and should NOT be used in production.

### Sampling Strategy

#### Default Sample Rate

```rust
use opentelemetry_sdk::trace::{Sampler, SamplingDecision};

// Production: 1.0 (100% sampling) by default
// Can be reduced based on traffic volume
let sampler = Sampler::TraceIdRatioBased(1.0);
```

#### Environment-Based Sampling

```rust
fn get_sampler() -> Sampler {
    let sampling_ratio = std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);

    if sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(sampling_ratio)
    }
}
```

#### Advanced: Parent-Based Sampling

```rust
use opentelemetry_sdk::trace::Sampler;

// Respect parent sampling decision, fall back to TraceIdRatioBased
let sampler = Sampler::ParentBased(Box::new(
    Sampler::TraceIdRatioBased(0.1) // 10% sampling for root spans
));
```

### Error Handling

All exporters MUST implement graceful error handling:

```rust
match opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(exporter)
    .install_batch(runtime::Tokio)
{
    Ok(provider) => {
        global::set_tracer_provider(provider);
        tracing::info!("OTLP exporter configured successfully");
    }
    Err(e) => {
        tracing::error!(error = %e, "Failed to initialize OTLP exporter");
        tracing::warn!("Continuing without OTLP export - traces will be lost");
        // Optionally fall back to stdout exporter
    }
}
```

---

## Tracing Initialization Pattern

### Standard Bootstrap Sequence

All services MUST follow this initialization order:

```
1. Load Configuration
2. Create Resource (service metadata)
3. Configure Sampler
4. Initialize Exporter (OTLP/Jaeger/Stdout)
5. Build TracerProvider
6. Set Global TracerProvider
7. Create Tracing Subscriber Layers
8. Initialize Subscriber (with OpenTelemetry layer)
9. Register Shutdown Handler
```

### Reference Implementation

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub otlp_endpoint: Option<String>,
    pub sampling_ratio: f64,
    pub json_logs: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: env!("CARGO_PKG_NAME").replace('-', "."),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
            sampling_ratio: std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
            json_logs: std::env::var("LOG_FORMAT")
                .map(|v| v == "json")
                .unwrap_or(false),
        }
    }
}

/// Initialize OpenTelemetry tracing
pub fn init_tracing(config: TracingConfig) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        "Initializing OpenTelemetry tracing"
    );

    // 1. Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
        KeyValue::new("service.namespace", "llm-devops"),
    ]);

    // 2. Configure sampler
    let sampler = if config.sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };

    // 3. Set up OTLP exporter if endpoint is configured
    if let Some(endpoint) = config.otlp_endpoint {
        tracing::info!(endpoint = %endpoint, "Configuring OTLP exporter");

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&endpoint),
            )
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(sampler.clone())
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(resource.clone()),
            )
            .install_batch(runtime::Tokio)
        {
            Ok(provider) => {
                global::set_tracer_provider(provider);
                tracing::info!("OTLP exporter configured successfully");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialize OTLP exporter");
                tracing::warn!("Continuing without OTLP export");
            }
        }
    } else {
        tracing::warn!("No OTLP endpoint configured, traces will not be exported");
    }

    // 4. Set up tracing subscriber with OpenTelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(global::tracer(config.service_name.clone()));

    // 5. Set up console/file logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("info,{}=debug",
                config.service_name.replace('.', "_")
            ))
        });

    // 6. Initialize subscriber
    if config.json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true),
            )
            .init();
    }

    tracing::info!("Tracing initialized successfully");

    Ok(())
}

/// Shutdown tracing gracefully
pub fn shutdown_tracing() {
    tracing::info!("Shutting down tracing");
    global::shutdown_tracer_provider();
}
```

### Integration in main.rs

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing first (before any other logging)
    let tracing_config = TracingConfig::default();
    init_tracing(tracing_config)?;

    tracing::info!("Starting application");

    // ... application code ...

    // Register shutdown handler
    tokio::select! {
        _ = shutdown_signal() => {
            tracing::info!("Received shutdown signal");
        }
    }

    // Gracefully shutdown tracing
    shutdown_tracing();

    Ok(())
}
```

### Graceful Shutdown

All services MUST flush spans before exit:

```rust
use opentelemetry::global;

pub fn shutdown_tracing() {
    tracing::info!("Flushing traces before shutdown");

    // Flush and shutdown the tracer provider
    global::shutdown_tracer_provider();

    // Give exporters time to flush
    std::thread::sleep(std::time::Duration::from_millis(500));
}
```

---

## Context Propagation Standard

### W3C Trace Context

All services MUST support W3C Trace Context propagation:

```rust
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use opentelemetry_sdk::propagation::TraceContextPropagator;

// Set global propagator
global::set_text_map_propagator(TraceContextPropagator::new());
```

### HTTP Header Propagation

#### Incoming Requests

Extract trace context from HTTP headers:

```rust
use axum::http::HeaderMap;
use opentelemetry::global;
use opentelemetry::propagation::Extractor;

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

// In request handler
let parent_context = global::get_text_map_propagator(|propagator| {
    propagator.extract(&HeaderExtractor(&headers))
});
```

#### Outgoing Requests

Inject trace context into HTTP headers:

```rust
use axum::http::HeaderMap;
use opentelemetry::global;
use opentelemetry::propagation::Injector;

struct HeaderInjector<'a>(&'a mut HeaderMap);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                self.0.insert(header_name, header_value);
            }
        }
    }
}

// Before making HTTP request
let mut headers = HeaderMap::new();
global::get_text_map_propagator(|propagator| {
    propagator.inject_context(
        &tracing::Span::current().context(),
        &mut HeaderInjector(&mut headers),
    )
});
```

### Baggage Propagation

Support for baggage (correlation data):

```rust
use opentelemetry::global;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry::propagation::TextMapCompositePropagator;

// Set composite propagator
global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
    Box::new(TraceContextPropagator::new()),
    Box::new(BaggagePropagator::new()),
]));
```

### Cross-Service Correlation

#### Request ID Propagation

All services MUST propagate request IDs:

```rust
use axum::http::HeaderMap;
use uuid::Uuid;

// Extract or generate request ID
fn get_or_create_request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

// Add to span attributes
tracing::Span::current().record("request_id", &request_id);
```

#### User/Tenant Context

Propagate user and tenant identifiers:

```rust
// Extract from JWT or API key
let user_id = extract_user_id(&headers)?;
let tenant_id = extract_tenant_id(&headers)?;

// Add to span
tracing::Span::current().record("user_id", &user_id);
tracing::Span::current().record("tenant_id", &tenant_id);

// Add to baggage for cross-service propagation
use opentelemetry::baggage::BaggageExt;
let cx = tracing::Span::current().context();
let cx = cx.with_baggage(vec![
    KeyValue::new("user_id", user_id),
    KeyValue::new("tenant_id", tenant_id),
]);
```

---

## Per-Repository Configuration

### 1. edge-agent

**Service Name**: `llm.edge-agent`

**Subcomponents**:
- `llm.edge-agent.proxy` - Main HTTP proxy
- `llm.edge-agent.cache` - Caching layer
- `llm.edge-agent.routing` - Routing engine

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.edge-agent".to_string(),
    service_version: env!("CARGO_PKG_VERSION").to_string(),
    environment: env::var("ENVIRONMENT").unwrap_or("development".into()),
    otlp_endpoint: env::var("OTLP_ENDPOINT").ok(),
    sampling_ratio: 1.0,
    json_logs: env::var("LOG_FORMAT").map(|v| v == "json").unwrap_or(false),
}
```

**Custom Span Attributes**:
```rust
// LLM request tracking
KeyValue::new("llm.provider", "openai"),
KeyValue::new("llm.model", "gpt-4"),
KeyValue::new("llm.request_id", request_id),
KeyValue::new("llm.cache.hit", cache_hit),
KeyValue::new("llm.cache.tier", "l1"),
```

**Files to Update**:
- `/workspaces/edge-agent/src/observability/tracing.rs` - Update service name
- `/workspaces/edge-agent/src/server/tracing.rs` - Align with standard
- `/workspaces/edge-agent/crates/llm-edge-monitoring/src/tracing.rs` - Implement standard
- `/workspaces/edge-agent/crates/llm-edge-proxy/src/server/tracing.rs` - Implement standard

---

### 2. shield (15 crates)

**Service Name**: `llm.shield`

**Subcomponents**:
- `llm.shield.core` - Core scanning engine
- `llm.shield.scanner` - Threat scanner
- `llm.shield.detector` - ML detector

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.shield".to_string(),
    // Same pattern as edge-agent
}
```

**Custom Span Attributes**:
```rust
// Security scanning
KeyValue::new("shield.scan_type", "input"),
KeyValue::new("shield.threat_level", "medium"),
KeyValue::new("shield.detector", "ml_classifier"),
KeyValue::new("shield.confidence", 0.85),
```

**Dependencies to Add**:
```toml
[workspace.dependencies]
opentelemetry-sdk = "0.26"
tracing-opentelemetry = "0.26"
```

---

### 3. sentinel (6 crates)

**Service Name**: `llm.sentinel`

**Subcomponents**:
- `llm.sentinel.core` - Core event processing
- `llm.sentinel.processor` - Anomaly detection
- `llm.sentinel.alerting` - Alert generation

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.sentinel".to_string(),
    // Same pattern
}
```

**Custom Span Attributes**:
```rust
// Anomaly detection
KeyValue::new("sentinel.event_type", "rate_anomaly"),
KeyValue::new("sentinel.severity", "high"),
KeyValue::new("sentinel.algorithm", "statistical"),
KeyValue::new("sentinel.confidence", 0.92),
```

---

### 4. connector-hub

**Service Name**: `llm.connector-hub`

**Subcomponents**:
- `llm.connector-hub.registry` - Provider registry
- `llm.connector-hub.adapters` - Protocol adapters

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.connector-hub".to_string(),
    // Same pattern
}
```

**Custom Span Attributes**:
```rust
// Provider operations
KeyValue::new("connector.provider_id", "openai"),
KeyValue::new("connector.operation", "register"),
KeyValue::new("connector.protocol", "rest"),
KeyValue::new("connector.health_status", "healthy"),
```

---

### 5. observatory (10 crates)

**Service Name**: `llm.observatory`

**Subcomponents**:
- `llm.observatory.collector` - Data collection
- `llm.observatory.api` - REST API
- `llm.observatory.storage` - Storage backend

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.observatory".to_string(),
    // Same pattern
}
```

**Note**: Observatory already has OpenTelemetry 0.27 - maintain compatibility.

**Custom Span Attributes**:
```rust
// Observability operations
KeyValue::new("observatory.metric_type", "gauge"),
KeyValue::new("observatory.storage_backend", "influxdb"),
KeyValue::new("observatory.query_duration_ms", 15),
```

---

### 6. cost-ops (5 crates)

**Service Name**: `llm.cost-ops`

**Subcomponents**:
- `llm.cost-ops.core` - Cost calculation
- `llm.cost-ops.api` - REST API
- `llm.cost-ops.compliance` - Regulatory compliance

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.cost-ops".to_string(),
    // Same pattern
}
```

**Custom Span Attributes**:
```rust
// Cost operations
KeyValue::new("cost.provider", "openai"),
KeyValue::new("cost.model", "gpt-4"),
KeyValue::new("cost.input_tokens", 100),
KeyValue::new("cost.output_tokens", 200),
KeyValue::new("cost.total_usd", 0.0051),
```

---

### 7. policy-engine

**Service Name**: `llm.policy-engine`

**Subcomponents**:
- `llm.policy-engine.cel` - CEL evaluation
- `llm.policy-engine.wasm` - WASM runtime

**Configuration**:
```rust
TracingConfig {
    service_name: "llm.policy-engine".to_string(),
    // Same pattern
}
```

**Custom Span Attributes**:
```rust
// Policy evaluation
KeyValue::new("policy.rule_id", "rate_limit_policy"),
KeyValue::new("policy.language", "cel"),
KeyValue::new("policy.result", "allow"),
KeyValue::new("policy.evaluation_time_ms", 2),
```

---

## Environment Variables Standard

### Required Variables

All services MUST support these environment variables:

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `ENVIRONMENT` | String | `development` | Deployment environment |
| `OTLP_ENDPOINT` | String | `http://localhost:4317` | OTLP gRPC endpoint |
| `OTEL_TRACES_SAMPLER_ARG` | Float | `1.0` | Sampling ratio (0.0-1.0) |

### Optional Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `OTLP_ENDPOINT_HTTP` | String | `http://localhost:4318` | OTLP HTTP endpoint |
| `LOG_FORMAT` | String | `text` | Log format (`text` or `json`) |
| `RUST_LOG` | String | `info` | Log level filter |
| `SERVICE_INSTANCE_ID` | String | (auto) | Service instance identifier |
| `SERVICE_NAMESPACE` | String | `llm-devops` | Service namespace |
| `OTEL_EXPORTER_OTLP_TIMEOUT` | Int | `10` | Export timeout (seconds) |
| `OTEL_EXPORTER_OTLP_COMPRESSION` | String | `none` | Compression (`none`, `gzip`) |

### Kubernetes-Specific Variables

For Kubernetes deployments, set via Downward API:

```yaml
env:
  - name: K8S_POD_NAME
    valueFrom:
      fieldRef:
        fieldPath: metadata.name
  - name: K8S_POD_UID
    valueFrom:
      fieldRef:
        fieldPath: metadata.uid
  - name: K8S_NAMESPACE
    valueFrom:
      fieldRef:
        fieldPath: metadata.namespace
  - name: K8S_NODE_NAME
    valueFrom:
      fieldRef:
        fieldPath: spec.nodeName
  - name: K8S_DEPLOYMENT_NAME
    value: "edge-agent"
  - name: K8S_CLUSTER_NAME
    value: "llm-prod-us-east-1"
```

### Docker Compose Example

```yaml
services:
  edge-agent:
    environment:
      - ENVIRONMENT=production
      - OTLP_ENDPOINT=http://otel-collector:4317
      - OTEL_TRACES_SAMPLER_ARG=1.0
      - LOG_FORMAT=json
      - SERVICE_NAMESPACE=llm-devops
```

---

## Migration Guide

### Phase 1: Edge-Agent (Week 1)

**Tasks:**

1. Update service name from `llm-edge-agent` to `llm.edge-agent`
2. Add missing OpenTelemetry SDK dependencies
3. Implement standard `TracingConfig` struct
4. Update all tracing initialization code
5. Add context propagation support
6. Update environment variable names
7. Test with local Jaeger instance

**Files to Update:**

```bash
/workspaces/edge-agent/src/observability/tracing.rs
/workspaces/edge-agent/src/server/tracing.rs
/workspaces/edge-agent/crates/llm-edge-monitoring/src/tracing.rs
/workspaces/edge-agent/crates/llm-edge-proxy/src/server/tracing.rs
/workspaces/edge-agent/Cargo.toml (workspace dependencies)
```

**Validation:**

```bash
# Start Jaeger
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest

# Set environment
export OTLP_ENDPOINT=http://localhost:4317
export ENVIRONMENT=development

# Run edge-agent
cargo run --release

# Verify traces in Jaeger UI
open http://localhost:16686
```

---

### Phase 2: Observatory (Week 2)

**Tasks:**

1. Update service name to `llm.observatory`
2. Align OpenTelemetry versions (0.26-0.27 compatible)
3. Implement standard configuration
4. Update subcomponent names

**Notes:**

- Observatory already uses OpenTelemetry 0.27
- Maintain backward compatibility
- Focus on service naming standardization

---

### Phase 3: Shield & Policy-Engine (Week 3)

**Tasks:**

1. Add OpenTelemetry dependencies
2. Implement tracing initialization
3. Update service names
4. Add custom span attributes

---

### Phase 4: Sentinel & Connector-Hub (Week 4)

**Tasks:**

1. Similar to Phase 3
2. Focus on event/provider-specific attributes

---

### Phase 5: Cost-Ops (Week 5)

**Tasks:**

1. Similar to Phase 3
2. Focus on cost-specific attributes

---

### Phase 6: Integration Testing (Week 6)

**Tasks:**

1. End-to-end trace validation
2. Cross-service correlation testing
3. Performance benchmarking
4. Documentation updates

---

## Testing and Validation

### Unit Tests

Test tracing configuration:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert!(config.service_name.starts_with("llm."));
        assert_eq!(config.sampling_ratio, 1.0);
    }

    #[test]
    fn test_service_name_format() {
        let config = TracingConfig::default();
        // Should use dots, not hyphens
        assert!(!config.service_name.contains('-'));
    }
}
```

### Integration Tests

Test context propagation:

```rust
#[tokio::test]
async fn test_context_propagation() {
    init_tracing(TracingConfig::default()).unwrap();

    let span = tracing::info_span!("parent");
    let _guard = span.enter();

    // Simulate HTTP call
    let mut headers = HeaderMap::new();
    inject_context(&mut headers);

    // Verify traceparent header
    assert!(headers.contains_key("traceparent"));
}
```

### End-to-End Tests

Test cross-service tracing:

```bash
# Start all services with Jaeger
docker-compose up -d

# Generate test traffic
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "test"}]}'

# Query Jaeger for traces
curl http://localhost:16686/api/traces?service=llm.edge-agent
```

### Performance Validation

Benchmark overhead:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_span_creation(c: &mut Criterion) {
    init_tracing(TracingConfig::default()).unwrap();

    c.bench_function("create_span", |b| {
        b.iter(|| {
            let _span = tracing::info_span!("test");
        });
    });
}
```

**Acceptance Criteria:**

- Span creation overhead: < 1μs
- Context propagation overhead: < 5μs
- Export batch processing: < 100ms
- Memory overhead: < 50MB

---

## Appendix A: Complete Code Example

See reference implementation at:
`/workspaces/edge-agent/src/observability/tracing.rs`

---

## Appendix B: Troubleshooting

### Common Issues

**Issue**: Traces not appearing in Jaeger

**Solution**:
1. Verify OTLP_ENDPOINT is correct
2. Check Jaeger OTLP receiver is enabled
3. Verify network connectivity
4. Check sampling configuration

**Issue**: Context not propagating

**Solution**:
1. Verify propagator is set globally
2. Check header extraction/injection
3. Verify W3C trace context format

---

## Appendix C: References

- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)

---

**Document Status**: Active Standard
**Version**: 1.0.0
**Last Updated**: December 4, 2025
**Next Review**: March 4, 2026
