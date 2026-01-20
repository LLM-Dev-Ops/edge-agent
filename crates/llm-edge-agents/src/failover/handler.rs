//! HTTP handler for Failover Agent Edge Function
//!
//! Exposes the Failover Agent as a Google Cloud Edge Function endpoint.
//! Provides test, simulate, and inspect CLI invocation modes.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, instrument};

use super::{
    CircuitState, FailoverAgent, FailoverAgentConfig, FailoverOutcome, FailoverPolicy,
    FailoverRequest, FailoverResponse, ProviderHealth, TargetProvider, AGENT_ID, AGENT_VERSION,
};
use crate::ExecutionContext;

/// Application state for the Edge Function
pub struct FailoverAppState {
    pub agent: FailoverAgent,
    pub config: FailoverAgentConfig,
}

/// Create the Axum router for the Failover Agent
pub fn create_failover_router(config: FailoverAgentConfig) -> Router {
    let agent = FailoverAgent::new(config.clone());
    let state = Arc::new(FailoverAppState { agent, config });

    Router::new()
        // Main failover endpoint
        .route("/failover", post(failover_handler))
        // CLI invocation endpoints
        .route("/failover/test", post(test_handler))
        .route("/failover/simulate", post(simulate_handler))
        .route("/failover/inspect", get(inspect_handler))
        // Health and metadata
        .route("/failover/health", get(health_handler))
        .route("/failover/info", get(info_handler))
        .with_state(state)
}

/// Main failover routing handler
///
/// POST /failover
/// Content-Type: application/json
///
/// Request body: FailoverRequest
/// Response body: FailoverResponse
#[instrument(skip(state, request), fields(agent_id = AGENT_ID))]
pub async fn failover_handler(
    State(state): State<Arc<FailoverAppState>>,
    Json(request): Json<FailoverRequest>,
) -> impl IntoResponse {
    info!(
        execution_ref = %request.execution_context.execution_ref,
        primary = %request.primary_target.provider_id,
        "Processing failover request"
    );

    match state.agent.process(request).await {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::success(response))),
        Err(e) => {
            error!(error = %e, "Failover processing failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<FailoverResponse>::error(e.to_string())),
            )
        }
    }
}

/// Test handler - validate failover logic with mock inputs
///
/// POST /failover/test
///
/// CLI: llm-edge-agent failover test --primary openai --alternates anthropic,google
#[instrument(skip(state))]
async fn test_handler(
    State(state): State<Arc<FailoverAppState>>,
    Json(params): Json<TestParams>,
) -> impl IntoResponse {
    info!("Running failover test");

    let ctx = ExecutionContext::new();
    let primary = TargetProvider {
        provider_id: format!("{}-primary", params.primary),
        provider_name: params.primary.clone(),
        model: params.model.clone().unwrap_or_else(|| "default".to_string()),
        endpoint: None,
        priority: 0,
        circuit_state: params.primary_circuit.unwrap_or(CircuitState::Closed),
        health: params.primary_health.unwrap_or(ProviderHealth::Healthy),
        metadata: Default::default(),
    };

    let alternates: Vec<TargetProvider> = params
        .alternates
        .iter()
        .enumerate()
        .map(|(i, name)| TargetProvider {
            provider_id: format!("{}-alt-{}", name, i),
            provider_name: name.clone(),
            model: params.model.clone().unwrap_or_else(|| "default".to_string()),
            endpoint: None,
            priority: (i + 1) as u32,
            circuit_state: CircuitState::Closed,
            health: ProviderHealth::Healthy,
            metadata: Default::default(),
        })
        .collect();

    let request = FailoverRequest::new(ctx, primary, alternates);

    match state.agent.process(request).await {
        Ok(response) => {
            let test_result = TestResult {
                passed: response.success,
                outcome: response.decision.outcome,
                selected_provider: response
                    .decision
                    .selected_target
                    .map(|t| t.provider_name),
                confidence: response.decision.confidence,
                constraints_evaluated: response.constraints_applied.len(),
                duration_us: response.duration_us,
            };
            (StatusCode::OK, Json(ApiResponse::success(test_result)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<TestResult>::error(e.to_string())),
        ),
    }
}

/// Simulate handler - run failover scenario without persistence
///
/// POST /failover/simulate
///
/// CLI: llm-edge-agent failover simulate --scenario primary-down
#[instrument(skip(state))]
async fn simulate_handler(
    State(state): State<Arc<FailoverAppState>>,
    Json(params): Json<SimulateParams>,
) -> impl IntoResponse {
    info!(scenario = %params.scenario, "Running failover simulation");

    // Create scenario-specific request
    let (primary, primary_circuit, primary_health) = match params.scenario.as_str() {
        "primary-down" => (
            TargetProvider::new("primary-1", "primary", "model-v1"),
            CircuitState::Open,
            ProviderHealth::Unhealthy,
        ),
        "primary-degraded" => (
            TargetProvider::new("primary-1", "primary", "model-v1"),
            CircuitState::Closed,
            ProviderHealth::Degraded,
        ),
        "all-down" => (
            TargetProvider::new("primary-1", "primary", "model-v1"),
            CircuitState::Open,
            ProviderHealth::Unhealthy,
        ),
        _ => (
            TargetProvider::new("primary-1", "primary", "model-v1"),
            CircuitState::Closed,
            ProviderHealth::Healthy,
        ),
    };

    let alternates = if params.scenario == "all-down" {
        vec![
            TargetProvider {
                provider_id: "alt-1".to_string(),
                provider_name: "alternate1".to_string(),
                model: "model-v1".to_string(),
                endpoint: None,
                priority: 1,
                circuit_state: CircuitState::Open,
                health: ProviderHealth::Unhealthy,
                metadata: Default::default(),
            },
            TargetProvider {
                provider_id: "alt-2".to_string(),
                provider_name: "alternate2".to_string(),
                model: "model-v1".to_string(),
                endpoint: None,
                priority: 2,
                circuit_state: CircuitState::Open,
                health: ProviderHealth::Unhealthy,
                metadata: Default::default(),
            },
        ]
    } else {
        vec![
            TargetProvider::new("alt-1", "alternate1", "model-v1"),
            TargetProvider::new("alt-2", "alternate2", "model-v1"),
        ]
    };

    let mut request = FailoverRequest::new(ExecutionContext::new(), primary, alternates)
        .with_provider_status("primary-1", primary_circuit, primary_health);

    if params.scenario == "all-down" {
        request = request
            .with_provider_status("alt-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("alt-2", CircuitState::Open, ProviderHealth::Unhealthy);
    }

    // Use a non-emitting agent for simulation
    let sim_agent = FailoverAgent::new(FailoverAgentConfig {
        emit_events: false,
        ..state.config.clone()
    });

    match sim_agent.process(request).await {
        Ok(response) => {
            let sim_result = SimulateResult {
                scenario: params.scenario,
                outcome: response.decision.outcome,
                selected_target: response.decision.selected_target,
                reason: response.decision.reason,
                evaluated_targets: response.decision.evaluated_targets,
                constraints_applied: response.constraints_applied,
                note: "Simulation mode - no events persisted to ruvector-service".to_string(),
            };
            (StatusCode::OK, Json(ApiResponse::success(sim_result)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<SimulateResult>::error(e.to_string())),
        ),
    }
}

/// Inspect handler - show current failover configuration
///
/// GET /failover/inspect?provider=openai
///
/// CLI: llm-edge-agent failover inspect --provider openai
#[instrument(skip(state))]
async fn inspect_handler(
    State(state): State<Arc<FailoverAppState>>,
    Query(params): Query<InspectParams>,
) -> impl IntoResponse {
    info!(provider = ?params.provider, "Inspecting failover configuration");

    let inspect_result = InspectResult {
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        agent_type: "ROUTING".to_string(),
        default_policy: state.config.default_policy.clone(),
        ruvector_url: state.config.ruvector_url.clone(),
        emit_events: state.config.emit_events,
        provider_filter: params.provider,
        non_responsibilities: vec![
            "Perform orchestration (LLM-Orchestrator responsibility)".to_string(),
            "Modify policies dynamically".to_string(),
            "Trigger retries directly (Orchestrator responsibility)".to_string(),
            "Emit alerts (Sentinel responsibility)".to_string(),
            "Perform analytics (Observatory responsibility)".to_string(),
            "Execute SQL directly (use ruvector-service)".to_string(),
        ],
    };

    (StatusCode::OK, Json(ApiResponse::success(inspect_result)))
}

/// Health check handler
///
/// GET /failover/health
async fn health_handler(State(state): State<Arc<FailoverAppState>>) -> impl IntoResponse {
    let ruvector_healthy = state
        .agent
        .ruvector_client()
        .health_check()
        .await
        .unwrap_or(false);

    let health = HealthStatus {
        status: if ruvector_healthy { "healthy" } else { "degraded" }.to_string(),
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        ruvector_healthy,
    };

    let status = if ruvector_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(health))
}

/// Info handler
///
/// GET /failover/info
async fn info_handler() -> impl IntoResponse {
    let info = AgentInfo {
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        agent_type: "ROUTING".to_string(),
        classification: "Routes execution to alternate backends when primary targets are unavailable".to_string(),
        decision_type: "failover_routing_decision".to_string(),
        cli_commands: vec![
            "llm-edge-agent failover test --primary <provider> --alternates <providers>".to_string(),
            "llm-edge-agent failover simulate --scenario <scenario>".to_string(),
            "llm-edge-agent failover inspect --provider <provider>".to_string(),
        ],
    };

    (StatusCode::OK, Json(info))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TestParams {
    primary: String,
    alternates: Vec<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    primary_circuit: Option<CircuitState>,
    #[serde(default)]
    primary_health: Option<ProviderHealth>,
}

#[derive(Debug, Serialize)]
struct TestResult {
    passed: bool,
    outcome: FailoverOutcome,
    selected_provider: Option<String>,
    confidence: f64,
    constraints_evaluated: usize,
    duration_us: u64,
}

#[derive(Debug, Deserialize)]
struct SimulateParams {
    scenario: String,
}

#[derive(Debug, Serialize)]
struct SimulateResult {
    scenario: String,
    outcome: FailoverOutcome,
    selected_target: Option<TargetProvider>,
    reason: String,
    evaluated_targets: Vec<super::EvaluatedTarget>,
    constraints_applied: Vec<crate::Constraint>,
    note: String,
}

#[derive(Debug, Deserialize)]
struct InspectParams {
    #[serde(default)]
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct InspectResult {
    agent_id: String,
    agent_version: String,
    agent_type: String,
    default_policy: FailoverPolicy,
    ruvector_url: String,
    emit_events: bool,
    provider_filter: Option<String>,
    non_responsibilities: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HealthStatus {
    status: String,
    agent_id: String,
    agent_version: String,
    ruvector_healthy: bool,
}

#[derive(Debug, Serialize)]
struct AgentInfo {
    agent_id: String,
    agent_version: String,
    agent_type: String,
    classification: String,
    decision_type: String,
    cli_commands: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        create_failover_router(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn test_info_endpoint() {
        let router = create_test_router();

        let request = Request::builder()
            .uri("/failover/info")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_inspect_endpoint() {
        let router = create_test_router();

        let request = Request::builder()
            .uri("/failover/inspect")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
