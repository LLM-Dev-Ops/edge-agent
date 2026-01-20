//! HTTP handler for Execution Guard Agent
//!
//! This module provides the HTTP handler for deployment as a Google Cloud
//! Edge Function. The handler is stateless and deterministic.
//!
//! # Endpoints
//!
//! - POST /guard - Process an execution guard request
//! - POST /test - Test validation without full processing
//! - POST /simulate - Simulate with mock data
//! - GET /inspect - Get agent configuration and metadata
//! - GET /health - Health check endpoint
//!
//! # Google Cloud Functions Compatibility
//!
//! The handler is designed to work with:
//! - Cloud Run (HTTP trigger)
//! - Cloud Functions (HTTP trigger)
//! - API Gateway integration
//!
//! # Non-Responsibilities (MUST NEVER DO)
//!
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Persist state locally (use ruvector-service only)

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{AgentError, AgentResult};

use super::{
    agent_metadata, validate_execution, ExecutionGuardAgent, ExecutionGuardConfig,
    ExecutionGuardRequest, ExecutionGuardResponse, RuvectorClient, RuvectorConfig,
};

/// Handler state
#[derive(Clone)]
pub struct ExecutionGuardHandler {
    agent: Arc<ExecutionGuardAgent>,
}

impl ExecutionGuardHandler {
    /// Create a new handler with default configuration
    pub fn new() -> Self {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);
        Self {
            agent: Arc::new(agent),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ExecutionGuardConfig) -> Self {
        let agent = ExecutionGuardAgent::new(config);
        Self {
            agent: Arc::new(agent),
        }
    }

    /// Create with ruvector-service integration
    pub fn with_ruvector(
        config: ExecutionGuardConfig,
        ruvector_config: RuvectorConfig,
    ) -> AgentResult<Self> {
        let client = RuvectorClient::new(ruvector_config)?;
        let agent = ExecutionGuardAgent::new(config).with_ruvector_client(client);
        Ok(Self {
            agent: Arc::new(agent),
        })
    }

    /// Build the router for this handler
    pub fn router(self) -> Router {
        Router::new()
            .route("/guard", post(guard_handler))
            .route("/test", post(test_handler))
            .route("/simulate", post(simulate_handler))
            .route("/inspect", get(inspect_handler))
            .route("/health", get(health_handler))
            .with_state(self)
    }
}

impl Default for ExecutionGuardHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error_response(code: impl Into<String>, message: impl Into<String>) -> serde_json::Value {
        serde_json::json!({
            "success": false,
            "data": null,
            "error": {
                "code": code.into(),
                "message": message.into()
            }
        })
    }
}

/// Guard endpoint - process an execution guard request
async fn guard_handler(
    State(handler): State<ExecutionGuardHandler>,
    Json(request): Json<ExecutionGuardRequest>,
) -> impl IntoResponse {
    match handler.agent.process(request).await {
        Ok(response) => (StatusCode::OK, Json(serde_json::to_value(ApiResponse::success(response)).unwrap())),
        Err(e) => {
            let status = match &e {
                AgentError::Validation(_) | AgentError::SchemaValidation { .. } => {
                    StatusCode::BAD_REQUEST
                }
                AgentError::ConstraintViolation { .. } => StatusCode::FORBIDDEN,
                AgentError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            (
                status,
                Json(ApiResponse::<()>::error_response(
                    e.error_code(),
                    e.to_string(),
                )),
            )
        }
    }
}

/// Test endpoint - validate without full processing
#[derive(Debug, Deserialize)]
pub struct TestRequest {
    pub request: ExecutionGuardRequest,
}

#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub validation: super::types::ValidationResult,
    pub would_allow: bool,
}

async fn test_handler(
    State(handler): State<ExecutionGuardHandler>,
    Json(test_req): Json<TestRequest>,
) -> impl IntoResponse {
    let validation = match validate_execution(&test_req.request, handler.agent.config()) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error_response(e.error_code(), e.to_string())),
            );
        }
    };

    let would_allow = validation.valid;

    (
        StatusCode::OK,
        Json(serde_json::to_value(ApiResponse::success(TestResponse {
            validation,
            would_allow,
        })).unwrap()),
    )
}

/// Simulate endpoint - simulate with mock data
#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    pub model: String,
    #[serde(default = "default_input_tokens")]
    pub input_tokens: u64,
    #[serde(default = "default_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default)]
    pub current_concurrency: u32,
    #[serde(default)]
    pub current_rate: Option<u32>,
}

fn default_input_tokens() -> u64 {
    1000
}

fn default_output_tokens() -> u64 {
    2000
}

async fn simulate_handler(
    State(handler): State<ExecutionGuardHandler>,
    Json(sim_req): Json<SimulateRequest>,
) -> impl IntoResponse {
    use crate::contracts::ExecutionContext;
    use super::types::ExecutionEnvelope;

    // Build a mock request
    let request = ExecutionGuardRequest {
        envelope: ExecutionEnvelope {
            request_id: format!("sim-{}", uuid::Uuid::new_v4()),
            model: sim_req.model.clone(),
            provider: None,
            input_tokens: sim_req.input_tokens,
            max_output_tokens: sim_req.max_output_tokens,
            estimated_duration_ms: None,
            estimated_memory_bytes: None,
            estimated_cost_micros: None,
            current_concurrency: sim_req.current_concurrency,
            metadata: std::collections::HashMap::new(),
        },
        context: Some(ExecutionContext::new()),
        limits: None,
        current_rate: sim_req.current_rate,
    };

    // Process the simulated request
    match handler.agent.process(request).await {
        Ok(response) => (StatusCode::OK, Json(serde_json::to_value(ApiResponse::success(response)).unwrap())),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error_response(
                e.error_code(),
                e.to_string(),
            )),
        ),
    }
}

/// Inspect endpoint - get agent configuration and metadata
#[derive(Debug, Serialize)]
pub struct InspectResponse {
    pub agent_id: String,
    pub agent_version: String,
    pub agent_type: String,
    pub classification: String,
    pub purpose: String,
    pub decision_type: String,
    pub config: serde_json::Value,
    pub endpoints: Vec<EndpointInfo>,
    pub constraints_enforced: Vec<String>,
    pub non_responsibilities: Vec<String>,
    pub upstream_invokers: Vec<String>,
    pub failure_modes: Vec<FailureMode>,
}

#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct FailureMode {
    pub name: String,
    pub description: String,
    pub http_status: u16,
}

async fn inspect_handler(State(handler): State<ExecutionGuardHandler>) -> impl IntoResponse {
    let metadata = agent_metadata();

    let response = InspectResponse {
        agent_id: metadata.agent_id,
        agent_version: metadata.agent_version,
        agent_type: format!("{:?}", metadata.agent_type),
        classification: "PROTECTION / GUARD".to_string(),
        purpose: "Enforce execution safety constraints at runtime including token, time, memory, cost, rate, and concurrency limits".to_string(),
        decision_type: "execution_guard_decision".to_string(),
        config: serde_json::to_value(handler.agent.config()).unwrap_or_default(),
        endpoints: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/guard".to_string(),
                description: "Process an execution guard request (allow/block decision)".to_string(),
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/test".to_string(),
                description: "Test validation without full processing or event emission".to_string(),
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/simulate".to_string(),
                description: "Simulate with mock execution envelope data".to_string(),
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/inspect".to_string(),
                description: "Get agent configuration, metadata, and contract".to_string(),
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/health".to_string(),
                description: "Health check endpoint for load balancers".to_string(),
            },
        ],
        constraints_enforced: vec![
            "Token limits (input, output, total)".to_string(),
            "Time limits (execution duration)".to_string(),
            "Memory limits".to_string(),
            "Cost limits (per-request budget)".to_string(),
            "Rate limits (requests per window)".to_string(),
            "Concurrency limits (max parallel executions)".to_string(),
            "Model restrictions (allowed/blocked models)".to_string(),
        ],
        non_responsibilities: vec![
            "Perform orchestration (that is LLM-Orchestrator)".to_string(),
            "Modify policies dynamically".to_string(),
            "Trigger retries directly (retry logic lives in Orchestrator)".to_string(),
            "Emit alerts (that is Sentinel)".to_string(),
            "Perform analytics (that is Observatory/Latency-Lens)".to_string(),
            "Persist state locally (use ruvector-service only)".to_string(),
            "Execute SQL directly (use ruvector-service client)".to_string(),
        ],
        upstream_invokers: vec![
            "LLM-Orchestrator: During workflow execution before LLM calls".to_string(),
            "LLM-Shield: For pre-execution resource validation".to_string(),
            "CostOps: For budget enforcement checks".to_string(),
            "Direct API: For testing and validation".to_string(),
        ],
        failure_modes: vec![
            FailureMode {
                name: "TOKEN_LIMIT_EXCEEDED".to_string(),
                description: "Input, output, or total token count exceeds configured limits".to_string(),
                http_status: 403,
            },
            FailureMode {
                name: "COST_LIMIT_EXCEEDED".to_string(),
                description: "Estimated cost exceeds per-request budget".to_string(),
                http_status: 403,
            },
            FailureMode {
                name: "RATE_LIMIT_EXCEEDED".to_string(),
                description: "Too many requests in the current time window".to_string(),
                http_status: 429,
            },
            FailureMode {
                name: "CONCURRENCY_LIMIT_EXCEEDED".to_string(),
                description: "Too many concurrent executions for this tenant".to_string(),
                http_status: 429,
            },
            FailureMode {
                name: "MODEL_BLOCKED".to_string(),
                description: "Requested model is blocked by policy".to_string(),
                http_status: 403,
            },
            FailureMode {
                name: "VALIDATION_ERROR".to_string(),
                description: "Request schema validation failed".to_string(),
                http_status: 400,
            },
        ],
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// Health check endpoint
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub agent_id: String,
    pub version: String,
    pub classification: String,
}

async fn health_handler(State(_handler): State<ExecutionGuardHandler>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::success(HealthResponse {
            status: "healthy".to_string(),
            agent_id: super::AGENT_ID.to_string(),
            version: crate::AGENT_VERSION.to_string(),
            classification: "PROTECTION / GUARD".to_string(),
        })),
    )
}

/// Create the handler router for Google Cloud Edge Function
///
/// This is the main entry point for deployment.
///
/// # Example
///
/// ```rust,ignore
/// use llm_edge_agents::execution_guard::execution_guard_handler;
///
/// #[tokio::main]
/// async fn main() {
///     let app = execution_guard_handler();
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
pub fn execution_guard_handler() -> Router {
    ExecutionGuardHandler::new().router()
}

/// Create handler with custom configuration
pub fn execution_guard_handler_with_config(config: ExecutionGuardConfig) -> Router {
    ExecutionGuardHandler::with_config(config).router()
}

/// Create handler with ruvector-service integration
pub fn execution_guard_handler_with_ruvector(
    config: ExecutionGuardConfig,
    ruvector_config: RuvectorConfig,
) -> AgentResult<Router> {
    Ok(ExecutionGuardHandler::with_ruvector(config, ruvector_config)?.router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = execution_guard_handler();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_inspect_endpoint() {
        let app = execution_guard_handler();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/inspect")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
