//! LLM-Edge-Agent Unified Service - Phase 3 Layer 1
//!
//! Phase 3 - Automation & Resilience (Layer 1) hardened service.
//!
//! # Phase 3 Requirements
//!
//! - AGENT_PHASE=phase3, AGENT_LAYER=layer1 enforcement
//! - Agents MUST: Coordinate, Route, Optimize, Escalate
//! - Agents MUST NOT: Make final decisions, Emit executive conclusions
//! - Performance budgets: MAX_TOKENS=1500, MAX_LATENCY_MS=3000, MAX_CALLS_PER_RUN=4
//! - DecisionEvent signals: execution_strategy_signal, optimization_signal, incident_signal
//! - HARD FAIL on: Ruvector unavailable, Execution guard violated
//! - Crashloop on misconfiguration
//!
//! # Architecture
//!
//! - ONE service, multiple agent endpoints
//! - Stateless execution at runtime
//! - All persistence via ruvector-service (NO direct SQL)
//! - Phase 3 signals with confidence + references

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
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use llm_edge_agents::{
    // Phase 3 components
    phase3::{
        Phase3StartupGuard, Phase3Config, BudgetEnforcer, PerformanceBudget,
        ExecutionRoleGuard, SignalEmitter, SignalReference, Phase3Signal,
    },
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
const PHASE: &str = "phase3";
const LAYER: &str = "layer1";

/// Phase 3 application state
#[derive(Clone)]
pub struct Phase3ServiceState {
    pub service_name: String,
    pub service_version: String,
    pub platform_env: String,
    pub phase: String,
    pub layer: String,
    pub budget: PerformanceBudget,
    pub ruvector_url: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing FIRST for visibility into failures
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
        phase = PHASE,
        layer = LAYER,
        "Starting LLM-Edge-Agent Unified Service - Phase 3 Layer 1"
    );

    // =========================================================================
    // PHASE 3 STARTUP HARDENING
    // Service WILL crashloop on misconfiguration
    // =========================================================================

    let startup_guard = match Phase3StartupGuard::new() {
        Ok(guard) => guard,
        Err(e) => {
            error!("FATAL: Phase 3 startup validation failed: {}", e);
            error!("Service will exit with code 1 (crashloop behavior)");
            error!("Fix configuration and redeploy:");
            error!("  - AGENT_PHASE must be 'phase3'");
            error!("  - AGENT_LAYER must be 'layer1'");
            error!("  - RUVECTOR_SERVICE_URL must be set");
            error!("  - RUVECTOR_API_KEY must be set (from Secret Manager)");
            std::process::exit(1);
        }
    };

    // Validate RuVector connectivity - HARD FAIL if unavailable
    if let Err(e) = startup_guard.run_validations().await {
        error!("FATAL: Phase 3 startup validation failed: {}", e);
        error!("RuVector service is REQUIRED for Phase 3 operation");
        error!("Service will exit with code 1 (crashloop behavior)");
        std::process::exit(1);
    }

    let config = startup_guard.config();

    info!(
        phase = %config.agent_phase,
        layer = %config.agent_layer,
        ruvector_url = %config.ruvector_url,
        max_tokens = config.max_tokens,
        max_latency_ms = config.max_latency_ms,
        max_calls_per_run = config.max_calls_per_run,
        "Phase 3 configuration validated successfully"
    );

    // Create shared state
    let state = Phase3ServiceState {
        service_name: SERVICE_NAME.to_string(),
        service_version: SERVICE_VERSION.to_string(),
        platform_env: std::env::var("PLATFORM_ENV").unwrap_or_else(|_| "prod".to_string()),
        phase: PHASE.to_string(),
        layer: LAYER.to_string(),
        budget: PerformanceBudget {
            max_tokens: config.max_tokens,
            max_latency_ms: config.max_latency_ms,
            max_calls_per_run: config.max_calls_per_run,
        },
        ruvector_url: config.ruvector_url.clone(),
    };

    // Emit startup signal
    let signal_emitter = SignalEmitter::new(
        SERVICE_NAME,
        SERVICE_VERSION,
        &config.ruvector_url,
    );

    let startup_signal = signal_emitter.execution_strategy_signal(
        "service_startup",
        "llm-edge-agent",
        "Phase 3 Layer 1 service started successfully",
        0.95,
        vec![
            SignalReference::event("startup", "Service initialization complete"),
            SignalReference::metric("health_check", "RuVector connectivity verified"),
        ],
    );

    if let Err(e) = signal_emitter.emit(&startup_signal).await {
        warn!("Failed to emit startup signal (non-fatal): {}", e);
    }

    // Build the unified router
    let app = build_phase3_router(config, state.clone());

    // Start the HTTP server
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!(address = %addr, "Starting HTTP server");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("FATAL: Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    info!(
        phase = PHASE,
        layer = LAYER,
        "LLM-Edge-Agent Phase 3 Layer 1 Service is ready!"
    );

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Build the Phase 3 unified router with all hardening
fn build_phase3_router(config: &Phase3Config, state: Phase3ServiceState) -> Router {
    // Build agent routers with Phase 3 constraints
    let tool_invocation_router = tool_invocation_handler();
    let circuit_breaker_router = build_circuit_breaker_router();
    let failover_router = build_failover_router(&config.ruvector_url);
    let execution_guard_router = execution_guard_handler();
    let caching_strategy_router = build_caching_strategy_router();

    // Build stateful routes (these need Phase3ServiceState)
    let stateful_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/", get(service_info_handler))
        .route("/phase3/status", get(phase3_status_handler))
        .route("/phase3/budgets", get(phase3_budgets_handler))
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
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_millis(
            config.max_latency_ms as u64
        )))
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
    phase: String,
    layer: String,
    timestamp: String,
}

async fn health_handler(State(state): State<Phase3ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            service: state.service_name,
            version: state.service_version,
            phase: state.phase,
            layer: state.layer,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn readiness_handler(State(state): State<Phase3ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ready": true,
            "service": state.service_name,
            "version": state.service_version,
            "phase": state.phase,
            "layer": state.layer,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

async fn liveness_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "alive": true,
            "phase": PHASE,
            "layer": LAYER,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

async fn metrics_handler() -> String {
    format!(
        r#"# HELP llm_edge_agent_info Service information
# TYPE llm_edge_agent_info gauge
llm_edge_agent_info{{service="{}",version="{}",phase="{}",layer="{}"}} 1

# HELP llm_edge_agent_up Service availability
# TYPE llm_edge_agent_up gauge
llm_edge_agent_up 1

# HELP llm_edge_agent_phase3_budgets Performance budget configuration
# TYPE llm_edge_agent_phase3_budgets gauge
llm_edge_agent_phase3_max_tokens 1500
llm_edge_agent_phase3_max_latency_ms 3000
llm_edge_agent_phase3_max_calls_per_run 4
"#,
        SERVICE_NAME, SERVICE_VERSION, PHASE, LAYER
    )
}

/// Phase 3 status endpoint
async fn phase3_status_handler(State(state): State<Phase3ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "phase": state.phase,
            "layer": state.layer,
            "status": "operational",
            "roles": {
                "allowed": ["coordinate", "route", "optimize", "escalate"],
                "forbidden": ["final_decision", "executive_conclusion"]
            },
            "signals": {
                "emitted": ["execution_strategy_signal", "optimization_signal", "incident_signal"],
                "requirements": ["confidence", "references"]
            },
            "ruvector": {
                "url": state.ruvector_url,
                "required": true
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

/// Phase 3 performance budgets endpoint
async fn phase3_budgets_handler(State(state): State<Phase3ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "phase": state.phase,
            "layer": state.layer,
            "budgets": {
                "max_tokens": state.budget.max_tokens,
                "max_latency_ms": state.budget.max_latency_ms,
                "max_calls_per_run": state.budget.max_calls_per_run
            },
            "enforcement": "strict",
            "failure_semantics": "hard_fail",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

#[derive(Serialize)]
struct ServiceInfo {
    service_name: String,
    version: String,
    phase: String,
    layer: String,
    platform_env: String,
    agents: Vec<AgentEndpoint>,
    phase3_endpoints: Vec<String>,
    health_endpoints: Vec<String>,
    documentation: String,
}

#[derive(Serialize)]
struct AgentEndpoint {
    name: String,
    classification: String,
    base_path: String,
    phase3_role: String,
    endpoints: Vec<String>,
}

async fn service_info_handler(State(state): State<Phase3ServiceState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ServiceInfo {
            service_name: state.service_name,
            version: state.service_version,
            phase: state.phase,
            layer: state.layer,
            platform_env: state.platform_env,
            agents: vec![
                AgentEndpoint {
                    name: "Tool Invocation Agent".to_string(),
                    classification: "EXECUTION CONTROL".to_string(),
                    base_path: "/agents/tool-invocation".to_string(),
                    phase3_role: "coordinate".to_string(),
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
                    phase3_role: "escalate".to_string(),
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
                    phase3_role: "route".to_string(),
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
                    phase3_role: "coordinate".to_string(),
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
                    phase3_role: "optimize".to_string(),
                    endpoints: vec![
                        "POST /cache-strategy".to_string(),
                        "POST /cache-strategy/test".to_string(),
                        "POST /cache-strategy/simulate".to_string(),
                        "GET /cache-strategy/inspect".to_string(),
                    ],
                },
            ],
            phase3_endpoints: vec![
                "/phase3/status".to_string(),
                "/phase3/budgets".to_string(),
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
