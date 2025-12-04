# Tracing Implementation Guide

**Companion to**: TRACING_STANDARDIZATION_SPEC.md
**Version**: 1.0.0
**Date**: December 4, 2025

---

## Quick Start

This guide provides step-by-step instructions for implementing the unified tracing standard in each repository.

---

## Table of Contents

1. [Cargo Dependencies Setup](#cargo-dependencies-setup)
2. [Configuration Module](#configuration-module)
3. [Initialization Code](#initialization-code)
4. [Middleware Integration](#middleware-integration)
5. [Custom Span Attributes](#custom-span-attributes)
6. [Context Propagation](#context-propagation)
7. [Testing](#testing)
8. [Deployment Configuration](#deployment-configuration)

---

## Cargo Dependencies Setup

### Step 1: Add Workspace Dependencies

Update your root `Cargo.toml`:

```toml
[workspace.dependencies]
# OpenTelemetry - Tracing
opentelemetry = "0.26"
opentelemetry-sdk = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["trace", "grpc-tonic"] }
opentelemetry-semantic-conventions = "0.26"

# Tracing integration
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.26"
```

### Step 2: Add to Crate Dependencies

In your crate's `Cargo.toml`:

```toml
[dependencies]
# Observability - Tracing
tracing.workspace = true
tracing-subscriber.workspace = true
opentelemetry.workspace = true
opentelemetry-sdk.workspace = true
opentelemetry-otlp.workspace = true
tracing-opentelemetry.workspace = true

# Async Runtime
tokio.workspace = true
```

---

## Configuration Module

### Step 1: Create tracing.rs Module

Create `src/observability/tracing.rs` (or `src/tracing.rs`):

```rust
//! OpenTelemetry distributed tracing setup
//!
//! Provides end-to-end tracing for service requests.

use opentelemetry::{
    global,
    trace::{TraceError, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name (e.g., "llm.edge-agent")
    pub service_name: String,

    /// Service version (from Cargo.toml)
    pub service_version: String,

    /// Deployment environment (development, staging, production)
    pub environment: String,

    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,

    /// Sampling ratio (0.0 to 1.0)
    pub sampling_ratio: f64,

    /// Enable JSON formatted logs
    pub json_logs: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        // Convert crate name to service name format
        // Example: "llm-edge-agent" -> "llm.edge-agent"
        let service_name = env!("CARGO_PKG_NAME").replace('-', ".");

        Self {
            service_name,
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
pub fn init_tracing(config: TracingConfig) -> Result<(), TraceError> {
    info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        "Initializing OpenTelemetry tracing"
    );

    // Create resource with service information
    let resource = create_resource(&config);

    // Configure sampler based on sampling ratio
    let sampler = create_sampler(config.sampling_ratio);

    // Set up OTLP exporter if endpoint is configured
    if let Some(endpoint) = &config.otlp_endpoint {
        setup_otlp_exporter(endpoint, &config, resource.clone(), sampler.clone())?;
    } else {
        warn!("No OTLP endpoint configured, traces will not be exported");
    }

    // Set up tracing subscriber with OpenTelemetry layer
    setup_subscriber(&config)?;

    info!("Tracing initialized successfully");

    Ok(())
}

/// Create OpenTelemetry resource with service metadata
fn create_resource(config: &TracingConfig) -> Resource {
    let mut attributes = vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
        KeyValue::new("service.namespace", "llm-devops"),
    ];

    // Add optional attributes if available
    if let Ok(instance_id) = std::env::var("SERVICE_INSTANCE_ID") {
        attributes.push(KeyValue::new("service.instance.id", instance_id));
    }

    if let Ok(hostname) = std::env::var("HOSTNAME") {
        attributes.push(KeyValue::new("host.name", hostname));
    }

    // Kubernetes attributes (if running in k8s)
    if let Ok(pod_name) = std::env::var("K8S_POD_NAME") {
        attributes.push(KeyValue::new("k8s.pod.name", pod_name));
    }

    if let Ok(namespace) = std::env::var("K8S_NAMESPACE") {
        attributes.push(KeyValue::new("k8s.namespace.name", namespace));
    }

    if let Ok(cluster) = std::env::var("K8S_CLUSTER_NAME") {
        attributes.push(KeyValue::new("k8s.cluster.name", cluster));
    }

    Resource::new(attributes)
}

/// Create sampler based on sampling ratio
fn create_sampler(sampling_ratio: f64) -> Sampler {
    if sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(sampling_ratio)
    }
}

/// Setup OTLP exporter
fn setup_otlp_exporter(
    endpoint: &str,
    config: &TracingConfig,
    resource: Resource,
    sampler: Sampler,
) -> Result<(), TraceError> {
    info!(endpoint = %endpoint, "Configuring OTLP exporter");

    match opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_timeout(std::time::Duration::from_secs(10)),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)
    {
        Ok(provider) => {
            global::set_tracer_provider(provider);
            info!("OTLP exporter configured successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Failed to initialize OTLP exporter");
            warn!("Continuing without OTLP export");
            Err(e)
        }
    }
}

/// Setup tracing subscriber
fn setup_subscriber(config: &TracingConfig) -> Result<(), TraceError> {
    // OpenTelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(global::tracer(config.service_name.clone()));

    // Environment filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "info,{}=debug",
            config.service_name.replace('.', "_")
        ))
    });

    // Initialize subscriber
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

    Ok(())
}

/// Shutdown tracing gracefully
pub fn shutdown_tracing() {
    info!("Shutting down tracing");
    global::shutdown_tracer_provider();
}

/// Helper module for common span attributes
pub mod span_attributes {
    use opentelemetry::KeyValue;

    /// Create span attributes for HTTP requests
    pub fn http_request(method: &str, route: &str, status_code: u16) -> Vec<KeyValue> {
        vec![
            KeyValue::new("http.request.method", method.to_string()),
            KeyValue::new("http.route", route.to_string()),
            KeyValue::new("http.response.status_code", status_code as i64),
        ]
    }

    /// Create span attributes for LLM requests
    pub fn llm_request(
        provider: &str,
        model: &str,
        request_id: &str,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("gen_ai.system", provider.to_string()),
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("llm.request_id", request_id.to_string()),
        ]
    }

    /// Create span attributes for cache operations
    pub fn cache_operation(
        operation: &str,
        cache_tier: &str,
        hit: bool,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("cache.operation", operation.to_string()),
            KeyValue::new("cache.tier", cache_tier.to_string()),
            KeyValue::new("cache.hit", hit),
        ]
    }

    /// Create span attributes for database operations
    pub fn db_operation(
        system: &str,
        operation: &str,
        namespace: &str,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("db.system", system.to_string()),
            KeyValue::new("db.operation", operation.to_string()),
            KeyValue::new("db.namespace", namespace.to_string()),
        ]
    }
}

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

    #[test]
    fn test_sampler_creation() {
        assert!(matches!(create_sampler(1.0), Sampler::AlwaysOn));
        assert!(matches!(create_sampler(0.0), Sampler::AlwaysOff));
        assert!(matches!(create_sampler(0.5), Sampler::TraceIdRatioBased(_)));
    }
}
```

### Step 2: Export from lib.rs

Add to your `src/lib.rs`:

```rust
pub mod observability;

// Re-export tracing utilities
pub use observability::tracing::{
    init_tracing, shutdown_tracing, span_attributes, TracingConfig,
};
```

---

## Initialization Code

### In main.rs

```rust
use your_crate::{init_tracing, shutdown_tracing, TracingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing FIRST (before any other logging)
    let tracing_config = TracingConfig::default();
    init_tracing(tracing_config)?;

    tracing::info!("Starting application");

    // ... your application code ...

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

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
}
```

---

## Middleware Integration

### Context Propagation Middleware (Axum)

```rust
use axum::http::{HeaderMap, Request};
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use tower::{Layer, Service};

/// Extractor for Axum HeaderMap
struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Injector for Axum HeaderMap
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

/// Extract trace context from incoming request
pub fn extract_trace_context(headers: &HeaderMap) -> opentelemetry::Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}

/// Inject trace context into outgoing request
pub fn inject_trace_context(headers: &mut HeaderMap) {
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(
            &tracing::Span::current().context(),
            &mut HeaderInjector(headers),
        )
    });
}

/// Tracing middleware for Axum
pub async fn tracing_middleware<B>(
    req: Request<B>,
    next: axum::middleware::Next<B>,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers();

    // Extract parent context
    let parent_cx = extract_trace_context(headers);

    // Create span with parent context
    let span = tracing::info_span!(
        "http.request",
        "otel.kind" = "server",
        "http.request.method" = %method,
        "http.route" = %uri.path(),
    );

    // Attach parent context to span
    span.set_parent(parent_cx);

    // Execute request within span
    let _guard = span.enter();
    let response = next.run(req).await;

    // Record response status
    tracing::Span::current().record(
        "http.response.status_code",
        response.status().as_u16(),
    );

    response
}
```

### Usage in Router

```rust
use axum::{Router, routing::get};

fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/completions", post(completions_handler))
        // Add tracing middleware
        .layer(axum::middleware::from_fn(tracing_middleware))
}
```

---

## Custom Span Attributes

### Example: LLM Request Tracing

```rust
use tracing::{info_span, instrument};
use opentelemetry::KeyValue;

#[instrument(
    name = "llm.completion",
    skip(client, request),
    fields(
        otel.kind = "client",
        gen_ai.system = %provider,
        gen_ai.request.model = %request.model,
    )
)]
async fn call_llm_provider(
    client: &reqwest::Client,
    provider: &str,
    request: &CompletionRequest,
) -> Result<CompletionResponse> {
    let span = tracing::Span::current();

    // Add request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    span.record("llm.request_id", &request_id);

    // Add token counts
    span.record("gen_ai.request.max_tokens", request.max_tokens);

    // Make request
    let response = client
        .post(format!("https://api.{}.com/v1/completions", provider))
        .json(&request)
        .send()
        .await?;

    // Record response metrics
    span.record("http.response.status_code", response.status().as_u16());

    let completion = response.json::<CompletionResponse>().await?;

    // Record token usage
    span.record("gen_ai.usage.input_tokens", completion.usage.input_tokens);
    span.record("gen_ai.usage.output_tokens", completion.usage.output_tokens);

    Ok(completion)
}
```

### Example: Cache Lookup Tracing

```rust
#[instrument(
    name = "cache.lookup",
    skip(cache),
    fields(
        cache.tier = "l1",
        cache.operation = "GET",
    )
)]
async fn lookup_cache(cache: &Cache, key: &str) -> Option<CachedValue> {
    let span = tracing::Span::current();

    // Add key hash (not the actual key for privacy)
    let key_hash = hash_key(key);
    span.record("cache.key_hash", &key_hash);

    // Perform lookup
    let result = cache.get(key).await;

    // Record hit/miss
    span.record("cache.hit", result.is_some());

    if let Some(ref value) = result {
        span.record("cache.value_size_bytes", value.size());
    }

    result
}
```

### Example: Database Query Tracing

```rust
#[instrument(
    name = "db.query",
    skip(pool, query),
    fields(
        db.system = "postgresql",
        db.operation = "SELECT",
        db.namespace = "cost_ops",
    )
)]
async fn query_database(
    pool: &sqlx::PgPool,
    query: &str,
) -> Result<Vec<Row>> {
    let span = tracing::Span::current();

    // Record query (sanitized)
    span.record("db.query.text", sanitize_query(query));

    // Execute query
    let start = std::time::Instant::now();
    let rows = sqlx::query(query)
        .fetch_all(pool)
        .await?;
    let duration = start.elapsed();

    // Record metrics
    span.record("db.query.duration_ms", duration.as_millis() as i64);
    span.record("db.query.rows_returned", rows.len() as i64);

    Ok(rows)
}
```

---

## Context Propagation

### Setup Propagator (in init_tracing)

```rust
use opentelemetry::global;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry::propagation::TextMapCompositePropagator;

pub fn init_tracing(config: TracingConfig) -> Result<(), TraceError> {
    // ... existing code ...

    // Set up composite propagator (W3C Trace Context + Baggage)
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(BaggagePropagator::new()),
    ]));

    // ... rest of initialization ...
}
```

### Propagate in HTTP Client

```rust
use reqwest::Client;

async fn call_downstream_service(
    client: &Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<Response> {
    let mut headers = reqwest::header::HeaderMap::new();

    // Inject trace context
    inject_trace_context(&mut headers);

    // Make request with trace headers
    let response = client
        .post(url)
        .headers(headers)
        .json(body)
        .send()
        .await?;

    Ok(response)
}
```

---

## Testing

### Unit Test: Configuration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("ENVIRONMENT", "production");
        std::env::set_var("OTLP_ENDPOINT", "http://collector:4317");
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.1");

        let config = TracingConfig::default();

        assert_eq!(config.environment, "production");
        assert_eq!(config.otlp_endpoint, Some("http://collector:4317".to_string()));
        assert_eq!(config.sampling_ratio, 0.1);
    }

    #[test]
    fn test_service_name_normalization() {
        // Service name should use dots, not hyphens
        let config = TracingConfig::default();
        assert!(!config.service_name.contains('-'));
        assert!(config.service_name.contains('.'));
    }
}
```

### Integration Test: Context Propagation

```rust
#[tokio::test]
async fn test_context_propagation() {
    init_tracing(TracingConfig::default()).unwrap();

    let span = tracing::info_span!("parent");
    let _guard = span.enter();

    // Create headers
    let mut headers = HeaderMap::new();

    // Inject context
    inject_trace_context(&mut headers);

    // Verify traceparent header exists
    assert!(headers.contains_key("traceparent"));

    // Extract context
    let extracted_cx = extract_trace_context(&headers);

    // Verify trace ID matches
    // (implementation-specific verification)
}
```

### End-to-End Test

```rust
#[tokio::test]
async fn test_e2e_tracing() {
    // Start Jaeger in-memory
    // (use testcontainers-rs or similar)

    // Initialize tracing
    init_tracing(TracingConfig {
        otlp_endpoint: Some("http://localhost:4317".to_string()),
        ..Default::default()
    }).unwrap();

    // Generate test spans
    {
        let span = tracing::info_span!("test_operation");
        let _guard = span.enter();

        tracing::info!("Test event");
    }

    // Flush traces
    shutdown_tracing();

    // Query Jaeger for traces
    // (verify traces were exported correctly)
}
```

---

## Deployment Configuration

### Docker Compose

```yaml
version: '3.8'

services:
  your-service:
    image: your-service:latest
    environment:
      - ENVIRONMENT=production
      - OTLP_ENDPOINT=http://otel-collector:4317
      - OTEL_TRACES_SAMPLER_ARG=1.0
      - LOG_FORMAT=json
      - SERVICE_NAMESPACE=llm-devops
    depends_on:
      - otel-collector

  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    command: ["--config=/etc/otel-collector-config.yaml"]
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml

  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"  # Jaeger UI
    environment:
      - COLLECTOR_OTLP_ENABLED=true
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edge-agent
  namespace: llm-prod
spec:
  replicas: 3
  template:
    metadata:
      labels:
        app: edge-agent
    spec:
      containers:
      - name: edge-agent
        image: edge-agent:v1.0.0
        env:
        - name: ENVIRONMENT
          value: "production"
        - name: OTLP_ENDPOINT
          value: "http://otel-collector.observability:4317"
        - name: OTEL_TRACES_SAMPLER_ARG
          value: "1.0"
        - name: LOG_FORMAT
          value: "json"
        - name: SERVICE_NAMESPACE
          value: "llm-devops"

        # Kubernetes metadata via Downward API
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

### OpenTelemetry Collector Config

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 5s
    send_batch_size: 512
    send_batch_max_size: 1024

  resource:
    attributes:
    - key: service.namespace
      value: "llm-devops"
      action: upsert

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  logging:
    loglevel: debug

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch, resource]
      exporters: [jaeger, logging]
```

---

## Checklist

Use this checklist when implementing tracing in a repository:

- [ ] Add OpenTelemetry dependencies to Cargo.toml
- [ ] Create `src/observability/tracing.rs` module
- [ ] Implement `TracingConfig` struct
- [ ] Implement `init_tracing()` function
- [ ] Implement `shutdown_tracing()` function
- [ ] Add span attribute helpers
- [ ] Implement context propagation (extract/inject)
- [ ] Update service name format (use dots, not hyphens)
- [ ] Add tracing middleware to HTTP server
- [ ] Add instrumentation to key functions
- [ ] Update main.rs initialization
- [ ] Add unit tests
- [ ] Add integration tests
- [ ] Update Docker Compose configuration
- [ ] Update Kubernetes manifests
- [ ] Update documentation
- [ ] Test with local Jaeger
- [ ] Performance benchmark

---

## Troubleshooting

### Traces not appearing in Jaeger

1. Verify OTLP endpoint is reachable:
   ```bash
   curl -v http://localhost:4317
   ```

2. Check Jaeger OTLP receiver is enabled:
   ```bash
   docker logs jaeger | grep OTLP
   ```

3. Verify tracing initialization succeeded:
   ```bash
   # Look for initialization logs
   grep "Tracing initialized" app.log
   ```

4. Check sampling configuration:
   ```bash
   echo $OTEL_TRACES_SAMPLER_ARG  # Should be > 0
   ```

### Context not propagating

1. Verify propagator is set:
   ```rust
   // In init_tracing()
   global::set_text_map_propagator(TraceContextPropagator::new());
   ```

2. Check header extraction/injection:
   ```bash
   # Verify traceparent header is present
   curl -v http://localhost:8080/api -H "traceparent: 00-..."
   ```

3. Verify middleware order:
   ```rust
   // Tracing middleware should be early in the chain
   Router::new()
       .layer(tracing_middleware)  // ← Early
       .layer(auth_middleware)
   ```

### Performance issues

1. Use batch export (not simple):
   ```rust
   .install_batch(runtime::Tokio)  // ✓ Good
   ```

2. Reduce sampling ratio:
   ```bash
   export OTEL_TRACES_SAMPLER_ARG=0.1  # 10% sampling
   ```

3. Tune batch configuration:
   ```rust
   BatchConfig::default()
       .with_max_queue_size(2048)
       .with_scheduled_delay(Duration::from_secs(5))
   ```

---

## Additional Resources

- [OpenTelemetry Rust SDK](https://github.com/open-telemetry/opentelemetry-rust)
- [Tracing Subscriber Documentation](https://docs.rs/tracing-subscriber/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)

---

**Version**: 1.0.0
**Last Updated**: December 4, 2025
