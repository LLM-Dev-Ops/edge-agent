//! Circuit Breaker HTTP Handler
//!
//! Implements the HTTP endpoint for the Circuit Breaker Agent.
//! Deployable as a Google Cloud Edge Function.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use super::agent::{CircuitBreakerAgent, RuVectorClient};
use super::types::*;
use crate::error::AgentError;

/// Handler state
pub struct CircuitBreakerHandler {
    agent: Arc<CircuitBreakerAgent>,
}

impl CircuitBreakerHandler {
    /// Create a new handler
    pub fn new(agent: CircuitBreakerAgent) -> Self {
        Self {
            agent: Arc::new(agent),
        }
    }

    /// Create from environment
    pub fn from_env() -> Result<Self, AgentError> {
        let agent = CircuitBreakerAgent::from_env()?;
        Ok(Self::new(agent))
    }

    /// Build the Axum router
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            // Main invocation endpoint
            .route("/invoke", post(invoke_handler))
            // CLI test endpoint
            .route("/test", post(test_handler))
            // CLI simulate endpoint
            .route("/simulate", post(simulate_handler))
            // CLI inspect endpoint
            .route("/inspect/{provider_id}", get(inspect_handler))
            // Health check
            .route("/health", get(health_handler))
            // Metrics
            .route("/metrics", get(metrics_handler))
            .with_state(state)
    }
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Invoke handler - main entry point
async fn invoke_handler(
    State(handler): State<Arc<CircuitBreakerHandler>>,
    Json(request): Json<CircuitBreakerRequest>,
) -> (StatusCode, Json<ApiResponse<CircuitBreakerResponse>>) {
    info!(
        provider_id = %request.provider_id,
        execution_ref = %request.execution_ref,
        "Invoke request received"
    );

    match handler.agent.invoke(request).await {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::success(response))),
        Err(e) => {
            error!("Invoke failed: {}", e);
            let status = match e {
                AgentError::ValidationError(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Test request for CLI testing
#[derive(Debug, Deserialize)]
pub struct TestRequest {
    pub provider_id: String,
    #[serde(default)]
    pub simulate_failures: u32,
    #[serde(default)]
    pub simulate_successes: u32,
}

/// Test response
#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub provider_id: String,
    pub initial_state: CircuitState,
    pub final_state: CircuitState,
    pub decisions: Vec<CircuitBreakerDecision>,
    pub state_transitions: Vec<StateTransition>,
}

/// Test handler - for CLI testing
async fn test_handler(
    State(handler): State<Arc<CircuitBreakerHandler>>,
    Json(test_req): Json<TestRequest>,
) -> (StatusCode, Json<ApiResponse<TestResponse>>) {
    info!(
        provider_id = %test_req.provider_id,
        simulate_failures = test_req.simulate_failures,
        simulate_successes = test_req.simulate_successes,
        "Test request received"
    );

    let mut decisions = Vec::new();
    let mut transitions = Vec::new();
    let initial_state = CircuitState::Closed;
    let mut current_state = initial_state;
    let mut failure_count = 0u32;
    let mut success_count = 0u32;

    // Simulate failures
    for i in 0..test_req.simulate_failures {
        let signal = FailureSignal::new(&test_req.provider_id, "simulated_failure");
        let request = CircuitBreakerRequest::record_failure(
            &test_req.provider_id,
            format!("test-exec-{}", i),
            signal,
        )
        .with_state(current_state, failure_count, success_count);

        match handler.agent.invoke(request).await {
            Ok(response) => {
                decisions.push(response.decision);
                if let Some(t) = response.state_transition {
                    transitions.push(t);
                }
                current_state = response.new_state;
                failure_count = response.new_failure_count;
                success_count = response.new_success_count;
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(e.to_string())),
                );
            }
        }
    }

    // Simulate successes
    for i in 0..test_req.simulate_successes {
        let signal = SuccessSignal::new(&test_req.provider_id);
        let request = CircuitBreakerRequest::record_success(
            &test_req.provider_id,
            format!("test-exec-s-{}", i),
            signal,
        )
        .with_state(current_state, failure_count, success_count);

        match handler.agent.invoke(request).await {
            Ok(response) => {
                decisions.push(response.decision);
                if let Some(t) = response.state_transition {
                    transitions.push(t);
                }
                current_state = response.new_state;
                failure_count = response.new_failure_count;
                success_count = response.new_success_count;
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(e.to_string())),
                );
            }
        }
    }

    let response = TestResponse {
        provider_id: test_req.provider_id,
        initial_state,
        final_state: current_state,
        decisions,
        state_transitions: transitions,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// Simulate request
#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    pub provider_id: String,
    pub initial_state: CircuitState,
    pub initial_failure_count: u32,
    pub initial_success_count: u32,
    #[serde(default)]
    pub failure_signal: Option<FailureSignal>,
    #[serde(default)]
    pub success_signal: Option<SuccessSignal>,
    #[serde(default)]
    pub config: Option<CircuitBreakerConfig>,
}

/// Simulate handler - dry-run without persistence
async fn simulate_handler(
    State(handler): State<Arc<CircuitBreakerHandler>>,
    Json(sim_req): Json<SimulateRequest>,
) -> (StatusCode, Json<ApiResponse<CircuitBreakerResponse>>) {
    info!(
        provider_id = %sim_req.provider_id,
        initial_state = %sim_req.initial_state,
        "Simulate request received"
    );

    // Create a temporary agent without persistence for simulation
    let sim_agent = CircuitBreakerAgent::new(None);

    let mut request = CircuitBreakerRequest::check(&sim_req.provider_id, "simulate");
    request.current_state = sim_req.initial_state;
    request.failure_count = sim_req.initial_failure_count;
    request.success_count = sim_req.initial_success_count;
    request.failure_signal = sim_req.failure_signal;
    request.success_signal = sim_req.success_signal;

    if let Some(config) = sim_req.config {
        request.config = config;
    }

    match sim_agent.invoke(request).await {
        Ok(response) => (StatusCode::OK, Json(ApiResponse::success(response))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ),
    }
}

/// Inspect response
#[derive(Debug, Serialize)]
pub struct InspectResponse {
    pub agent_id: String,
    pub agent_version: String,
    pub provider_id: String,
    pub current_state: Option<CircuitBreakerPersistence>,
    pub config: CircuitBreakerConfig,
    pub health: bool,
}

/// Inspect handler - view current state
async fn inspect_handler(
    State(handler): State<Arc<CircuitBreakerHandler>>,
    axum::extract::Path(provider_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<InspectResponse>>) {
    info!(provider_id = %provider_id, "Inspect request received");

    let metadata = handler.agent.metadata();
    let health = handler.agent.health_check().await;

    // Note: In a real implementation, we would load state from ruvector-service
    // For now, we return None for current_state as this is a stateless agent
    let response = InspectResponse {
        agent_id: metadata.agent_id.clone(),
        agent_version: metadata.agent_version.clone(),
        provider_id: provider_id.clone(),
        current_state: None, // Would be loaded from ruvector-service
        config: CircuitBreakerConfig::from_env(&provider_id),
        health,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// Health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub agent_id: String,
    pub version: String,
    pub ruvector_connected: bool,
}

/// Health handler
async fn health_handler(
    State(handler): State<Arc<CircuitBreakerHandler>>,
) -> (StatusCode, Json<HealthResponse>) {
    let metadata = handler.agent.metadata();
    let ruvector_connected = handler.agent.health_check().await;

    let response = HealthResponse {
        healthy: true, // Agent itself is always healthy if it can respond
        agent_id: metadata.agent_id.clone(),
        version: metadata.agent_version.clone(),
        ruvector_connected,
    };

    (StatusCode::OK, Json(response))
}

/// Metrics handler (Prometheus format)
async fn metrics_handler() -> String {
    // Basic metrics - in production, would integrate with metrics crate
    format!(
        r#"# HELP circuit_breaker_agent_info Agent information
# TYPE circuit_breaker_agent_info gauge
circuit_breaker_agent_info{{agent_id="circuit_breaker_agent",version="{}"}} 1
"#,
        super::agent::AGENT_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let handler = CircuitBreakerHandler::new(CircuitBreakerAgent::new(None));
        let app = handler.router();

        let request = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invoke_endpoint() {
        let handler = CircuitBreakerHandler::new(CircuitBreakerAgent::new(None));
        let app = handler.router();

        let request_body = CircuitBreakerRequest::check("openai", "test-exec");
        let body = serde_json::to_string(&request_body).unwrap();

        let request = Request::builder()
            .uri("/invoke")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
