//! HTTP handler for Tool Invocation Agent
//!
//! This module provides the HTTP handler for deployment as a Google Cloud
//! Edge Function. The handler is stateless and deterministic.
//!
//! # Endpoints
//!
//! - POST /invoke - Process a tool invocation request
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
    agent_metadata, validate_invocation, ToolInvocationAgent, ToolInvocationConfig,
    ToolInvocationRequest, ToolInvocationResponse, RuvectorClient, RuvectorConfig,
};

/// Handler state
#[derive(Clone)]
pub struct ToolInvocationHandler {
    agent: Arc<ToolInvocationAgent>,
}

impl ToolInvocationHandler {
    /// Create a new handler with default configuration
    pub fn new() -> Self {
        let config = ToolInvocationConfig::default();
        let agent = ToolInvocationAgent::new(config);
        Self {
            agent: Arc::new(agent),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ToolInvocationConfig) -> Self {
        let agent = ToolInvocationAgent::new(config);
        Self {
            agent: Arc::new(agent),
        }
    }

    /// Create with ruvector-service integration
    pub fn with_ruvector(config: ToolInvocationConfig, ruvector_config: RuvectorConfig) -> AgentResult<Self> {
        let client = RuvectorClient::new(ruvector_config)?;
        let agent = ToolInvocationAgent::new(config).with_ruvector_client(client);
        Ok(Self {
            agent: Arc::new(agent),
        })
    }

    /// Build the router for this handler
    pub fn router(self) -> Router {
        Router::new()
            .route("/invoke", post(invoke_handler))
            .route("/test", post(test_handler))
            .route("/simulate", post(simulate_handler))
            .route("/inspect", get(inspect_handler))
            .route("/health", get(health_handler))
            .with_state(self)
    }
}

impl Default for ToolInvocationHandler {
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

/// Invoke endpoint - process a tool invocation request
async fn invoke_handler(
    State(handler): State<ToolInvocationHandler>,
    Json(request): Json<ToolInvocationRequest>,
) -> impl IntoResponse {
    match handler.agent.process(request).await {
        Ok(response) => (
            StatusCode::OK,
            Json(serde_json::to_value(ApiResponse::success(response)).unwrap()),
        ),
        Err(e) => {
            let status = match &e {
                AgentError::Validation(_) | AgentError::SchemaValidation { .. } => {
                    StatusCode::BAD_REQUEST
                }
                AgentError::ToolNotAllowed(_) | AgentError::ToolBlocked { .. } => {
                    StatusCode::FORBIDDEN
                }
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
    pub request: ToolInvocationRequest,
}

#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub validation: super::types::ValidationResult,
    pub would_allow: bool,
}

async fn test_handler(
    State(handler): State<ToolInvocationHandler>,
    Json(test_req): Json<TestRequest>,
) -> impl IntoResponse {
    let validation = match validate_invocation(&test_req.request, handler.agent.config()) {
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
    pub tool_name: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub blocked: bool,
}

async fn simulate_handler(
    State(handler): State<ToolInvocationHandler>,
    Json(sim_req): Json<SimulateRequest>,
) -> impl IntoResponse {
    use super::types::ToolDefinition;
    use crate::contracts::ExecutionContext;

    // Build a mock request
    let request = ToolInvocationRequest {
        tool: ToolDefinition {
            name: sim_req.tool_name.clone(),
            description: Some(format!("Simulated tool: {}", sim_req.tool_name)),
            parameters: vec![],
            capabilities: vec![],
        },
        arguments: sim_req.params,
        context: Some(ExecutionContext::new()),
        constraints: None,
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
    pub config: serde_json::Value,
    pub endpoints: Vec<EndpointInfo>,
    pub non_responsibilities: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub method: String,
    pub path: String,
    pub description: String,
}

async fn inspect_handler(
    State(handler): State<ToolInvocationHandler>,
) -> impl IntoResponse {
    let metadata = agent_metadata();

    let response = InspectResponse {
        agent_id: metadata.agent_id,
        agent_version: metadata.agent_version,
        agent_type: format!("{:?}", metadata.agent_type),
        classification: "EXECUTION CONTROL".to_string(),
        config: serde_json::to_value(handler.agent.config()).unwrap_or_default(),
        endpoints: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/invoke".to_string(),
                description: "Process a tool invocation request".to_string(),
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/test".to_string(),
                description: "Test validation without full processing".to_string(),
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/simulate".to_string(),
                description: "Simulate with mock data".to_string(),
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/inspect".to_string(),
                description: "Get agent configuration and metadata".to_string(),
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/health".to_string(),
                description: "Health check endpoint".to_string(),
            },
        ],
        non_responsibilities: vec![
            "Perform orchestration (that is LLM-Orchestrator)".to_string(),
            "Modify policies dynamically".to_string(),
            "Trigger retries directly (retry logic lives in Orchestrator)".to_string(),
            "Emit alerts (that is Sentinel)".to_string(),
            "Perform analytics (that is Observatory/Latency-Lens)".to_string(),
            "Persist state locally (use ruvector-service only)".to_string(),
            "Execute SQL directly".to_string(),
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
}

async fn health_handler(
    State(_handler): State<ToolInvocationHandler>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::success(HealthResponse {
            status: "healthy".to_string(),
            agent_id: super::AGENT_ID.to_string(),
            version: crate::AGENT_VERSION.to_string(),
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
/// use llm_edge_agents::tool_invocation::tool_invocation_handler;
///
/// #[tokio::main]
/// async fn main() {
///     let app = tool_invocation_handler();
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
pub fn tool_invocation_handler() -> Router {
    ToolInvocationHandler::new().router()
}

/// Create handler with custom configuration
pub fn tool_invocation_handler_with_config(
    config: ToolInvocationConfig,
) -> Router {
    ToolInvocationHandler::with_config(config).router()
}

/// Create handler with ruvector-service integration
pub fn tool_invocation_handler_with_ruvector(
    config: ToolInvocationConfig,
    ruvector_config: RuvectorConfig,
) -> AgentResult<Router> {
    Ok(ToolInvocationHandler::with_ruvector(config, ruvector_config)?.router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = tool_invocation_handler();

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
        let app = tool_invocation_handler();

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
