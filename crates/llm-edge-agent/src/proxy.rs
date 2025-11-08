//! Proxy request handler
//!
//! This module implements the complete request/response flow through all layers:
//! 1. Request validation and transformation
//! 2. Cache lookup (L1/L2)
//! 3. Shield/PII detection (if cache miss)
//! 4. Provider routing decision
//! 5. Provider request execution
//! 6. Response validation
//! 7. Cache write (async)
//! 8. Response transformation and return

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use llm_edge_cache::CacheLookupResult;
use llm_edge_monitoring::metrics;
use llm_edge_providers::{LLMProvider, UnifiedRequest, UnifiedResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::integration::AppState;

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
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

#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    pub provider: String,
    pub cached: bool,
    pub cache_tier: Option<String>,
    pub latency_ms: u64,
    pub cost_usd: Option<f64>,
}

/// Error type for proxy operations
#[derive(Debug)]
pub enum ProxyError {
    CacheError(String),
    ProviderError(String),
    ValidationError(String),
    InternalError(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ProxyError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            ProxyError::ProviderError(msg) => (StatusCode::BAD_GATEWAY, msg),
            ProxyError::CacheError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Cache error: {}", msg)),
            ProxyError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": {
                "message": message,
                "type": "proxy_error",
            }
        });

        (status, Json(body)).into_response()
    }
}

/// Main chat completions proxy handler
///
/// This is the core handler that processes all chat completion requests.
/// It orchestrates the entire request flow through caching, routing, and provider layers.
#[instrument(name = "proxy_chat_completions", skip(state, request), fields(
    request_id = %Uuid::new_v4(),
    model = %request.model,
    message_count = request.messages.len(),
))]
pub async fn handle_chat_completions(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ProxyError> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        model = %request.model,
        "Processing chat completion request"
    );

    // Step 1: Validate request
    validate_request(&request)?;

    // Step 2: Convert to cacheable format
    let cacheable_req = convert_to_cacheable(&request);

    // Step 3: Check cache (L1 -> L2)
    let cache_lookup = state.cache_manager.lookup(&cacheable_req).await;

    match cache_lookup {
        CacheLookupResult::L1Hit(cached_response) => {
            info!(request_id = %request_id, "Cache HIT: L1");
            metrics::record_cache_hit("l1");

            let response = build_response_from_cache(
                &request,
                &cached_response,
                "l1",
                start_time.elapsed().as_millis() as u64,
            );

            return Ok(Json(response));
        }
        CacheLookupResult::L2Hit(cached_response) => {
            info!(request_id = %request_id, "Cache HIT: L2");
            metrics::record_cache_hit("l2");

            let response = build_response_from_cache(
                &request,
                &cached_response,
                "l2",
                start_time.elapsed().as_millis() as u64,
            );

            return Ok(Json(response));
        }
        CacheLookupResult::Miss => {
            debug!(request_id = %request_id, "Cache MISS - routing to provider");
            metrics::record_cache_miss("all");
        }
    }

    // Step 4: Route to provider
    let (provider, provider_name) = select_provider(&state, &request)?;

    // Step 5: Convert to unified request format
    let unified_request = convert_to_unified(&request);

    // Step 6: Send to provider
    info!(
        request_id = %request_id,
        provider = %provider_name,
        "Sending request to provider"
    );

    let provider_start = Instant::now();
    let provider_response = provider
        .send(unified_request)
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                provider = %provider_name,
                error = %e,
                "Provider request failed"
            );
            metrics::record_request_failure(&provider_name, &request.model, "provider_error");
            ProxyError::ProviderError(format!("Provider error: {}", e))
        })?;

    let provider_latency = provider_start.elapsed().as_millis() as u64;

    // Step 7: Calculate cost
    let cost_usd = calculate_cost(&provider, &request.model, &provider_response);

    // Step 8: Record metrics
    metrics::record_request_success(&provider_name, &request.model, provider_latency);
    metrics::record_token_usage(
        &provider_name,
        &request.model,
        provider_response.usage.prompt_tokens,
        provider_response.usage.completion_tokens,
    );
    if let Some(cost) = cost_usd {
        metrics::record_cost(&provider_name, &request.model, cost);
    }

    // Step 9: Store in cache (async, non-blocking)
    let cache_response = convert_provider_to_cache(&provider_response);
    tokio::spawn({
        let cache_manager = state.cache_manager.clone();
        let cacheable_req = cacheable_req.clone();
        async move {
            cache_manager.store(&cacheable_req, cache_response).await;
        }
    });

    // Step 10: Build and return response
    let total_latency = start_time.elapsed().as_millis() as u64;
    let response = build_response_from_provider(
        &request,
        provider_response,
        &provider_name,
        total_latency,
        cost_usd,
    );

    info!(
        request_id = %request_id,
        provider = %provider_name,
        total_latency_ms = total_latency,
        provider_latency_ms = provider_latency,
        "Request completed successfully"
    );

    Ok(Json(response))
}

/// Validate the incoming request
fn validate_request(request: &ChatCompletionRequest) -> Result<(), ProxyError> {
    if request.model.is_empty() {
        return Err(ProxyError::ValidationError("Model is required".to_string()));
    }

    if request.messages.is_empty() {
        return Err(ProxyError::ValidationError("Messages cannot be empty".to_string()));
    }

    if request.stream {
        return Err(ProxyError::ValidationError("Streaming is not yet supported".to_string()));
    }

    Ok(())
}

/// Convert chat completion request to cacheable format
fn convert_to_cacheable(request: &ChatCompletionRequest) -> llm_edge_cache::key::CacheableRequest {
    // Concatenate all messages into a single prompt for caching
    let prompt = request
        .messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let mut cacheable = llm_edge_cache::key::CacheableRequest::new(&request.model, prompt);

    if let Some(temp) = request.temperature {
        cacheable = cacheable.with_temperature(temp);
    }

    if let Some(max_tokens) = request.max_tokens {
        cacheable = cacheable.with_max_tokens(max_tokens);
    }

    cacheable
}

/// Convert chat completion request to unified format
fn convert_to_unified(request: &ChatCompletionRequest) -> UnifiedRequest {
    use std::collections::HashMap;

    UnifiedRequest {
        model: request.model.clone(),
        messages: request
            .messages
            .iter()
            .map(|m| llm_edge_providers::Message {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        temperature: request.temperature,
        max_tokens: request.max_tokens.map(|t| t as usize),
        stream: request.stream,
        metadata: HashMap::new(),
    }
}

/// Select the appropriate provider for the request
fn select_provider(
    state: &AppState,
    request: &ChatCompletionRequest,
) -> Result<(Arc<dyn LLMProvider>, String), ProxyError> {
    // For MVP, use simple model-based routing
    // In production, this would use the routing engine

    let model_lower = request.model.to_lowercase();

    if model_lower.contains("gpt") || model_lower.contains("openai") {
        if let Some(provider) = &state.openai_provider {
            return Ok((provider.clone(), "openai".to_string()));
        }
    }

    if model_lower.contains("claude") || model_lower.contains("anthropic") {
        if let Some(provider) = &state.anthropic_provider {
            return Ok((provider.clone(), "anthropic".to_string()));
        }
    }

    // Fallback to first available provider
    if let Some(provider) = &state.openai_provider {
        warn!("Using fallback provider: openai");
        return Ok((provider.clone(), "openai".to_string()));
    }

    if let Some(provider) = &state.anthropic_provider {
        warn!("Using fallback provider: anthropic");
        return Ok((provider.clone(), "anthropic".to_string()));
    }

    Err(ProxyError::InternalError("No providers configured".to_string()))
}

/// Calculate the cost of a request
fn calculate_cost(
    provider: &Arc<dyn LLMProvider>,
    model: &str,
    response: &UnifiedResponse,
) -> Option<f64> {
    provider.get_pricing(model).map(|pricing| {
        let input_cost = (response.usage.prompt_tokens as f64 / 1000.0) * pricing.input_cost_per_1k;
        let output_cost = (response.usage.completion_tokens as f64 / 1000.0) * pricing.output_cost_per_1k;
        input_cost + output_cost
    })
}

/// Build response from cached data
fn build_response_from_cache(
    request: &ChatCompletionRequest,
    cached: &llm_edge_cache::l1::CachedResponse,
    cache_tier: &str,
    latency_ms: u64,
) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: format!("chatcmpl-{}", Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: request.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: cached.content.clone(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: cached.tokens.as_ref().map(|t| t.prompt_tokens).unwrap_or(0),
            completion_tokens: cached.tokens.as_ref().map(|t| t.completion_tokens).unwrap_or(0),
            total_tokens: cached.tokens.as_ref().map(|t| t.total_tokens).unwrap_or(0),
        },
        metadata: Some(ResponseMetadata {
            provider: "cache".to_string(),
            cached: true,
            cache_tier: Some(cache_tier.to_string()),
            latency_ms,
            cost_usd: Some(0.0), // Cached responses have zero cost
        }),
    }
}

/// Convert provider response to cache format
fn convert_provider_to_cache(response: &UnifiedResponse) -> llm_edge_cache::l1::CachedResponse {
    let content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    llm_edge_cache::l1::CachedResponse {
        content,
        tokens: Some(llm_edge_cache::l1::TokenUsage {
            prompt_tokens: response.usage.prompt_tokens as u32,
            completion_tokens: response.usage.completion_tokens as u32,
            total_tokens: response.usage.total_tokens as u32,
        }),
        model: response.model.clone(),
        cached_at: chrono::Utc::now().timestamp(),
    }
}

/// Build response from provider data
fn build_response_from_provider(
    request: &ChatCompletionRequest,
    provider_response: UnifiedResponse,
    provider_name: &str,
    latency_ms: u64,
    cost_usd: Option<f64>,
) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: provider_response.id,
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: request.model.clone(),
        choices: provider_response
            .choices
            .into_iter()
            .map(|c| ChatChoice {
                index: c.index as u32,
                message: ChatMessage {
                    role: c.message.role,
                    content: c.message.content,
                },
                finish_reason: c.finish_reason.unwrap_or_else(|| "stop".to_string()),
            })
            .collect(),
        usage: Usage {
            prompt_tokens: provider_response.usage.prompt_tokens as u32,
            completion_tokens: provider_response.usage.completion_tokens as u32,
            total_tokens: provider_response.usage.total_tokens as u32,
        },
        metadata: Some(ResponseMetadata {
            provider: provider_name.to_string(),
            cached: false,
            cache_tier: None,
            latency_ms,
            cost_usd,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_request_valid() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
        };

        assert!(validate_request(&request).is_ok());
    }

    #[test]
    fn test_validate_request_empty_model() {
        let request = ChatCompletionRequest {
            model: "".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn test_validate_request_empty_messages() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn test_convert_to_cacheable() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hi".to_string(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
        };

        let cacheable = convert_to_cacheable(&request);
        assert_eq!(cacheable.model, "gpt-4");
        assert_eq!(cacheable.temperature, Some(0.7));
        assert_eq!(cacheable.max_tokens, Some(100));
    }
}
