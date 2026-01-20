//! HTTP Handler for Caching Strategy Agent
//!
//! This module provides HTTP endpoints for the Caching Strategy Agent,
//! designed for deployment as a Google Cloud Edge Function.
//!
//! # Endpoints
//!
//! - `POST /cache-strategy` - Process a cache strategy request
//! - `POST /cache-strategy/test` - Test mode (validate without persistence)
//! - `POST /cache-strategy/simulate` - Simulate with mock data
//! - `GET /cache-strategy/inspect` - Inspect agent configuration

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{
    CacheDecision, CacheDirective, CacheLayer, CachePolicyConfig, CacheRelevantParams,
    CacheStrategyConfig, CacheStrategyInput, CacheStrategyOutput, CacheStrategyRequest,
    CacheStrategyResponse,
};
use super::CachingStrategyAgent;
use crate::contracts::{AgentType, ExecutionContext};
use crate::error::AgentError;

/// Handler state
#[derive(Clone)]
pub struct CacheStrategyHandlerState {
    pub agent: Arc<CachingStrategyAgent>,
}

/// Create HTTP router for the Caching Strategy Agent
pub fn cache_strategy_router(state: CacheStrategyHandlerState) -> Router {
    Router::new()
        .route("/cache-strategy", post(handle_cache_strategy))
        .route("/cache-strategy/test", post(handle_test))
        .route("/cache-strategy/simulate", post(handle_simulate))
        .route("/cache-strategy/inspect", get(handle_inspect))
        .with_state(state)
}

/// Main cache strategy handler
async fn handle_cache_strategy(
    State(state): State<CacheStrategyHandlerState>,
    Json(request): Json<CacheStrategyRequest>,
) -> impl IntoResponse {
    match state.agent.process(request).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => error_response(e),
    }
}

/// Test mode handler - validate without persistence
#[derive(Debug, Deserialize)]
pub struct TestRequest {
    pub input: CacheStrategyInput,
    #[serde(default)]
    pub validate_only: bool,
}

#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub valid: bool,
    pub validation_result: super::types::CacheValidationResult,
    pub would_be_decision: CacheDecision,
    pub would_be_cacheable: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

async fn handle_test(
    State(state): State<CacheStrategyHandlerState>,
    Json(test_req): Json<TestRequest>,
) -> impl IntoResponse {
    use super::validator::validate_cache_strategy;

    let validation = match validate_cache_strategy(&test_req.input, state.agent.config()) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TestResponse {
                    valid: false,
                    validation_result: super::types::CacheValidationResult::default(),
                    would_be_decision: CacheDecision::CacheBypass,
                    would_be_cacheable: false,
                    errors: vec![e.to_string()],
                    warnings: vec![],
                }),
            )
                .into_response();
        }
    };

    let errors: Vec<String> = validation.errors.iter().map(|e| e.message.clone()).collect();
    let warnings: Vec<String> = validation.warnings.iter().map(|w| w.message.clone()).collect();

    // Determine what decision would be made
    let (would_be_decision, would_be_cacheable) = if !validation.valid {
        (CacheDecision::CacheBypass, false)
    } else if test_req.input.existing_entry.is_some()
        && test_req.input.existing_entry.as_ref().map(|e| e.is_valid).unwrap_or(false)
    {
        (CacheDecision::CacheHit, true)
    } else if test_req.input.is_write_operation {
        (CacheDecision::CacheWrite, true)
    } else {
        (CacheDecision::CacheMiss, true)
    };

    (
        StatusCode::OK,
        Json(TestResponse {
            valid: validation.valid,
            validation_result: validation,
            would_be_decision,
            would_be_cacheable,
            errors,
            warnings,
        }),
    )
        .into_response()
}

/// Simulate request for testing
#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    /// Request type to simulate
    pub request_type: String,

    /// Provider ID
    #[serde(default)]
    pub provider_id: Option<String>,

    /// Model ID
    #[serde(default)]
    pub model_id: Option<String>,

    /// Temperature setting
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Whether to simulate a cache hit
    #[serde(default)]
    pub simulate_cache_hit: bool,

    /// Whether this is a write operation
    #[serde(default)]
    pub is_write_operation: bool,

    /// Response size for write simulation
    #[serde(default)]
    pub response_size_bytes: Option<usize>,
}

async fn handle_simulate(
    State(state): State<CacheStrategyHandlerState>,
    Json(sim_req): Json<SimulateRequest>,
) -> impl IntoResponse {
    // Build simulated input
    let execution_ref = uuid::Uuid::new_v4().to_string();
    let content_hash = format!("sim_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let existing_entry = if sim_req.simulate_cache_hit {
        Some(super::types::CacheEntryMetadata {
            cache_key: format!("cache:sim:{}", content_hash),
            layer: CacheLayer::L1,
            created_at: chrono::Utc::now() - chrono::Duration::minutes(30),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
            last_accessed_at: Some(chrono::Utc::now()),
            access_count: 5,
            response_size_bytes: 1000,
            is_valid: true,
            is_stale: false,
            content_hash: content_hash.clone(),
            semantic_hash: None,
        })
    } else {
        None
    };

    let response_metadata = if sim_req.is_write_operation {
        Some(super::types::ResponseMetadata {
            response_size_bytes: sim_req.response_size_bytes.unwrap_or(1000),
            latency_ms: 250,
            token_count: Some(150),
            success: true,
            contains_sensitive_data: false,
            finish_reason: Some("stop".to_string()),
        })
    } else {
        None
    };

    let input = CacheStrategyInput {
        execution_ref,
        request_type: sim_req.request_type,
        provider_id: sim_req.provider_id,
        model_id: sim_req.model_id,
        content_hash,
        prompt_hash: format!("prompt_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        semantic_hash: None,
        cache_relevant_params: CacheRelevantParams {
            temperature: sim_req.temperature,
            top_p: None,
            max_tokens: None,
            seed: Some(42),
            response_format: None,
            stream: false,
            system_prompt_hash: None,
        },
        policy: CachePolicyConfig::default(),
        existing_entry,
        check_cache: true,
        is_write_operation: sim_req.is_write_operation,
        response_metadata,
        tenant_id: None,
        user_id_hash: None,
    };

    let request = CacheStrategyRequest {
        input,
        context: Some(ExecutionContext::new()),
        config_override: None,
    };

    match state.agent.process(request).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => error_response(e),
    }
}

/// Agent inspection response
#[derive(Debug, Serialize)]
pub struct InspectResponse {
    pub agent_id: String,
    pub agent_version: String,
    pub agent_type: AgentType,
    pub classification: String,
    pub decision_type: String,
    pub config: CacheStrategyConfig,
    pub capabilities: Vec<String>,
    pub non_responsibilities: Vec<String>,
    pub upstream_invokers: Vec<String>,
    pub cli_commands: CliCommands,
}

#[derive(Debug, Serialize)]
pub struct CliCommands {
    pub test: String,
    pub simulate: String,
    pub inspect: String,
}

async fn handle_inspect(State(state): State<CacheStrategyHandlerState>) -> impl IntoResponse {
    let metadata = state.agent.metadata();

    (
        StatusCode::OK,
        Json(InspectResponse {
            agent_id: metadata.agent_id,
            agent_version: metadata.agent_version,
            agent_type: metadata.agent_type,
            classification: "EXECUTION CONTROL".to_string(),
            decision_type: "cache_strategy_decision".to_string(),
            config: state.agent.config().clone(),
            capabilities: vec![
                "Evaluate cache eligibility".to_string(),
                "Apply cache read/write/bypass decisions".to_string(),
                "Emit cache directives".to_string(),
                "Semantic caching support".to_string(),
                "Multi-layer cache recommendations".to_string(),
                "Stale-while-revalidate support".to_string(),
            ],
            non_responsibilities: vec![
                "Perform orchestration (that is LLM-Orchestrator)".to_string(),
                "Modify policies dynamically".to_string(),
                "Trigger retries directly (retry logic lives in Orchestrator)".to_string(),
                "Emit alerts (that is Sentinel)".to_string(),
                "Perform analytics (that is Observatory/Latency-Lens)".to_string(),
                "Persist state locally (use ruvector-service only)".to_string(),
                "Execute SQL directly".to_string(),
                "Store actual response payloads (only metadata)".to_string(),
            ],
            upstream_invokers: vec![
                "LLM-Orchestrator: During request processing for cache check".to_string(),
                "LLM-Orchestrator: After response for cache write decision".to_string(),
                "Direct API: For testing and validation".to_string(),
            ],
            cli_commands: CliCommands {
                test: "llm-edge-agent agent cache-strategy test --input <json>".to_string(),
                simulate: "llm-edge-agent agent cache-strategy simulate --request-type chat_completion --temperature 0.0".to_string(),
                inspect: "llm-edge-agent agent cache-strategy inspect".to_string(),
            },
        }),
    )
        .into_response()
}

/// Convert AgentError to HTTP response
fn error_response(error: AgentError) -> axum::response::Response {
    let (status, code, message) = match &error {
        AgentError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
        AgentError::SchemaValidation { field, message } => (
            StatusCode::BAD_REQUEST,
            "SCHEMA_VALIDATION_ERROR",
            format!("{}: {}", field, message),
        ),
        AgentError::Configuration(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIGURATION_ERROR",
            msg.clone(),
        ),
        AgentError::Internal(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            msg.clone(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error.error_code(),
            error.to_string(),
        ),
    };

    (
        status,
        Json(serde_json::json!({
            "error": {
                "code": code,
                "message": message,
                "severity": error.severity(),
            }
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn create_test_state() -> CacheStrategyHandlerState {
        CacheStrategyHandlerState {
            agent: Arc::new(CachingStrategyAgent::new(CacheStrategyConfig::default())),
        }
    }

    #[tokio::test]
    async fn test_inspect_endpoint() {
        let state = create_test_state();
        let router = cache_strategy_router(state);

        let request = Request::builder()
            .method("GET")
            .uri("/cache-strategy/inspect")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_test_endpoint() {
        let state = create_test_state();
        let router = cache_strategy_router(state);

        let test_input = TestRequest {
            input: CacheStrategyInput {
                execution_ref: "test-123".to_string(),
                request_type: "chat_completion".to_string(),
                provider_id: Some("openai".to_string()),
                model_id: Some("gpt-4".to_string()),
                content_hash: "hash123".to_string(),
                prompt_hash: "prompt456".to_string(),
                semantic_hash: None,
                cache_relevant_params: CacheRelevantParams::default(),
                policy: CachePolicyConfig::default(),
                existing_entry: None,
                check_cache: true,
                is_write_operation: false,
                response_metadata: None,
                tenant_id: None,
                user_id_hash: None,
            },
            validate_only: true,
        };

        let request = Request::builder()
            .method("POST")
            .uri("/cache-strategy/test")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&test_input).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_simulate_endpoint() {
        let state = create_test_state();
        let router = cache_strategy_router(state);

        let sim_request = SimulateRequest {
            request_type: "chat_completion".to_string(),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-4".to_string()),
            temperature: Some(0.0),
            simulate_cache_hit: false,
            is_write_operation: false,
            response_size_bytes: None,
        };

        let request = Request::builder()
            .method("POST")
            .uri("/cache-strategy/simulate")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&sim_request).unwrap()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
