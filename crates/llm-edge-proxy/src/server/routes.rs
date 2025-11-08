//! Route handlers for the HTTP server

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, instrument};

use crate::Config;
use crate::error::ProxyResult;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
}

/// Health check endpoint
#[instrument(name = "health_check")]
pub async fn health_check() -> Json<HealthResponse> {
    info!("Health check requested");
    
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness check - checks if service is ready to accept traffic
#[instrument(name = "readiness_check")]
pub async fn readiness_check() -> Json<HealthResponse> {
    // In a full implementation, this would check:
    // - Database connectivity
    // - Cache availability
    // - Downstream service health
    
    Json(HealthResponse {
        status: "ready".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Liveness check - simple check if service is running
#[instrument(name = "liveness_check")]
pub async fn liveness_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Prometheus metrics endpoint
#[instrument(name = "metrics")]
pub async fn metrics() -> Response {
    // In a full implementation, this would use the prometheus crate
    // to collect and export metrics
    let metrics_text = r#"# HELP llm_requests_total Total number of LLM requests
# TYPE llm_requests_total counter
llm_requests_total{provider="openai",status="success"} 0

# HELP llm_request_duration_seconds Request duration in seconds
# TYPE llm_request_duration_seconds histogram
llm_request_duration_seconds_bucket{le="0.005"} 0
llm_request_duration_seconds_bucket{le="0.01"} 0
llm_request_duration_seconds_bucket{le="0.025"} 0
llm_request_duration_seconds_bucket{le="0.05"} 0
llm_request_duration_seconds_bucket{le="0.1"} 0
llm_request_duration_seconds_bucket{le="+Inf"} 0
llm_request_duration_seconds_sum 0
llm_request_duration_seconds_count 0

# HELP llm_cache_hit_total Total cache hits
# TYPE llm_cache_hit_total counter
llm_cache_hit_total 0
"#;

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        metrics_text,
    )
        .into_response()
}

/// Chat completions request (OpenAI-compatible)
#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completions response
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible chat completions endpoint
#[instrument(name = "chat_completions", skip(_config, request))]
pub async fn chat_completions(
    State(_config): State<Config>,
    Json(request): Json<ChatCompletionRequest>,
) -> ProxyResult<Json<ChatCompletionResponse>> {
    info!(
        model = %request.model,
        message_count = request.messages.len(),
        stream = request.stream,
        "Processing chat completion request"
    );

    // For now, return a mock response
    // In Layer 2, this will be routed to actual providers
    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: request.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "This is a mock response from LLM Edge Agent Layer 1. Provider integration will be added in Layer 2.".to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
    };

    Ok(Json(response))
}

/// Legacy completions endpoint
#[instrument(name = "completions")]
pub async fn completions(
    State(_config): State<Config>,
    Json(request): Json<serde_json::Value>,
) -> ProxyResult<Json<serde_json::Value>> {
    info!("Processing legacy completion request");

    // Mock response
    Ok(Json(json!({
        "id": format!("cmpl-{}", uuid::Uuid::new_v4()),
        "object": "text_completion",
        "created": chrono::Utc::now().timestamp(),
        "model": request.get("model").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "choices": [
            {
                "text": "Mock completion response",
                "index": 0,
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 5,
            "completion_tokens": 10,
            "total_tokens": 15
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0.status, "healthy");
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let response = readiness_check().await;
        assert_eq!(response.0.status, "ready");
    }
}
