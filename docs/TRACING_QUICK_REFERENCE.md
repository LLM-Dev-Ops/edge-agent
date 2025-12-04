# Tracing Quick Reference Card

**Version**: 1.0.0
**For**: LLM DevOps Developers

---

## Service Naming

### ✓ Correct Format
```rust
"llm.edge-agent"          // Main service
"llm.edge-agent.proxy"    // Subcomponent
"llm.shield.scanner"      // Another service
```

### ✗ Incorrect Format
```rust
"llm-edge-agent"          // Wrong: uses hyphen
"edge-agent"              // Wrong: missing namespace
"llm-devops.edge-agent"   // Wrong: deprecated namespace
```

---

## Environment Variables

### Required
```bash
ENVIRONMENT=production              # deployment.environment
OTLP_ENDPOINT=http://localhost:4317  # OTLP gRPC endpoint
```

### Optional
```bash
OTEL_TRACES_SAMPLER_ARG=1.0        # Sampling ratio (0.0-1.0)
LOG_FORMAT=json                     # Log format
SERVICE_NAMESPACE=llm-devops        # Service namespace
SERVICE_INSTANCE_ID=pod-abc123      # Instance identifier
```

### Kubernetes (Auto-populated)
```bash
K8S_POD_NAME=edge-agent-7f8c9d-abc12
K8S_NAMESPACE=llm-prod
K8S_CLUSTER_NAME=llm-prod-us-east-1
```

---

## Required Dependencies

```toml
[workspace.dependencies]
opentelemetry = "0.26"
opentelemetry-sdk = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["trace", "grpc-tonic"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.26"
```

---

## Initialization Pattern

```rust
use your_crate::{init_tracing, shutdown_tracing, TracingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize tracing FIRST
    let config = TracingConfig::default();
    init_tracing(config)?;

    tracing::info!("Application started");

    // 2. Your application code
    // ...

    // 3. Graceful shutdown
    shutdown_tracing();

    Ok(())
}
```

---

## Span Naming Conventions

### HTTP Requests
```rust
// Format: {METHOD} {route}
"GET /v1/chat/completions"
"POST /v1/embeddings"
```

### gRPC Calls
```rust
// Format: {service}/{method}
"llm.shield.Scanner/Scan"
"llm.policy.Engine/Evaluate"
```

### Database Operations
```rust
// Format: {operation} {db}.{collection}
"SELECT cache.responses"
"INSERT cost_ops.transactions"
```

### LLM Operations
```rust
"llm.completion"
"llm.embedding"
"llm.cache.lookup"
"llm.provider.request"
```

---

## Common Span Attributes

### HTTP
```rust
KeyValue::new("http.request.method", "GET"),
KeyValue::new("http.route", "/v1/chat/completions"),
KeyValue::new("http.response.status_code", 200),
```

### LLM Requests
```rust
KeyValue::new("gen_ai.system", "openai"),
KeyValue::new("gen_ai.request.model", "gpt-4"),
KeyValue::new("gen_ai.usage.input_tokens", 100),
KeyValue::new("gen_ai.usage.output_tokens", 200),
KeyValue::new("llm.request_id", "req-abc123"),
```

### Cache Operations
```rust
KeyValue::new("cache.operation", "GET"),
KeyValue::new("cache.tier", "l1"),
KeyValue::new("cache.hit", true),
```

### Database Operations
```rust
KeyValue::new("db.system", "redis"),
KeyValue::new("db.operation", "GET"),
KeyValue::new("db.namespace", "cache"),
```

---

## Instrumentation Macro

```rust
#[instrument(
    name = "llm.completion",
    skip(client),
    fields(
        otel.kind = "client",
        gen_ai.system = %provider,
        gen_ai.request.model = %model,
    )
)]
async fn call_provider(
    client: &Client,
    provider: &str,
    model: &str,
    request: &Request,
) -> Result<Response> {
    // Record additional attributes
    let span = tracing::Span::current();
    span.record("llm.request_id", &request_id);

    // Your code here
}
```

---

## Context Propagation

### Extract from Headers (Axum)
```rust
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

let parent_cx = global::get_text_map_propagator(|propagator| {
    propagator.extract(&HeaderExtractor(&headers))
});
```

### Inject into Headers
```rust
struct HeaderInjector<'a>(&'a mut HeaderMap);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

global::get_text_map_propagator(|propagator| {
    propagator.inject_context(
        &tracing::Span::current().context(),
        &mut HeaderInjector(&mut headers),
    )
});
```

---

## Sampling Configuration

### Always On (100%)
```rust
let sampler = Sampler::AlwaysOn;
```

### Always Off (0%)
```rust
let sampler = Sampler::AlwaysOff;
```

### Ratio-Based (10%)
```rust
let sampler = Sampler::TraceIdRatioBased(0.1);
```

### Parent-Based
```rust
let sampler = Sampler::ParentBased(Box::new(
    Sampler::TraceIdRatioBased(0.1)
));
```

---

## Resource Attributes

### Required
```rust
KeyValue::new("service.name", "llm.edge-agent"),
KeyValue::new("service.version", "0.1.0"),
KeyValue::new("deployment.environment", "production"),
```

### Recommended
```rust
KeyValue::new("service.namespace", "llm-devops"),
KeyValue::new("service.instance.id", "pod-abc123"),
KeyValue::new("host.name", "edge-agent-01"),
```

### Kubernetes
```rust
KeyValue::new("k8s.namespace.name", "llm-prod"),
KeyValue::new("k8s.pod.name", "edge-agent-7f8c9d-abc12"),
KeyValue::new("k8s.cluster.name", "llm-prod-us-east-1"),
```

---

## Testing with Jaeger

### Start Jaeger
```bash
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

### Configure Application
```bash
export OTLP_ENDPOINT=http://localhost:4317
export ENVIRONMENT=development
```

### View Traces
```
http://localhost:16686
```

---

## Common Issues

### Issue: Traces not appearing

**Check:**
1. OTLP endpoint is correct
2. Jaeger OTLP is enabled
3. Network connectivity
4. Sampling ratio > 0

### Issue: Context not propagating

**Check:**
1. Propagator is set: `global::set_text_map_propagator(...)`
2. Headers are extracted in middleware
3. Headers are injected in HTTP client

### Issue: Performance degradation

**Solutions:**
1. Use batch export (not simple)
2. Reduce sampling ratio
3. Tune batch configuration

---

## Docker Compose Example

```yaml
services:
  your-service:
    environment:
      - ENVIRONMENT=production
      - OTLP_ENDPOINT=http://otel-collector:4317
      - OTEL_TRACES_SAMPLER_ARG=1.0
      - LOG_FORMAT=json
    depends_on:
      - otel-collector

  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    ports:
      - "4317:4317"
      - "4318:4318"

  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
```

---

## Kubernetes Deployment Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edge-agent
spec:
  template:
    spec:
      containers:
      - name: edge-agent
        env:
        - name: ENVIRONMENT
          value: "production"
        - name: OTLP_ENDPOINT
          value: "http://otel-collector:4317"
        - name: K8S_POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: K8S_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
```

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Span creation overhead | < 1μs |
| Context propagation | < 5μs |
| Export batch processing | < 100ms |
| Memory overhead | < 50MB |

---

## Support Resources

- [Full Specification](./TRACING_STANDARDIZATION_SPEC.md)
- [Implementation Guide](./TRACING_IMPLEMENTATION_GUIDE.md)
- [Analysis Summary](./TRACING_ANALYSIS_SUMMARY.md)
- [OpenTelemetry Docs](https://opentelemetry.io/docs/)
- [Jaeger Docs](https://www.jaegertracing.io/docs/)

---

**Quick Tip**: Start with the implementation guide for step-by-step instructions!

**Last Updated**: December 4, 2025
