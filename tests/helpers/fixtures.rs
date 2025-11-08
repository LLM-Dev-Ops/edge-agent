//! Test fixtures and sample data

use super::*;

/// Sample chat completion requests for testing
pub mod requests {
    use super::*;

    pub fn simple() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Hello, how are you?".to_string(),
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
        }
    }

    pub fn with_system_message() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "What is Rust?".to_string(),
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(200),
            stream: None,
        }
    }

    pub fn streaming() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Write a story".to_string(),
                }
            ],
            temperature: Some(0.9),
            max_tokens: Some(500),
            stream: Some(true),
        }
    }

    pub fn large_prompt() -> ChatCompletionRequest {
        let large_content = "x".repeat(10_000);
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: large_content,
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
        }
    }

    pub fn with_pii() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "My email is john.doe@example.com and my SSN is 123-45-6789. \
                              My credit card is 4532-1234-5678-9010.".to_string(),
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
        }
    }
}

/// Sample chat completion responses for testing
pub mod responses {
    use super::*;

    pub fn simple() -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-4".to_string(),
            choices: vec![
                Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content: "I'm doing well, thank you!".to_string(),
                    },
                    finish_reason: "stop".to_string(),
                }
            ],
            usage: Some(Usage {
                prompt_tokens: 12,
                completion_tokens: 8,
                total_tokens: 20,
            }),
            provider: "openai".to_string(),
        }
    }

    pub fn from_anthropic() -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: "msg_abc123".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            choices: vec![
                Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content: "Hello! I'm Claude, an AI assistant.".to_string(),
                    },
                    finish_reason: "end_turn".to_string(),
                }
            ],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 12,
                total_tokens: 22,
            }),
            provider: "anthropic".to_string(),
        }
    }

    pub fn long_response() -> ChatCompletionResponse {
        let long_content = "This is a long response. ".repeat(100);
        ChatCompletionResponse {
            id: "chatcmpl-456".to_string(),
            model: "gpt-4".to_string(),
            choices: vec![
                Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content: long_content,
                    },
                    finish_reason: "stop".to_string(),
                }
            ],
            usage: Some(Usage {
                prompt_tokens: 20,
                completion_tokens: 500,
                total_tokens: 520,
            }),
            provider: "openai".to_string(),
        }
    }
}

/// Error fixtures for testing
pub mod errors {
    use super::*;

    pub fn unauthorized() -> ErrorResponse {
        ErrorResponse {
            error: "401".to_string(),
            message: "Invalid API key".to_string(),
        }
    }

    pub fn rate_limited() -> ErrorResponse {
        ErrorResponse {
            error: "429".to_string(),
            message: "Rate limit exceeded".to_string(),
        }
    }

    pub fn service_unavailable() -> ErrorResponse {
        ErrorResponse {
            error: "503".to_string(),
            message: "Service temporarily unavailable".to_string(),
        }
    }

    pub fn timeout() -> ErrorResponse {
        ErrorResponse {
            error: "408".to_string(),
            message: "Request timeout".to_string(),
        }
    }

    pub fn invalid_model() -> ErrorResponse {
        ErrorResponse {
            error: "400".to_string(),
            message: "Invalid model specified".to_string(),
        }
    }
}

/// Metrics fixtures for testing
pub mod metrics_data {
    use super::MetricsSnapshot;

    pub fn empty() -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: 0,
            cache_hits: 0,
            cache_misses: 0,
            l1_hits: 0,
            l1_misses: 0,
            l1_writes: 0,
            l2_hits: 0,
            l2_misses: 0,
            l2_writes: 0,
            provider_requests: 0,
            cache_writes: 0,
            total_cost: 0.0,
        }
    }

    pub fn with_cache_hits() -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: 10,
            cache_hits: 7,
            cache_misses: 3,
            l1_hits: 7,
            l1_misses: 3,
            l1_writes: 3,
            l2_hits: 0,
            l2_misses: 3,
            l2_writes: 3,
            provider_requests: 3,
            cache_writes: 3,
            total_cost: 0.0027, // 3 requests * 30 tokens * $0.03/1K
        }
    }
}
