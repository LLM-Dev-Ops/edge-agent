// Anthropic provider implementation
// Supports Claude 3.5 Sonnet, Claude 3 Opus, Sonnet, and Haiku

use super::{
    LLMProvider, LLMRequest, LLMResponse, Message, MessageContent, Choice, Usage,
    FinishReason, ProviderError, ProviderResult, HealthStatus, ProviderCapabilities, Role, ContentPart,
};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic provider implementation
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    timeout_ms: u64,
    max_retries: u32,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
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

    /// Transform our unified request to Anthropic format
    fn transform_request(&self, request: &LLMRequest) -> AnthropicRequest {
        // Separate system messages from other messages
        let (system_message, messages) = self.extract_system_message(&request.messages);

        AnthropicRequest {
            model: request.model.clone(),
            messages: messages.into_iter().map(|m| self.transform_message(m)).collect(),
            system: system_message,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k.map(|k| k as i32),
            stop_sequences: request.stop_sequences.clone(),
            stream: Some(request.stream),
            metadata: None,
        }
    }

    /// Extract system message and return it separately (Anthropic format)
    fn extract_system_message(&self, messages: &[Message]) -> (Option<String>, Vec<&Message>) {
        let mut system_content = Vec::new();
        let mut other_messages = Vec::new();

        for msg in messages {
            if matches!(msg.role, Role::System) {
                if let MessageContent::Text(text) = &msg.content {
                    system_content.push(text.clone());
                }
            } else {
                other_messages.push(msg);
            }
        }

        let system = if system_content.is_empty() {
            None
        } else {
            Some(system_content.join("\n\n"))
        };

        (system, other_messages)
    }

    /// Transform a message to Anthropic format
    fn transform_message(&self, message: &Message) -> AnthropicMessage {
        let content = match &message.content {
            MessageContent::Text(text) => {
                AnthropicContent::Text(text.clone())
            }
            MessageContent::Parts(parts) => {
                AnthropicContent::Blocks(
                    parts.iter().map(|p| self.transform_content_part(p)).collect()
                )
            }
        };

        AnthropicMessage {
            role: self.transform_role(&message.role),
            content,
        }
    }

    /// Transform content part to Anthropic format
    fn transform_content_part(&self, part: &ContentPart) -> AnthropicContentBlock {
        match part {
            ContentPart::Text { text } => {
                AnthropicContentBlock::Text {
                    r#type: "text".to_string(),
                    text: text.clone(),
                }
            }
            ContentPart::Image { source } => {
                // Transform image source
                match source {
                    super::ImageSource::Url { url } => {
                        // Anthropic doesn't support URLs directly, would need to download
                        AnthropicContentBlock::Text {
                            r#type: "text".to_string(),
                            text: format!("[Image: {}]", url),
                        }
                    }
                    super::ImageSource::Base64 { media_type, data } => {
                        AnthropicContentBlock::Image {
                            r#type: "image".to_string(),
                            source: AnthropicImageSource {
                                r#type: "base64".to_string(),
                                media_type: media_type.clone(),
                                data: data.clone(),
                            },
                        }
                    }
                }
            }
        }
    }

    /// Transform role to Anthropic format
    fn transform_role(&self, role: &Role) -> String {
        match role {
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            // Anthropic only supports user/assistant in messages
            _ => "user".to_string(),
        }
    }

    /// Transform Anthropic response to our unified format
    fn transform_response(&self, response: AnthropicResponse) -> LLMResponse {
        let content = match &response.content[0] {
            AnthropicContentBlock::Text { text, .. } => text.clone(),
            _ => String::new(),
        };

        LLMResponse {
            id: response.id,
            model: response.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: MessageContent::Text(content),
                    name: None,
                },
                finish_reason: self.parse_stop_reason(&response.stop_reason),
            }],
            usage: Usage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            },
            created: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Parse stop reason
    fn parse_stop_reason(&self, reason: &str) -> Option<FinishReason> {
        match reason {
            "end_turn" => Some(FinishReason::EndTurn),
            "max_tokens" => Some(FinishReason::Length),
            "stop_sequence" => Some(FinishReason::Stop),
            _ => None,
        }
    }

    /// Send a request with retry logic
    async fn send_request(&self, request: &LLMRequest) -> ProviderResult<AnthropicResponse> {
        let anthropic_request = self.transform_request(request);
        let url = format!("{}/messages", ANTHROPIC_API_BASE);

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // Exponential backoff: 100ms, 200ms, 400ms, etc.
                let backoff_ms = 100 * (1 << (attempt - 1));
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header(header::CONTENT_TYPE, "application/json")
                .json(&anthropic_request)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        match response.json::<AnthropicResponse>().await {
                            Ok(anthropic_response) => return Ok(anthropic_response),
                            Err(e) => {
                                last_error = Some(ProviderError::SerializationError(
                                    serde_json::Error::custom(format!("Failed to parse response: {}", e))
                                ));
                                continue;
                            }
                        }
                    } else if status.as_u16() == 401 {
                        return Err(ProviderError::InvalidApiKey {
                            provider: "anthropic".to_string(),
                        });
                    } else if status.as_u16() == 429 {
                        // Rate limit - retry
                        last_error = Some(ProviderError::RateLimitExceeded {
                            message: "Anthropic rate limit exceeded".to_string(),
                        });
                        continue;
                    } else {
                        let error_body = response.text().await.unwrap_or_default();
                        return Err(ProviderError::ProviderError {
                            message: format!("Anthropic API error ({}): {}", status, error_body),
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
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            max_context_tokens: 200000, // Claude 3 supports up to 200k
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

        let anthropic_response = self.send_request(&request).await?;
        let response = self.transform_response(anthropic_response);

        let elapsed = start.elapsed();
        tracing::info!(
            provider = "anthropic",
            model = %request.model,
            tokens = response.usage.total_tokens,
            latency_ms = elapsed.as_millis() as u64,
            "Completed Anthropic request"
        );

        Ok(response)
    }

    async fn health_check(&self) -> ProviderResult<HealthStatus> {
        let start = Instant::now();

        // Simple health check: make a minimal request
        let test_request = LLMRequest::new(
            "claude-3-haiku-20240307",
            vec![Message::user("Hi")]
        ).with_max_tokens(10);

        match self.send_request(&test_request).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                Ok(HealthStatus {
                    healthy: true,
                    last_check: chrono::Utc::now().timestamp(),
                    response_time_ms: Some(elapsed.as_millis() as u64),
                    error: None,
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
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3.5-sonnet".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-opus".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-3-haiku".to_string(),
        ]
    }
}

// Anthropic API request format
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<AnthropicMetadata>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text {
        r#type: String,
        text: String,
    },
    #[serde(rename = "image")]
    Image {
        r#type: String,
        source: AnthropicImageSource,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicImageSource {
    r#type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct AnthropicMetadata {
    user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContentBlock>,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new(
            "test-key".to_string(),
            30000,
            3
        );
        assert!(provider.is_ok());
    }

    #[test]
    fn test_model_validation() {
        let provider = AnthropicProvider::new("test-key".to_string(), 30000, 3).unwrap();
        assert!(provider.validate_model("claude-3-5-sonnet-20241022"));
        assert!(provider.validate_model("claude-3-opus"));
        assert!(!provider.validate_model("invalid-model"));
    }

    #[test]
    fn test_list_models() {
        let provider = AnthropicProvider::new("test-key".to_string(), 30000, 3).unwrap();
        let models = provider.list_models();
        assert!(!models.is_empty());
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
    }

    #[test]
    fn test_system_message_extraction() {
        let provider = AnthropicProvider::new("test-key".to_string(), 30000, 3).unwrap();
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello"),
        ];

        let (system, other) = provider.extract_system_message(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "You are a helpful assistant");
        assert_eq!(other.len(), 1);
    }
}
