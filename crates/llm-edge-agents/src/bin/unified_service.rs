//! LLM-Edge-Agent Unified Service
//!
//! This binary combines ALL edge agents into a single unified Google Cloud Run service.
//! All agents share the same runtime, configuration, and telemetry stack.
//!
//! # Architecture
//!
//! - ONE service, multiple agent endpoints
//! - Stateless execution at runtime
//! - All persistence via ruvector-service (NO direct SQL)
//! - DecisionEvent emission per invocation
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
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use llm_edge_agents::{
    // Failover
    failover::{FailoverAgentConfig, create_failover_router},
    // Tool Invocation
    tool_invocation::{ToolInvocationConfig, tool_invocation_handler},
    // Circuit Breaker
    circuit_breaker::{CircuitBreakerAgent, CircuitBreakerHandler},
    // Execution Guard
    execution_guard::{ExecutionGuardConfig, execution_guard_handler},
    // Caching Strategy
    caching_strategy::{CacheStrategyConfig, CachingStrategyAgent, CacheStrategyHandlerState, cache_strategy_router},
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

/// Build the unified router combining all agent endpoints
fn build_unified_router(config: &ServiceConfig, state: ServiceState) -> Router {
    // Build agent routers (each has their own internal state)
    let tool_invocation_router = tool_invocation_handler();
    let circuit_breaker_router = build_circuit_breaker_router();
    let failover_router = build_failover_router(&config.ruvector_url);
    let execution_guard_router = execution_guard_handler();
    let caching_strategy_router = build_caching_strategy_router();

    // Build stateful routes (these need ServiceState)
    let stateful_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/", get(service_info_handler))
        .with_state(state);

    // Build stateless routes and merge agent routers
    let stateless_routes = Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/metrics", get(metrics_handler))
        .nest("/agents/tool-invocation", tool_invocation_router)
        .nest("/agents/circuit-breaker", circuit_breaker_router)
        .nest("/agents/failover", failover_router)
        .nest("/agents/execution-guard", execution_guard_router)
        .nest("/agents/caching-strategy", caching_strategy_router);

    // Combine all routes and add middleware
    stateful_routes
        .merge(stateless_routes)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(30)))
}

fn build_circuit_breaker_router() -> Router {
    let agent = CircuitBreakerAgent::new(None);
    CircuitBreakerHandler::new(agent).router()
}

fn build_failover_router(ruvector_url: &str) -> Router {
    let config = FailoverAgentConfig {
        ruvector_url: ruvector_url.to_string(),
        emit_events: true,
        ..Default::default()
    };
    create_failover_router(config)
}

fn build_caching_strategy_router() -> Router {
    let config = CacheStrategyConfig::default();
    let agent = CachingStrategyAgent::new(config);
    let state = CacheStrategyHandlerState {
        agent: Arc::new(agent),
    };
    cache_strategy_router(state)
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
