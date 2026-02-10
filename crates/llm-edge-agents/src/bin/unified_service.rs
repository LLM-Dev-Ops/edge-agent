//! LLM-Edge-Agent Unified Service
//!
//! This binary combines ALL edge agents into a single unified Google Cloud Run service.
//! All agents share the same runtime, configuration, and telemetry stack.
//!
//! # Agentics Foundational Execution Unit
//!
//! This service is instrumented as a Foundational Execution Unit within the Agentics
//! execution system. Every agent invocation:
//! - REQUIRES `X-Parent-Span-Id` header (rejected without it)
//! - Creates a repo-level execution span
//! - Creates an agent-level execution span per agent
//! - Returns the full span graph in the response
//!
//! # Architecture
//!
//! - ONE service, multiple agent endpoints
//! - Stateless execution at runtime
//! - All persistence via ruvector-service (NO direct SQL)
//! - DecisionEvent emission per invocation
//! - Execution span emission per invocation (Agentics contract)
//!
//! # Agent Endpoints
//!
//! - `/agents/tool-invocation/*` - Tool Invocation Agent
//! - `/agents/circuit-breaker/*` - Circuit Breaker Agent
//! - `/agents/failover/*` - Failover Agent
//! - `/agents/execution-guard/*` - Execution Guard Agent
//! - `/agents/caching-strategy/*` - Caching Strategy Agent
//!
//! # Health & Observability
//!
//! - `/health` - Service health check
//! - `/health/ready` - Readiness probe
//! - `/health/live` - Liveness probe
//! - `/metrics` - Prometheus metrics

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Serialize;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agentics_contracts::spans::SpanCollector;
use llm_edge_agents::{
    // Failover
    failover::{FailoverAgentConfig, FailoverAgent, FailoverRequest},
    // Tool Invocation
    tool_invocation::{ToolInvocationConfig, ToolInvocationAgent, ToolInvocationRequest},
    // Circuit Breaker
    circuit_breaker::{CircuitBreakerAgent, CircuitBreakerRequest},
    // Execution Guard
    execution_guard::{ExecutionGuardConfig, ExecutionGuardAgent, ExecutionGuardRequest},
    // Caching Strategy
    caching_strategy::{CacheStrategyConfig, CachingStrategyAgent, CacheStrategyRequest},
    // Span instrumentation
    span_instrumentation::with_agent_span,
    // Errors
    AgentError,
};

/// Service metadata
const SERVICE_NAME: &str = "llm-edge-agent";
const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application state for the unified service
#[derive(Clone)]
pub struct ServiceState {
    pub service_name: String,
    pub service_version: String,
    pub platform_env: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "llm_edge_agents=info,tower_http=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!(
        service = SERVICE_NAME,
        version = SERVICE_VERSION,
        "Starting LLM-Edge-Agent Unified Service"
    );

    // Load configuration from environment
    let config = ServiceConfig::from_env();
    info!(
        host = %config.host,
        port = config.port,
        platform_env = %config.platform_env,
        ruvector_url = %config.ruvector_url,
        "Configuration loaded"
    );

    // Create shared state
    let state = ServiceState {
        service_name: SERVICE_NAME.to_string(),
        service_version: SERVICE_VERSION.to_string(),
        platform_env: config.platform_env.clone(),
    };

    // Build the unified router
    let app = build_unified_router(&config, state.clone());

    // Start the HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!(address = %addr, "Starting HTTP server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("LLM-Edge-Agent Unified Service is ready!");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Service configuration from environment
#[derive(Debug, Clone)]
struct ServiceConfig {
    host: String,
    port: u16,
    platform_env: String,
    ruvector_url: String,
    #[allow(dead_code)]
    ruvector_api_key: Option<String>,
    #[allow(dead_code)]
    telemetry_endpoint: Option<String>,
}

impl ServiceConfig {
    fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            platform_env: std::env::var("PLATFORM_ENV").unwrap_or_else(|_| "dev".to_string()),
            ruvector_url: std::env::var("RUVECTOR_SERVICE_URL")
                .or_else(|_| std::env::var("RUVECTOR_ENDPOINT"))
                .unwrap_or_else(|_| "http://ruvector-service:3000".to_string()),
            ruvector_api_key: std::env::var("RUVECTOR_API_KEY").ok(),
            telemetry_endpoint: std::env::var("TELEMETRY_ENDPOINT")
                .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
                .ok(),
        }
    }
}

// =============================================================================
// Execution Span Middleware (Agentics Foundational Execution Unit)
// =============================================================================

/// Middleware that validates execution context headers and creates a SpanCollector.
///
/// Required headers for agent endpoints:
///   - `X-Parent-Span-Id`: The span ID from the calling Core (REQUIRED)
///   - `X-Execution-Id`: The execution ID from the caller (optional, generated if missing)
///
/// If `X-Parent-Span-Id` is missing, returns 400 Bad Request.
/// The SpanCollector is inserted into request extensions for downstream handlers.
async fn execution_span_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let parent_span_id = request
        .headers()
        .get("X-Parent-Span-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let execution_id = request
        .headers()
        .get("X-Execution-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // REJECT if parent_span_id is missing or empty
    let parent_span_id = match parent_span_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            warn!("Rejecting request: missing X-Parent-Span-Id header");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "MISSING_SPAN_CONTEXT",
                        "message": "X-Parent-Span-Id header is required for all agent invocations"
                    }
                })),
            )
                .into_response();
        }
    };

    let execution_id = execution_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    info!(
        execution_id = %execution_id,
        parent_span_id = %parent_span_id,
        "Creating execution span context"
    );

    // Create the span collector and insert into extensions
    let collector = SpanCollector::new(&execution_id, &parent_span_id);
    request.extensions_mut().insert(collector);

    next.run(request).await
}

// =============================================================================
// Instrumented Agent Handlers
// =============================================================================

/// Shared agent state for instrumented handlers
#[derive(Clone)]
struct InstrumentedAgentState {
    tool_invocation: Arc<ToolInvocationAgent>,
    circuit_breaker: Arc<CircuitBreakerAgent>,
    failover: Arc<FailoverAgent>,
    execution_guard: Arc<ExecutionGuardAgent>,
    caching_strategy: Arc<CachingStrategyAgent>,
}

/// Helper to map AgentError to HTTP status code
fn agent_error_status(e: &AgentError) -> StatusCode {
    match e {
        AgentError::Validation(_)
        | AgentError::SchemaValidation { .. }
        | AgentError::ValidationError(_) => StatusCode::BAD_REQUEST,
        AgentError::ToolNotAllowed(_)
        | AgentError::ToolBlocked { .. }
        | AgentError::ConstraintViolation { .. } => StatusCode::FORBIDDEN,
        AgentError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
        AgentError::MissingSpanContext(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Build a successful instrumented response with span graph
fn build_instrumented_response(
    data: serde_json::Value,
    collector: &SpanCollector,
) -> Response {
    match collector.finalize_cloned() {
        Ok(graph) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "data": data,
                "spans": graph,
            })),
        )
            .into_response(),
        Err(msg) => {
            error!(error = %msg, "Span graph validation failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "NO_AGENT_SPANS",
                        "message": msg,
                    },
                })),
            )
                .into_response()
        }
    }
}

/// Build an error response with span graph (spans still returned on failure)
fn build_instrumented_error_response(
    status: StatusCode,
    error_code: &str,
    error_message: &str,
    collector: &SpanCollector,
) -> Response {
    let graph = collector.finalize_failed_cloned(error_message);
    (
        status,
        Json(serde_json::json!({
            "success": false,
            "error": {
                "code": error_code,
                "message": error_message,
            },
            "spans": graph,
        })),
    )
        .into_response()
}

// -- Tool Invocation --

async fn instrumented_tool_invocation_invoke(
    State(agents): State<InstrumentedAgentState>,
    Extension(collector): Extension<SpanCollector>,
    Json(request): Json<ToolInvocationRequest>,
) -> Response {
    let agent = agents.tool_invocation.clone();
    let (result, _idx) =
        with_agent_span(&collector, "tool-invocation", || async move {
            agent.process(request).await
        })
        .await;

    match result {
        Ok(response) => {
            let data = serde_json::to_value(&response).unwrap_or_default();
            build_instrumented_response(data, &collector)
        }
        Err(e) => build_instrumented_error_response(
            agent_error_status(&e),
            e.error_code(),
            &e.to_string(),
            &collector,
        ),
    }
}

// -- Circuit Breaker --

async fn instrumented_circuit_breaker_invoke(
    State(agents): State<InstrumentedAgentState>,
    Extension(collector): Extension<SpanCollector>,
    Json(request): Json<CircuitBreakerRequest>,
) -> Response {
    let agent = agents.circuit_breaker.clone();
    let (result, _idx) =
        with_agent_span(&collector, "circuit-breaker", || async move {
            agent.invoke(request).await
        })
        .await;

    match result {
        Ok(response) => {
            let data = serde_json::to_value(&response).unwrap_or_default();
            build_instrumented_response(data, &collector)
        }
        Err(e) => build_instrumented_error_response(
            agent_error_status(&e),
            e.error_code(),
            &e.to_string(),
            &collector,
        ),
    }
}

// -- Failover --

async fn instrumented_failover_invoke(
    State(agents): State<InstrumentedAgentState>,
    Extension(collector): Extension<SpanCollector>,
    Json(request): Json<FailoverRequest>,
) -> Response {
    let agent = agents.failover.clone();
    let (result, _idx) =
        with_agent_span(&collector, "failover", || async move {
            agent.process(request).await
        })
        .await;

    match result {
        Ok(response) => {
            let data = serde_json::to_value(&response).unwrap_or_default();
            build_instrumented_response(data, &collector)
        }
        Err(e) => build_instrumented_error_response(
            agent_error_status(&e),
            e.error_code(),
            &e.to_string(),
            &collector,
        ),
    }
}

// -- Execution Guard --

async fn instrumented_execution_guard_invoke(
    State(agents): State<InstrumentedAgentState>,
    Extension(collector): Extension<SpanCollector>,
    Json(request): Json<ExecutionGuardRequest>,
) -> Response {
    let agent = agents.execution_guard.clone();
    let (result, _idx) =
        with_agent_span(&collector, "execution-guard", || async move {
            agent.process(request).await
        })
        .await;

    match result {
        Ok(response) => {
            let data = serde_json::to_value(&response).unwrap_or_default();
            build_instrumented_response(data, &collector)
        }
        Err(e) => build_instrumented_error_response(
            agent_error_status(&e),
            e.error_code(),
            &e.to_string(),
            &collector,
        ),
    }
}

// -- Caching Strategy --

async fn instrumented_caching_strategy_invoke(
    State(agents): State<InstrumentedAgentState>,
    Extension(collector): Extension<SpanCollector>,
    Json(request): Json<CacheStrategyRequest>,
) -> Response {
    let agent = agents.caching_strategy.clone();
    let (result, _idx) =
        with_agent_span(&collector, "caching-strategy", || async move {
            agent.process(request).await
        })
        .await;

    match result {
        Ok(response) => {
            let data = serde_json::to_value(&response).unwrap_or_default();
            build_instrumented_response(data, &collector)
        }
        Err(e) => build_instrumented_error_response(
            agent_error_status(&e),
            e.error_code(),
            &e.to_string(),
            &collector,
        ),
    }
}

// =============================================================================
// Router Construction
// =============================================================================

/// Build the unified router combining all agent endpoints.
///
/// Agent invocation endpoints are instrumented with execution span middleware.
/// Health, metrics, and info endpoints are NOT instrumented (no span context required).
fn build_unified_router(config: &ServiceConfig, state: ServiceState) -> Router {
    // Create shared agent instances
    let agents = InstrumentedAgentState {
        tool_invocation: Arc::new(ToolInvocationAgent::new(ToolInvocationConfig::default())),
        circuit_breaker: Arc::new(CircuitBreakerAgent::new(None)),
        failover: Arc::new(FailoverAgent::new(FailoverAgentConfig {
            ruvector_url: config.ruvector_url.clone(),
            emit_events: true,
            ..Default::default()
        })),
        execution_guard: Arc::new(ExecutionGuardAgent::new(ExecutionGuardConfig::default())),
        caching_strategy: Arc::new(CachingStrategyAgent::new(CacheStrategyConfig::default())),
    };

    // Instrumented agent invocation routes (require X-Parent-Span-Id)
    let instrumented_routes = Router::new()
        .route(
            "/agents/tool-invocation/invoke",
            post(instrumented_tool_invocation_invoke),
        )
        .route(
            "/agents/circuit-breaker/invoke",
            post(instrumented_circuit_breaker_invoke),
        )
        .route(
            "/agents/failover/failover",
            post(instrumented_failover_invoke),
        )
        .route(
            "/agents/execution-guard/guard",
            post(instrumented_execution_guard_invoke),
        )
        .route(
            "/agents/caching-strategy/cache-strategy",
            post(instrumented_caching_strategy_invoke),
        )
        .with_state(agents)
        .layer(middleware::from_fn(execution_span_middleware));

    // Non-instrumented agent routes (test/simulate/inspect/health per agent)
    // These remain accessible without span context for development and CLI use.
    let tool_invocation_secondary = llm_edge_agents::tool_invocation::tool_invocation_handler();
    let circuit_breaker_secondary = {
        let agent = CircuitBreakerAgent::new(None);
        llm_edge_agents::circuit_breaker::CircuitBreakerHandler::new(agent).router()
    };
    let failover_secondary = llm_edge_agents::failover::create_failover_router(
        FailoverAgentConfig {
            ruvector_url: config.ruvector_url.clone(),
            emit_events: true,
            ..Default::default()
        },
    );
    let execution_guard_secondary = llm_edge_agents::execution_guard::execution_guard_handler();
    let caching_strategy_secondary = {
        let cfg = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(cfg);
        let handler_state = llm_edge_agents::caching_strategy::CacheStrategyHandlerState {
            agent: Arc::new(agent),
        };
        llm_edge_agents::caching_strategy::cache_strategy_router(handler_state)
    };

    let secondary_routes = Router::new()
        .nest("/agents/tool-invocation", tool_invocation_secondary)
        .nest("/agents/circuit-breaker", circuit_breaker_secondary)
        .nest("/agents/failover", failover_secondary)
        .nest("/agents/execution-guard", execution_guard_secondary)
        .nest("/agents/caching-strategy", caching_strategy_secondary);

    // Build stateful routes (these need ServiceState)
    let stateful_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/", get(service_info_handler))
        .with_state(state);

    // Build stateless routes
    let stateless_routes = Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/metrics", get(metrics_handler));

    // Combine: instrumented routes take precedence (merged first),
    // then secondary (nested) routes, then health/info.
    // Axum resolves routes by specificity, so the flat instrumented routes
    // match the primary invoke endpoints, while nested secondary routes
    // match test/simulate/inspect/health sub-paths.
    instrumented_routes
        .merge(secondary_routes)
        .merge(stateful_routes)
        .merge(stateless_routes)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(30)))
}

// =============================================================================
// Health & Info Handlers
// =============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    timestamp: String,
}

async fn health_handler(State(state): State<ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            service: state.service_name,
            version: state.service_version,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn readiness_handler(State(state): State<ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ready": true,
            "service": state.service_name,
            "version": state.service_version,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

async fn liveness_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "alive": true,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

async fn metrics_handler() -> String {
    format!(
        r#"# HELP llm_edge_agent_info Service information
# TYPE llm_edge_agent_info gauge
llm_edge_agent_info{{service="{}",version="{}"}} 1

# HELP llm_edge_agent_up Service availability
# TYPE llm_edge_agent_up gauge
llm_edge_agent_up 1
"#,
        SERVICE_NAME, SERVICE_VERSION
    )
}

#[derive(Serialize)]
struct ServiceInfo {
    service_name: String,
    version: String,
    platform_env: String,
    agents: Vec<AgentEndpoint>,
    health_endpoints: Vec<String>,
    documentation: String,
}

#[derive(Serialize)]
struct AgentEndpoint {
    name: String,
    classification: String,
    base_path: String,
    endpoints: Vec<String>,
}

async fn service_info_handler(State(state): State<ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ServiceInfo {
            service_name: state.service_name,
            version: state.service_version,
            platform_env: state.platform_env,
            agents: vec![
                AgentEndpoint {
                    name: "Tool Invocation Agent".to_string(),
                    classification: "EXECUTION CONTROL".to_string(),
                    base_path: "/agents/tool-invocation".to_string(),
                    endpoints: vec![
                        "POST /invoke".to_string(),
                        "POST /test".to_string(),
                        "POST /simulate".to_string(),
                        "GET /inspect".to_string(),
                        "GET /health".to_string(),
                    ],
                },
                AgentEndpoint {
                    name: "Circuit Breaker Agent".to_string(),
                    classification: "PROTECTION / GUARD".to_string(),
                    base_path: "/agents/circuit-breaker".to_string(),
                    endpoints: vec![
                        "POST /invoke".to_string(),
                        "POST /test".to_string(),
                        "POST /simulate".to_string(),
                        "GET /inspect/:provider_id".to_string(),
                        "GET /health".to_string(),
                        "GET /metrics".to_string(),
                    ],
                },
                AgentEndpoint {
                    name: "Failover Agent".to_string(),
                    classification: "ROUTING".to_string(),
                    base_path: "/agents/failover".to_string(),
                    endpoints: vec![
                        "POST /failover".to_string(),
                        "POST /failover/test".to_string(),
                        "POST /failover/simulate".to_string(),
                        "GET /failover/inspect".to_string(),
                        "GET /failover/health".to_string(),
                        "GET /failover/info".to_string(),
                    ],
                },
                AgentEndpoint {
                    name: "Execution Guard Agent".to_string(),
                    classification: "PROTECTION / GUARD".to_string(),
                    base_path: "/agents/execution-guard".to_string(),
                    endpoints: vec![
                        "POST /guard".to_string(),
                        "POST /test".to_string(),
                        "POST /simulate".to_string(),
                        "GET /inspect".to_string(),
                        "GET /health".to_string(),
                    ],
                },
                AgentEndpoint {
                    name: "Caching Strategy Agent".to_string(),
                    classification: "EXECUTION CONTROL".to_string(),
                    base_path: "/agents/caching-strategy".to_string(),
                    endpoints: vec![
                        "POST /cache-strategy".to_string(),
                        "POST /cache-strategy/test".to_string(),
                        "POST /cache-strategy/simulate".to_string(),
                        "GET /cache-strategy/inspect".to_string(),
                    ],
                },
            ],
            health_endpoints: vec![
                "/health".to_string(),
                "/health/ready".to_string(),
                "/health/live".to_string(),
                "/metrics".to_string(),
            ],
            documentation: "https://github.com/globalbusinessadvisors/llm-edge-agent".to_string(),
        }),
    )
}
