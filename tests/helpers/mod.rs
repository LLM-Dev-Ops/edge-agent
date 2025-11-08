//! Test helpers and utilities for integration tests

use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

pub mod server;
pub mod config;
pub mod metrics;
pub mod fixtures;

pub use server::TestServer;
pub use config::TestConfig;
pub use metrics::TestMetrics;
pub use fixtures::*;

// =============================================================================
// Common Test Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedHealth {
    pub status: String,
    pub l1_healthy: bool,
    pub l2_healthy: bool,
    pub providers: Vec<ProviderHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub name: String,
    pub healthy: bool,
    pub circuit_breaker_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

// =============================================================================
// Circuit Breaker States
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

// =============================================================================
// Routing Strategies
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    RoundRobin,
    Failover,
    LeastLatency,
    CostOptimized,
    HealthAware,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a test chat completion request
pub fn create_test_request(content: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: content.to_string(),
            }
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: None,
    }
}

/// Create a test chat completion response
pub fn create_test_response(content: &str) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        model: "gpt-4".to_string(),
        choices: vec![
            Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: content.to_string(),
                },
                finish_reason: "stop".to_string(),
            }
        ],
        usage: Some(Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        }),
        provider: "openai".to_string(),
    }
}

/// Wait for condition with timeout
pub async fn wait_for<F>(mut condition: F, timeout: Duration) -> bool
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

/// Assert metrics match expected values
#[macro_export]
macro_rules! assert_metrics {
    ($metrics:expr, { $($field:ident: $value:expr),* $(,)? }) => {
        let snapshot = $metrics.snapshot();
        $(
            assert_eq!(
                snapshot.$field,
                $value,
                "Metric {} expected {} but got {}",
                stringify!($field),
                $value,
                snapshot.$field
            );
        )*
    };
}

// =============================================================================
// Test Assertions
// =============================================================================

/// Assert response is successful and contains expected data
pub fn assert_success_response(response: &ChatCompletionResponse) {
    assert!(!response.id.is_empty(), "Response ID should not be empty");
    assert!(!response.model.is_empty(), "Model should not be empty");
    assert!(!response.choices.is_empty(), "Should have at least one choice");
    assert!(!response.choices[0].message.content.is_empty(), "Content should not be empty");
}

/// Assert error response contains expected error
pub fn assert_error_response(response: &ErrorResponse, expected_error: &str) {
    assert!(
        response.message.contains(expected_error),
        "Error message '{}' should contain '{}'",
        response.message,
        expected_error
    );
}
