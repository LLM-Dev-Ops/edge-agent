# Rust Technical Recommendations for LLM-Edge-Agent

## Executive Summary

This document provides comprehensive technical recommendations for implementing an LLM Edge Agent in Rust, focusing on performance, security, and production-readiness. The recommendations are based on 2025 best practices and proven production deployments.

### Performance Highlights from Production Systems
- **Cloudflare Pingora**: 90M requests/sec, 70% less CPU, 67% less memory vs nginx
- **Helicone AI Gateway**: 8ms P50 latency, ~50ms added overhead
- **Rust at Scale**: 75ms intra-token latency at 15k connections (vs 200ms Python, 150ms Node.js)

---

## 1. Core Web Framework Stack

### Primary Recommendation: Axum + Hyper + Tower

**Axum v0.8** (Latest as of 2025)
- Built on top of Hyper 1.0 and Tower middleware
- Minimal overhead: performance comparable to raw Hyper
- First-class async/await support with Tokio runtime
- Type-safe extractors and handlers
- Official HTTP proxy example available

```toml
[dependencies]
axum = "0.8"
hyper = "1.0"
hyper-util = "0.1"
tower = "0.5"
tower-http = "0.6"
tokio = { version = "1.40", features = ["full"] }
```

**Why Axum over alternatives:**
1. **Performance**: Thin layer over Hyper with negligible overhead
2. **Ecosystem**: Full compatibility with Tower middleware ecosystem
3. **Developer Experience**: Type-safe, ergonomic API design
4. **Production Ready**: Used by major companies at scale
5. **Active Development**: Strong community support and frequent updates

**Key Tower Middleware for LLM Proxy:**
```rust
use tower::ServiceBuilder;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
};

let middleware = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(TraceLayer::new_for_http())
    .layer(CompressionLayer::new())
    .layer(RequestBodyLimitLayer::new(10_000_000)) // 10MB
    .layer(CorsLayer::permissive());
```

---

## 2. Async Runtime: Tokio

**Tokio v1.40+** (Current Stable)

**Key Features for LLM Proxy:**
- **Work-stealing scheduler**: Optimal CPU utilization
- **Zero-cost abstractions**: Bare-metal performance
- **Thread locality**: Excellent cache locality for hot paths
- **Cooperative multitasking**: Predictable latency

**Runtime Configuration:**
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    // Application entry point
}
```

**Best Practices:**
1. Use `tokio::spawn` for CPU-bound tasks to prevent blocking
2. Leverage `tokio::select!` for concurrent operations
3. Use `tokio::time::timeout` for request deadlines
4. Enable `tokio-console` for production debugging

```toml
[dependencies]
tokio = { version = "1.40", features = ["full", "tracing"] }
tokio-util = { version = "0.7", features = ["codec", "io"] }
console-subscriber = "0.4" # For tokio-console
```

---

## 3. Caching Strategy

### Primary: Moka Cache

**Moka v0.12+** (High-performance concurrent cache)

**Key Features:**
- **TinyLFU admission policy**: Intelligent cache population
- **Lock-free hash table**: Excellent concurrency
- **Size-aware eviction**: Memory-based limits
- **Both sync and async APIs**: Flexible integration

**Recommended Configuration:**
```rust
use moka::future::Cache;
use std::time::Duration;

// Response cache with TTL
let response_cache = Cache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(300))
    .time_to_idle(Duration::from_secs(60))
    .weigher(|_key, value: &CachedResponse| {
        // Weight by response size
        (value.body.len() as u32) / 1024 // KB
    })
    .build();

// Embedding cache (longer TTL)
let embedding_cache = Cache::builder()
    .max_capacity(100_000)
    .time_to_live(Duration::from_secs(3600))
    .build();
```

**Cache Strategies by Use Case:**

1. **LLM Response Caching**
   - Key: Hash of (model + prompt + parameters)
   - TTL: 5-15 minutes
   - Weight by response size

2. **Embedding Caching**
   - Key: Hash of input text
   - TTL: 1 hour
   - Fixed weight per entry

3. **Rate Limit Tracking**
   - Use mini-moka for simplicity
   - TTL: Based on rate limit window
   - Low memory footprint

```toml
[dependencies]
moka = { version = "0.12", features = ["future"] }
mini-moka = "0.10" # Lightweight alternative
```

---

## 4. Load Balancing & Routing

### Tower Balance + Custom Routing

**Tower Balance v0.5**
- Implements "Power of Two Choices" algorithm
- Built-in service discovery integration
- Automatic unhealthy endpoint detection

**Architecture:**
```rust
use tower::balance::p2c::Balance;
use tower::discover::ServiceList;
use tower::load::Load;

// Define backend services
let backends = vec![
    ("openai", OpenAIService::new()),
    ("anthropic", AnthropicService::new()),
    ("local", LocalModelService::new()),
];

// Create load balancer
let load_balancer = Balance::new(
    ServiceList::new(backends),
);

// Routing logic
async fn route_request(
    request: LLMRequest,
    balancer: &mut Balance<ServiceList<LLMService>>,
) -> Result<Response> {
    match request.routing_strategy {
        RoutingStrategy::Fastest => {
            balancer.call(request).await
        }
        RoutingStrategy::CostOptimized => {
            select_cheapest_backend(&request).call(request).await
        }
        RoutingStrategy::Specific(provider) => {
            get_backend(provider).call(request).await
        }
    }
}
```

**Smart Routing Patterns:**

1. **Semantic Routing**
   ```toml
   [dependencies]
   candle-core = "0.7"
   candle-nn = "0.7"
   tokenizers = "0.20"
   ```

   Use BERT embeddings for intelligent request classification:
   - Simple queries → Smaller, faster models
   - Complex queries → Larger, more capable models

2. **Cost-Based Routing**
   - Track token usage and costs per model
   - Route based on budget constraints
   - Implement fallback chains

3. **Performance-Based Routing**
   - Monitor P50/P95/P99 latencies
   - Automatic circuit breaking for slow backends
   - Dynamic weight adjustment

---

## 5. Observability & Telemetry

### OpenTelemetry Integration

**Production-Ready Telemetry Stack:**
```toml
[dependencies]
opentelemetry = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["grpc"] }
opentelemetry_sdk = { version = "0.26", features = ["rt-tokio"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.27"
metrics = "0.23"
metrics-exporter-prometheus = "0.15"
```

**Complete Observability Setup:**
```rust
use opentelemetry::global;
use opentelemetry_sdk::trace::{self, Tracer};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_telemetry() -> Result<Tracer> {
    // OTLP exporter for traces
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .grpc()
                .with_endpoint("http://localhost:4317")
        )
        .with_trace_config(
            trace::config()
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "llm-edge-agent"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Tracing subscriber with OpenTelemetry
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_opentelemetry::layer().with_tracer(tracer.clone()))
        .init();

    Ok(tracer)
}
```

**Key Metrics to Track:**

1. **Request Metrics**
   - Request rate (RPS)
   - Error rate (%)
   - Latency distribution (P50, P95, P99)
   - Request size distribution

2. **LLM-Specific Metrics**
   - Token usage (input/output)
   - Model-specific latency
   - Cache hit/miss ratio
   - Cost per request
   - First token latency
   - Streaming chunk delays

3. **System Metrics**
   - CPU utilization
   - Memory usage
   - Connection pool stats
   - Active connections
   - Thread pool utilization

**Structured Logging:**
```rust
use tracing::{info, warn, error, instrument};

#[instrument(
    skip(request),
    fields(
        request_id = %request.id,
        model = %request.model,
        tokens = tracing::field::Empty,
    )
)]
async fn process_llm_request(request: LLMRequest) -> Result<Response> {
    info!("Processing LLM request");

    let response = llm_service.call(request).await?;

    tracing::Span::current().record("tokens", response.tokens);

    Ok(response)
}
```

---

## 6. Rate Limiting & Traffic Control

### Tower Governor

**tower-governor v0.8+**

```toml
[dependencies]
tower-governor = "0.8"
governor = "0.6"
```

**Multi-tier Rate Limiting:**
```rust
use tower_governor::{
    governor::GovernorConfigBuilder,
    GovernorLayer,
    key_extractor::{SmartIpKeyExtractor, KeyExtractor},
};

// Global rate limit
let global_governor_conf = GovernorConfigBuilder::default()
    .per_second(1000)
    .burst_size(2000)
    .finish()
    .unwrap();

// Per-IP rate limit
let ip_governor_conf = GovernorConfigBuilder::default()
    .per_second(10)
    .burst_size(20)
    .use_headers() // Add rate limit headers
    .finish()
    .unwrap();

// Per-API-key rate limit (custom extractor)
#[derive(Clone)]
struct ApiKeyExtractor;

impl KeyExtractor for ApiKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        req.headers()
            .get("x-api-key")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

let api_key_governor_conf = GovernorConfigBuilder::default()
    .per_second(100)
    .burst_size(150)
    .key_extractor(ApiKeyExtractor)
    .finish()
    .unwrap();
```

**Tiered Rate Limiting Strategy:**

1. **Free Tier**: 10 req/sec, 1000 req/day
2. **Pro Tier**: 100 req/sec, 100k req/day
3. **Enterprise**: 1000 req/sec, unlimited daily

```rust
#[derive(Debug)]
struct TierLimits {
    requests_per_second: u64,
    daily_quota: u64,
    concurrent_requests: u32,
}

impl TierLimits {
    fn for_tier(tier: Tier) -> Self {
        match tier {
            Tier::Free => Self {
                requests_per_second: 10,
                daily_quota: 1_000,
                concurrent_requests: 5,
            },
            Tier::Pro => Self {
                requests_per_second: 100,
                daily_quota: 100_000,
                concurrent_requests: 50,
            },
            Tier::Enterprise => Self {
                requests_per_second: 1_000,
                daily_quota: u64::MAX,
                concurrent_requests: 500,
            },
        }
    }
}
```

---

## 7. Circuit Breakers & Resilience

### Failsafe-rs

**failsafe v1.3+**

```toml
[dependencies]
failsafe = { version = "1.3", features = ["futures"] }
```

**Production Circuit Breaker Configuration:**
```rust
use failsafe::{Config, CircuitBreaker, backoff};
use std::time::Duration;

// Create circuit breaker for each backend
fn create_backend_circuit_breaker() -> CircuitBreaker {
    let config = Config::new()
        .failure_policy(
            failsafe::failure_policy::consecutive_failures(5, Duration::from_secs(60))
        )
        .backoff_policy(
            backoff::exponential(
                Duration::from_millis(100),
                Duration::from_secs(30)
            )
        );

    CircuitBreaker::new(config)
}

// Usage with LLM backend
async fn call_llm_with_circuit_breaker(
    circuit_breaker: &CircuitBreaker,
    request: LLMRequest,
) -> Result<Response> {
    circuit_breaker
        .call(|| async {
            // Timeout wrapper
            tokio::time::timeout(
                Duration::from_secs(30),
                llm_client.send_request(request)
            )
            .await?
        })
        .await
}
```

**Multi-Backend Failover:**
```rust
#[derive(Clone)]
struct ResilientLLMProxy {
    primary: Arc<CircuitBreaker>,
    secondary: Arc<CircuitBreaker>,
    fallback: Arc<CircuitBreaker>,
}

impl ResilientLLMProxy {
    async fn call(&self, request: LLMRequest) -> Result<Response> {
        // Try primary
        match self.primary.call(|| self.call_primary(&request)).await {
            Ok(response) => return Ok(response),
            Err(e) => warn!("Primary backend failed: {}", e),
        }

        // Try secondary
        match self.secondary.call(|| self.call_secondary(&request)).await {
            Ok(response) => return Ok(response),
            Err(e) => warn!("Secondary backend failed: {}", e),
        }

        // Try fallback (e.g., cached response or simplified model)
        self.fallback.call(|| self.call_fallback(&request)).await
    }
}
```

**Timeout Strategy:**
```rust
use tower_http::timeout::TimeoutLayer;

// Different timeouts for different operations
struct TimeoutConfig {
    embedding: Duration,
    completion: Duration,
    streaming: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            embedding: Duration::from_secs(5),
            completion: Duration::from_secs(30),
            streaming: Duration::from_secs(120),
        }
    }
}
```

---

## 8. API Schema & Documentation

### utoipa (OpenAPI Integration)

**utoipa v5+** (Compile-time OpenAPI generation)

```toml
[dependencies]
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid"] }
utoipa-swagger-ui = { version = "8", features = ["axum"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Complete API Schema Definition:**
```rust
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        chat_completion,
        embeddings,
        models_list,
    ),
    components(
        schemas(
            ChatCompletionRequest,
            ChatCompletionResponse,
            Message,
            EmbeddingRequest,
            EmbeddingResponse,
        )
    ),
    tags(
        (name = "chat", description = "Chat completion endpoints"),
        (name = "embeddings", description = "Text embedding endpoints"),
        (name = "models", description = "Model management endpoints"),
    ),
    info(
        title = "LLM Edge Agent API",
        version = "1.0.0",
        description = "High-performance LLM proxy with caching and routing",
        contact(
            name = "API Support",
            email = "support@example.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "https://api.example.com/v1", description = "Production"),
        (url = "http://localhost:8080/v1", description = "Local development")
    )
)]
struct ApiDoc;

// Request/Response schemas with full documentation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct ChatCompletionRequest {
    /// The model to use for completion
    #[schema(example = "gpt-4")]
    model: String,

    /// List of messages in the conversation
    messages: Vec<Message>,

    /// Sampling temperature (0-2)
    #[schema(minimum = 0.0, maximum = 2.0, example = 0.7)]
    #[serde(default)]
    temperature: Option<f32>,

    /// Maximum tokens to generate
    #[schema(minimum = 1, maximum = 4096, example = 1000)]
    #[serde(default)]
    max_tokens: Option<u32>,

    /// Enable streaming responses
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Message {
    /// Role of the message sender
    #[schema(example = "user")]
    role: MessageRole,

    /// Content of the message
    #[schema(example = "Hello, how are you?")]
    content: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum MessageRole {
    System,
    User,
    Assistant,
}

// API endpoint with OpenAPI documentation
#[utoipa::path(
    post,
    path = "/chat/completions",
    tag = "chat",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "Successful completion", body = ChatCompletionResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    )
)]
async fn chat_completion(
    State(state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ApiError> {
    // Implementation
}

// Integrate Swagger UI
async fn create_app() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/chat/completions", post(chat_completion))
        // ... other routes
}
```

**Alternative: oasgen** (Less intrusive)
```toml
[dependencies]
oasgen = "0.22"
```

Respects existing serde attributes with minimal additional annotations.

---

## 9. HTTP Client Configuration

### reqwest (High-level Client)

**reqwest v0.12+**

```toml
[dependencies]
reqwest = { version = "0.12", features = [
    "json",
    "stream",
    "rustls-tls",
    "gzip",
    "brotli",
    "http2",
] }
```

**Production Client Configuration:**
```rust
use reqwest::{Client, ClientBuilder};
use std::time::Duration;

fn create_llm_client() -> Result<Client> {
    ClientBuilder::new()
        // Connection pooling
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))

        // Timeouts
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))

        // HTTP/2
        .http2_prior_knowledge()
        .http2_adaptive_window(true)
        .http2_keep_alive_interval(Duration::from_secs(30))

        // Compression
        .gzip(true)
        .brotli(true)

        // TLS with Rustls
        .use_rustls_tls()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)

        // Retry configuration (use reqwest-middleware for this)
        .build()
}
```

**Streaming Support:**
```rust
async fn stream_completion(
    client: &Client,
    request: ChatCompletionRequest,
) -> Result<impl Stream<Item = Result<ChatChunk>>> {
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .json(&request)
        .send()
        .await?;

    let stream = response
        .bytes_stream()
        .map(|chunk| {
            // Parse SSE chunks
            parse_sse_chunk(chunk?)
        });

    Ok(stream)
}
```

**Retry Middleware:**
```toml
[dependencies]
reqwest-middleware = "0.4"
reqwest-retry = "0.7"
```

```rust
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

fn create_resilient_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .build_with_max_retries(3);

    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}
```

---

## 10. Authentication & Authorization

### JWT with jsonwebtoken

**jsonwebtoken v9+**

```toml
[dependencies]
jsonwebtoken = "9"
argon2 = "0.5" # For password hashing
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = "0.4"
```

**Complete JWT Implementation:**
```rust
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,          // Subject (user ID)
    exp: i64,             // Expiration time
    iat: i64,             // Issued at
    tier: Tier,           // User tier for rate limiting
    permissions: Vec<String>,
}

impl Claims {
    fn new(user_id: String, tier: Tier, permissions: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            exp: (now + Duration::hours(24)).timestamp(),
            iat: now.timestamp(),
            tier,
            permissions,
        }
    }
}

// JWT middleware for Axum
#[derive(Clone)]
struct JwtAuth {
    secret: Vec<u8>,
}

impl JwtAuth {
    fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    fn verify(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )?;
        Ok(token_data.claims)
    }

    fn create(&self, user_id: String, tier: Tier, permissions: Vec<String>) -> Result<String> {
        let claims = Claims::new(user_id, tier, permissions);
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )?;
        Ok(token)
    }
}

// Axum extractor for authentication
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract from Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        // Get JWT auth from app state
        let Extension(jwt_auth) = Extension::<JwtAuth>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Internal)?;

        // Verify token
        jwt_auth.verify(token)
            .map_err(|_| ApiError::Unauthorized)
    }
}

// Usage in route handlers
async fn protected_endpoint(
    claims: Claims, // Automatically extracted and validated
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>> {
    // claims.tier, claims.permissions available here
    process_request(request, claims.tier).await
}
```

**API Key Management:**
```rust
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

#[derive(Clone)]
struct ApiKeyAuth {
    // In production, use Redis or database
    keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
}

#[derive(Debug, Clone)]
struct ApiKeyInfo {
    user_id: String,
    tier: Tier,
    permissions: Vec<String>,
    created_at: DateTime<Utc>,
    last_used: DateTime<Utc>,
}

impl ApiKeyAuth {
    async fn verify(&self, api_key: &str) -> Result<ApiKeyInfo> {
        let keys = self.keys.read().await;
        keys.get(api_key)
            .cloned()
            .ok_or(ApiError::Unauthorized)
    }

    async fn create(&self, user_id: String, tier: Tier) -> Result<String> {
        let api_key = format!("sk_{}", uuid::Uuid::new_v4());
        let info = ApiKeyInfo {
            user_id,
            tier,
            permissions: vec!["chat.completions".to_string(), "embeddings".to_string()],
            created_at: Utc::now(),
            last_used: Utc::now(),
        };

        let mut keys = self.keys.write().await;
        keys.insert(api_key.clone(), info);

        Ok(api_key)
    }
}
```

---

## 11. Configuration Management

### Figment (Hierarchical Configuration)

**figment v0.10+**

```toml
[dependencies]
figment = { version = "0.10", features = ["toml", "env", "json"] }
serde = { version = "1.0", features = ["derive"] }
```

**Configuration Structure:**
```rust
use figment::{Figment, providers::{Format, Toml, Env}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    server: ServerConfig,
    cache: CacheConfig,
    backends: BackendsConfig,
    observability: ObservabilityConfig,
    rate_limiting: RateLimitingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct ServerConfig {
    host: String,
    port: u16,
    workers: usize,
    max_connections: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct CacheConfig {
    max_capacity: u64,
    ttl_seconds: u64,
    memory_limit_mb: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct BackendsConfig {
    openai: BackendConfig,
    anthropic: BackendConfig,
    local: Option<BackendConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BackendConfig {
    endpoint: String,
    api_key: String,
    timeout_seconds: u64,
    max_retries: u32,
}

impl Config {
    fn load() -> Result<Self> {
        Figment::new()
            // Base configuration
            .merge(Toml::file("config/default.toml"))
            // Environment-specific
            .merge(Toml::file(format!(
                "config/{}.toml",
                std::env::var("ENVIRONMENT").unwrap_or("development".to_string())
            )))
            // Environment variables override
            .merge(Env::prefixed("LLM_AGENT_").split("__"))
            .extract()
            .map_err(Into::into)
    }
}
```

**Configuration Files:**

`config/default.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 8
max_connections = 10000

[cache]
max_capacity = 10000
ttl_seconds = 300
memory_limit_mb = 1024

[backends.openai]
endpoint = "https://api.openai.com/v1"
api_key = ""
timeout_seconds = 30
max_retries = 3

[backends.anthropic]
endpoint = "https://api.anthropic.com/v1"
api_key = ""
timeout_seconds = 30
max_retries = 3

[observability]
otlp_endpoint = "http://localhost:4317"
metrics_port = 9090
log_level = "info"

[rate_limiting.free]
requests_per_second = 10
daily_quota = 1000
```

`config/production.toml`:
```toml
[server]
workers = 16
max_connections = 50000

[cache]
max_capacity = 100000
memory_limit_mb = 4096

[observability]
log_level = "warn"
```

**Environment Variable Override:**
```bash
# Override backend API key
export LLM_AGENT__BACKENDS__OPENAI__API_KEY="sk-..."

# Override server port
export LLM_AGENT__SERVER__PORT=9000
```

---

## 12. Security Hardening

### Production Security Best Practices

**1. Dependency Auditing**

```toml
# .cargo/audit.toml
[advisories]
ignore = []

[licenses]
deny = ["GPL-3.0"]
```

```bash
# Install cargo-audit
cargo install cargo-audit

# Run in CI/CD
cargo audit
```

**2. TLS Configuration with Rustls**

```toml
[dependencies]
rustls = "0.23"
rustls-pemfile = "2.0"
tokio-rustls = "0.26"
```

```rust
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls::server::AllowAnyAuthenticatedClient;
use std::sync::Arc;

fn create_tls_config() -> Result<Arc<ServerConfig>> {
    let certs = load_certs("certs/server.pem")?;
    let key = load_private_key("certs/server.key")?;

    let mut config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    // Enable ALPN for HTTP/2
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}
```

**3. Input Validation with validator**

```toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }
```

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
struct ChatCompletionRequest {
    #[validate(length(min = 1, max = 100))]
    model: String,

    #[validate(length(min = 1, max = 100))]
    messages: Vec<Message>,

    #[validate(range(min = 0.0, max = 2.0))]
    temperature: Option<f32>,

    #[validate(range(min = 1, max = 4096))]
    max_tokens: Option<u32>,
}

async fn validate_request(
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionRequest>> {
    request.validate()
        .map_err(|e| ApiError::ValidationError(e))?;
    Ok(Json(request))
}
```

**4. Secret Management**

```toml
[dependencies]
secrecy = "0.8"
```

```rust
use secrecy::{Secret, ExposeSecret};

#[derive(Clone)]
struct Secrets {
    jwt_secret: Secret<String>,
    openai_key: Secret<String>,
    anthropic_key: Secret<String>,
}

impl Secrets {
    fn from_env() -> Result<Self> {
        Ok(Self {
            jwt_secret: Secret::new(
                std::env::var("JWT_SECRET")
                    .map_err(|_| Error::MissingSecret("JWT_SECRET"))?
            ),
            openai_key: Secret::new(
                std::env::var("OPENAI_API_KEY")
                    .map_err(|_| Error::MissingSecret("OPENAI_API_KEY"))?
            ),
            anthropic_key: Secret::new(
                std::env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| Error::MissingSecret("ANTHROPIC_API_KEY"))?
            ),
        })
    }
}

// Usage - secrets never logged or printed
async fn call_openai(secrets: &Secrets, request: Request) -> Result<Response> {
    let client = reqwest::Client::new();
    client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", secrets.openai_key.expose_secret()))
        .json(&request)
        .send()
        .await
}
```

**5. Request Size Limits**

```rust
use tower_http::limit::RequestBodyLimitLayer;

let app = Router::new()
    .route("/v1/chat/completions", post(chat_completion))
    .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)); // 10MB max
```

**6. CORS Configuration**

```rust
use tower_http::cors::{CorsLayer, Any};
use http::header::{AUTHORIZATION, CONTENT_TYPE};

let cors = CorsLayer::new()
    .allow_origin(["https://example.com".parse().unwrap()])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .max_age(Duration::from_secs(3600));
```

**7. Security Headers**

```rust
use tower_http::set_header::SetResponseHeaderLayer;

let security_headers = ServiceBuilder::new()
    .layer(SetResponseHeaderLayer::overriding(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        HeaderValue::from_name("X-XSS-Protection").unwrap(),
        HeaderValue::from_static("1; mode=block"),
    ));
```

---

## 13. Testing Strategy

### Comprehensive Test Suite

**Test Dependencies:**
```toml
[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
criterion = "0.5"
mockall = "0.13"
proptest = "1.4"
rstest = "0.22"
```

**1. Unit Tests with Mocking**

```rust
use mockall::{automock, predicate::*};

#[automock]
#[async_trait]
trait LLMBackend: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_completion_with_cache_hit() {
        let mut mock_backend = MockLLMBackend::new();

        // Should not be called due to cache hit
        mock_backend
            .expect_complete()
            .times(0);

        let cache = Cache::new(100);
        cache.insert("key", response.clone()).await;

        let service = CachedLLMService::new(mock_backend, cache);
        let result = service.complete(request).await;

        assert_eq!(result.unwrap(), response);
    }
}
```

**2. Integration Tests with WireMock**

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

#[tokio::test]
async fn test_openai_integration() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Setup mock response
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                }
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Test with mock
    let client = create_client(&mock_server.uri());
    let response = client.complete(request).await.unwrap();

    assert_eq!(response.content, "Hello!");
}
```

**3. Property-Based Testing**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_cache_key_generation(
        model in "[a-z]{3,10}",
        prompt in ".{1,100}",
        temp in 0.0f32..2.0f32
    ) {
        let key1 = generate_cache_key(&model, &prompt, temp);
        let key2 = generate_cache_key(&model, &prompt, temp);

        // Same inputs should produce same key
        prop_assert_eq!(key1, key2);

        // Different temperature should produce different key
        let key3 = generate_cache_key(&model, &prompt, temp + 0.1);
        prop_assert_ne!(key1, key3);
    }
}
```

**4. Performance Benchmarks**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_cache_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let cache = Cache::new(10000);

    c.bench_function("cache insert", |b| {
        b.to_async(&runtime).iter(|| async {
            cache.insert(
                black_box("key"),
                black_box("value")
            ).await
        })
    });

    c.bench_function("cache get", |b| {
        b.to_async(&runtime).iter(|| async {
            cache.get(black_box("key")).await
        })
    });
}

criterion_group!(benches, bench_cache_operations);
criterion_main!(benches);
```

**5. Load Testing**

```rust
use tokio::time::{sleep, Duration};
use std::sync::atomic::{AtomicU64, Ordering};

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn load_test_concurrent_requests() {
    let server = start_test_server().await;
    let successes = Arc::new(AtomicU64::new(0));
    let failures = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    // Spawn 1000 concurrent requests
    for _ in 0..1000 {
        let successes = successes.clone();
        let failures = failures.clone();

        handles.push(tokio::spawn(async move {
            match make_request().await {
                Ok(_) => successes.fetch_add(1, Ordering::Relaxed),
                Err(_) => failures.fetch_add(1, Ordering::Relaxed),
            };
        }));
    }

    // Wait for all requests
    for handle in handles {
        handle.await.unwrap();
    }

    let total_successes = successes.load(Ordering::Relaxed);
    let total_failures = failures.load(Ordering::Relaxed);

    assert!(total_successes > 950); // >95% success rate
    println!("Successes: {}, Failures: {}", total_successes, total_failures);
}
```

---

## 14. Performance Optimization Patterns

### 1. Connection Pooling

```rust
use deadpool::managed::{Pool, Manager};

#[derive(Clone)]
struct LLMClientManager {
    endpoint: String,
    api_key: String,
}

#[async_trait]
impl Manager for LLMClientManager {
    type Type = reqwest::Client;
    type Error = Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(reqwest::Client::new())
    }

    async fn recycle(&self, _conn: &mut Self::Type) -> Result<(), Self::Error> {
        Ok(())
    }
}

async fn create_connection_pool() -> Pool<LLMClientManager> {
    let manager = LLMClientManager {
        endpoint: "https://api.openai.com".to_string(),
        api_key: std::env::var("OPENAI_API_KEY").unwrap(),
    };

    Pool::builder(manager)
        .max_size(100)
        .build()
        .unwrap()
}
```

### 2. Request Batching

```rust
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

struct BatchProcessor {
    sender: mpsc::Sender<(Request, oneshot::Sender<Response>)>,
}

impl BatchProcessor {
    fn new(batch_size: usize, timeout: Duration) -> Self {
        let (tx, mut rx) = mpsc::channel::<(Request, oneshot::Sender<Response>)>(1000);

        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut interval = interval(timeout);

            loop {
                tokio::select! {
                    Some(item) = rx.recv() => {
                        batch.push(item);
                        if batch.len() >= batch_size {
                            Self::process_batch(&mut batch).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            Self::process_batch(&mut batch).await;
                        }
                    }
                }
            }
        });

        Self { sender: tx }
    }

    async fn process_batch(batch: &mut Vec<(Request, oneshot::Sender<Response>)>) {
        let requests: Vec<_> = batch.iter().map(|(req, _)| req.clone()).collect();
        let responses = batch_api_call(requests).await;

        for ((_, tx), response) in batch.drain(..).zip(responses) {
            let _ = tx.send(response);
        }
    }
}
```

### 3. Zero-Copy Serialization with simd-json

```toml
[dependencies]
simd-json = "0.13"
```

```rust
use simd_json;

async fn parse_response(body: &mut [u8]) -> Result<Response> {
    // Mutates input for zero-copy parsing
    let parsed = simd_json::to_borrowed_value(body)?;
    Ok(Response::from_value(parsed)?)
}
```

### 4. Memory Pool for Allocations

```rust
use typed_arena::Arena;

struct RequestProcessor {
    arena: Arena<Request>,
}

impl RequestProcessor {
    fn process(&self, data: Vec<u8>) -> &Request {
        let request = parse_request(data);
        self.arena.alloc(request)
    }
}
```

### 5. Async Batching with Stream Processing

```rust
use futures::stream::{Stream, StreamExt};

async fn process_streaming_response(
    stream: impl Stream<Item = Result<Bytes>>,
) -> Result<String> {
    stream
        .try_fold(String::new(), |mut acc, chunk| async move {
            if let Ok(text) = std::str::from_utf8(&chunk) {
                acc.push_str(text);
            }
            Ok(acc)
        })
        .await
}
```

---

## 15. Complete Cargo.toml

```toml
[package]
name = "llm-edge-agent"
version = "1.0.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# Web framework
axum = { version = "0.8", features = ["macros", "ws", "http2"] }
hyper = { version = "1.0", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = [
    "trace",
    "timeout",
    "compression-full",
    "cors",
    "limit",
    "set-header",
] }

# Async runtime
tokio = { version = "1.40", features = ["full", "tracing"] }
tokio-util = { version = "0.7", features = ["codec", "io"] }
tokio-stream = "0.1"
futures = "0.3"

# HTTP client
reqwest = { version = "0.12", features = [
    "json",
    "stream",
    "rustls-tls",
    "gzip",
    "brotli",
    "http2",
] }
reqwest-middleware = "0.4"
reqwest-retry = "0.7"

# Caching
moka = { version = "0.12", features = ["future"] }
mini-moka = "0.10"

# Rate limiting
tower-governor = "0.8"
governor = "0.6"

# Circuit breaker
failsafe = { version = "1.3", features = ["futures"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
simd-json = "0.13"

# Configuration
figment = { version = "0.10", features = ["toml", "env", "json"] }

# OpenAPI / Documentation
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid"] }
utoipa-swagger-ui = { version = "8", features = ["axum"] }

# Authentication
jsonwebtoken = "9"
argon2 = "0.5"

# Observability
opentelemetry = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["grpc"] }
opentelemetry_sdk = { version = "0.26", features = ["rt-tokio"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.27"
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# TLS
rustls = "0.23"
rustls-pemfile = "2.0"
tokio-rustls = "0.26"

# Security
secrecy = "0.8"
validator = { version = "0.18", features = ["derive"] }

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "1.0"
dashmap = "6.0"
bytes = "1.0"
http = "1.0"
async-trait = "0.1"

# Optional: Semantic routing
candle-core = { version = "0.7", optional = true }
candle-nn = { version = "0.7", optional = true }
tokenizers = { version = "0.20", optional = true }

[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
criterion = { version = "0.5", features = ["async_tokio"] }
mockall = "0.13"
proptest = "1.4"
rstest = "0.22"

[features]
default = []
semantic-routing = ["candle-core", "candle-nn", "tokenizers"]

[[bench]]
name = "cache_benchmark"
harness = false

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

[profile.dev]
opt-level = 0

[profile.test]
opt-level = 1
```

---

## 16. Project Structure

```
llm-edge-agent/
├── Cargo.toml
├── config/
│   ├── default.toml
│   ├── development.toml
│   └── production.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── chat.rs
│   │   ├── embeddings.rs
│   │   ├── models.rs
│   │   └── schema.rs
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs
│   │   └── api_key.rs
│   ├── cache/
│   │   ├── mod.rs
│   │   ├── response.rs
│   │   └── embedding.rs
│   ├── backends/
│   │   ├── mod.rs
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   └── local.rs
│   ├── routing/
│   │   ├── mod.rs
│   │   ├── load_balancer.rs
│   │   ├── semantic.rs
│   │   └── strategies.rs
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── rate_limit.rs
│   │   ├── circuit_breaker.rs
│   │   └── timeout.rs
│   ├── observability/
│   │   ├── mod.rs
│   │   ├── tracing.rs
│   │   └── metrics.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs
│   └── error.rs
├── tests/
│   ├── integration/
│   │   ├── chat_test.rs
│   │   └── cache_test.rs
│   └── common/
│       └── mod.rs
├── benches/
│   ├── cache_benchmark.rs
│   └── routing_benchmark.rs
└── examples/
    ├── simple_proxy.rs
    └── semantic_routing.rs
```

---

## 17. Production Deployment Checklist

### Infrastructure
- [ ] Multi-region deployment with geographic load balancing
- [ ] Container orchestration (Kubernetes/ECS)
- [ ] Auto-scaling based on CPU/memory/request rate
- [ ] Health checks and readiness probes
- [ ] Graceful shutdown handling

### Monitoring
- [ ] OpenTelemetry traces exported to observability platform
- [ ] Prometheus metrics with Grafana dashboards
- [ ] Error tracking (e.g., Sentry)
- [ ] Log aggregation (e.g., Loki, CloudWatch)
- [ ] Alerting rules for SLO violations

### Security
- [ ] TLS 1.3 with strong cipher suites
- [ ] API key rotation policy
- [ ] Rate limiting per tier
- [ ] DDoS protection
- [ ] Regular security audits with `cargo audit`
- [ ] Secrets in secure vault (not environment variables)

### Performance
- [ ] Connection pooling tuned for workload
- [ ] Cache hit rate > 40% for common queries
- [ ] P95 latency < 500ms (excluding LLM processing)
- [ ] Circuit breakers configured for all backends
- [ ] Request/response compression enabled

### Reliability
- [ ] Circuit breakers on all external calls
- [ ] Retry logic with exponential backoff
- [ ] Fallback mechanisms for backend failures
- [ ] Database connection pooling
- [ ] Chaos engineering tests passed

---

## 18. Key Takeaways

### Why Rust for LLM Proxy?

1. **Performance**: 70% less CPU, 67% less memory vs traditional solutions
2. **Latency**: <100ms added overhead vs 200ms+ in other languages
3. **Memory Safety**: Eliminates entire classes of bugs
4. **Concurrency**: Fearless concurrency with compile-time guarantees
5. **Ecosystem**: Mature, production-ready crates available

### Critical Success Factors

1. **Use Axum + Tower**: Best performance/ergonomics tradeoff
2. **Enable Caching**: 40%+ cache hit rate dramatically improves cost/latency
3. **Implement Circuit Breakers**: Essential for backend failures
4. **Comprehensive Observability**: Can't fix what you can't measure
5. **Rate Limiting**: Protect backends and manage costs
6. **Security First**: Audit dependencies, use Rustls, validate inputs

### Performance Targets

- **P50 Latency**: < 50ms (proxy overhead)
- **P95 Latency**: < 150ms (proxy overhead)
- **Throughput**: > 10,000 RPS per instance
- **Memory**: < 500MB per instance (excluding cache)
- **Cache Hit Rate**: > 40%
- **Error Rate**: < 0.1%

### Recommended First Implementation

Start with this minimal but production-ready stack:

1. Axum + Tower + Tokio (web framework)
2. Moka (caching)
3. tower-governor (rate limiting)
4. reqwest (HTTP client)
5. OpenTelemetry (observability)
6. figment (configuration)
7. jsonwebtoken (authentication)

This provides a solid foundation that can scale to millions of requests while maintaining low latency and high reliability.

---

## References

- Axum HTTP Proxy Example: https://github.com/tokio-rs/axum/blob/main/examples/http-proxy/src/main.rs
- Cloudflare Pingora Case Study: https://blog.cloudflare.com/pingora-open-source
- Helicone AI Gateway: https://www.helicone.ai/blog/top-llm-gateways-comparison-2025
- Tower Middleware: https://docs.rs/tower/latest/tower/
- OpenTelemetry Rust: https://opentelemetry.io/docs/languages/rust/
- Moka Cache: https://github.com/moka-rs/moka
- Production Rust at Scale: https://rust-trends.com/newsletter/production-rust-internet-scale/
