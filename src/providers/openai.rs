// OpenAI provider implementation
// Supports GPT-4, GPT-3.5, and O1 models

use super::{
    LLMProvider, LLMRequest, LLMResponse, Message, MessageContent, Choice, Usage,
    FinishReason, ProviderError, ProviderResult, HealthStatus, ProviderCapabilities, Role,
};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const OPENAI_HEALTH_MODEL: &str = "gpt-3.5-turbo";

/// OpenAI provider implementation
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    timeout_ms: u64,
    max_retries: u32,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String, timeout_ms: u64, max_retries: u32) -> ProviderResult<Self> {
        // Create HTTP client with connection pooling
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .use_rustls_tls()
            .build()
            .map_err(|e| ProviderError::InternalError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key,
            timeout_ms,
            max_retries,
        })
    }

    /// Transform our unified request to OpenAI format
    fn transform_request(&self, request: &LLMRequest) -> OpenAIRequest {
        OpenAIRequest {
            model: request.model.clone(),
            messages: request.messages.iter().map(|m| self.transform_message(m)).collect(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences.clone(),
            stream: Some(request.stream),
            n: Some(1),
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: request.metadata.as_ref().and_then(|m| m.user_id.clone()),
        }
    }

    /// Transform a message to OpenAI format
    fn transform_message(&self, message: &Message) -> OpenAIMessage {
        let content = match &message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => {
                // For OpenAI, we'll serialize complex content as JSON
                // In a real implementation, this should handle vision content properly
                serde_json::to_string(parts).unwrap_or_default()
            }
        };

        OpenAIMessage {
            role: self.transform_role(&message.role),
            content,
            name: message.name.clone(),
        }
    }

    /// Transform role to OpenAI format
    fn transform_role(&self, role: &Role) -> String {
        match role {
            Role::System => "system".to_string(),
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::Function => "function".to_string(),
            Role::Tool => "tool".to_string(),
        }
    }

    /// Transform OpenAI response to our unified format
    fn transform_response(&self, response: OpenAIResponse) -> LLMResponse {
        LLMResponse {
            id: response.id,
            model: response.model,
            choices: response.choices.into_iter().map(|c| {
                Choice {
                    index: c.index,
                    message: Message {
                        role: self.parse_role(&c.message.role),
                        content: MessageContent::Text(c.message.content.unwrap_or_default()),
                        name: c.message.name,
                    },
                    finish_reason: c.finish_reason.and_then(|r| self.parse_finish_reason(&r)),
                }
            }).collect(),
            usage: Usage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
            created: response.created,
            metadata: None,
        }
    }

    /// Parse role from string
    fn parse_role(&self, role: &str) -> Role {
        match role {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "function" => Role::Function,
            "tool" => Role::Tool,
            _ => Role::Assistant, // Default fallback
        }
    }

    /// Parse finish reason
    fn parse_finish_reason(&self, reason: &str) -> Option<FinishReason> {
        match reason {
            "stop" => Some(FinishReason::Stop),
            "length" => Some(FinishReason::Length),
            "content_filter" => Some(FinishReason::ContentFilter),
            "tool_calls" | "function_call" => Some(FinishReason::ToolCalls),
            _ => None,
        }
    }

    /// Send a request with retry logic
    async fn send_request(&self, request: &LLMRequest) -> ProviderResult<OpenAIResponse> {
        let openai_request = self.transform_request(request);
        let url = format!("{}/chat/completions", OPENAI_API_BASE);

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // Exponential backoff: 100ms, 200ms, 400ms, etc.
                let backoff_ms = 100 * (1 << (attempt - 1));
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.client
                .post(&url)
                .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
                .header(header::CONTENT_TYPE, "application/json")
                .json(&openai_request)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        match response.json::<OpenAIResponse>().await {
                            Ok(openai_response) => return Ok(openai_response),
                            Err(e) => {
                                last_error = Some(ProviderError::SerializationError(
                                    serde_json::Error::custom(format!("Failed to parse response: {}", e))
                                ));
                                continue;
                            }
                        }
                    } else if status.as_u16() == 401 {
                        return Err(ProviderError::InvalidApiKey {
                            provider: "openai".to_string(),
                        });
                    } else if status.as_u16() == 429 {
                        // Rate limit - retry
                        last_error = Some(ProviderError::RateLimitExceeded {
                            message: "OpenAI rate limit exceeded".to_string(),
                        });
                        continue;
                    } else {
                        let error_body = response.text().await.unwrap_or_default();
                        return Err(ProviderError::ProviderError {
                            message: format!("OpenAI API error ({}): {}", status, error_body),
                        });
                    }
                }
                Err(e) if e.is_timeout() => {
                    last_error = Some(ProviderError::Timeout { timeout_ms: self.timeout_ms });
                    continue;
                }
                Err(e) => {
                    last_error = Some(ProviderError::HttpError(e));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::InternalError("Unknown error".to_string())))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            max_context_tokens: 128000, // GPT-4 Turbo
            max_output_tokens: 4096,
        }
    }

    async fn complete(&self, request: LLMRequest) -> ProviderResult<LLMResponse> {
        let start = Instant::now();

        // Validate model
        if !self.validate_model(&request.model) {
            return Err(ProviderError::ModelNotFound {
                model: request.model.clone(),
            });
        }

        let openai_response = self.send_request(&request).await?;
        let response = self.transform_response(openai_response);

        let elapsed = start.elapsed();
        tracing::info!(
            provider = "openai",
            model = %request.model,
            tokens = response.usage.total_tokens,
            latency_ms = elapsed.as_millis() as u64,
            "Completed OpenAI request"
        );

        Ok(response)
    }

    async fn health_check(&self) -> ProviderResult<HealthStatus> {
        let start = Instant::now();

        // Simple health check: try to list models
        let url = format!("{}/models", OPENAI_API_BASE);

        match self.client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let elapsed = start.elapsed();
                Ok(HealthStatus {
                    healthy: true,
                    last_check: chrono::Utc::now().timestamp(),
                    response_time_ms: Some(elapsed.as_millis() as u64),
                    error: None,
                })
            }
            Ok(response) => {
                Ok(HealthStatus {
                    healthy: false,
                    last_check: chrono::Utc::now().timestamp(),
                    response_time_ms: None,
                    error: Some(format!("HTTP {}", response.status())),
                })
            }
            Err(e) => {
                Ok(HealthStatus {
                    healthy: false,
                    last_check: chrono::Utc::now().timestamp(),
                    response_time_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4-turbo-preview".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
            "o1-preview".to_string(),
            "o1-mini".to_string(),
        ]
    }
}

// OpenAI API request format
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logit_bias: Option<std::collections::HashMap<String, f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    created: i64,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            30000,
            3
        );
        assert!(provider.is_ok());
    }

    #[test]
    fn test_model_validation() {
        let provider = OpenAIProvider::new("test-key".to_string(), 30000, 3).unwrap();
        assert!(provider.validate_model("gpt-4"));
        assert!(provider.validate_model("gpt-3.5-turbo"));
        assert!(!provider.validate_model("invalid-model"));
    }

    #[test]
    fn test_list_models() {
        let provider = OpenAIProvider::new("test-key".to_string(), 30000, 3).unwrap();
        let models = provider.list_models();
        assert!(!models.is_empty());
        assert!(models.contains(&"gpt-4".to_string()));
    }
}
