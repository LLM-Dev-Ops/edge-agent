//! Mock LLM provider implementations

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::helpers::*;

/// Mock provider for testing
pub struct MockProvider {
    state: Arc<RwLock<MockProviderState>>,
}

struct MockProviderState {
    responses: Vec<Result<ChatCompletionResponse, ProviderError>>,
    call_count: u32,
    latency: Duration,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MockProviderState {
                responses: vec![],
                call_count: 0,
                latency: Duration::from_millis(10),
            })),
        }
    }

    pub fn expect_chat_completion(&self) -> &Self {
        // Mock implementation
        self
    }

    pub fn returning<F>(&self, _f: F) -> &Self
    where
        F: Fn(&ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> + Send + Sync + 'static,
    {
        // Mock implementation
        self
    }

    pub fn times(&self, _n: usize) -> &Self {
        self
    }

    pub async fn set_latency(&self, latency: Duration) {
        self.state.write().await.latency = latency;
    }

    pub async fn call_count(&self) -> u32 {
        self.state.read().await.call_count
    }
}

/// Mock OpenAI provider
pub struct MockOpenAI {
    inner: MockProvider,
}

impl MockOpenAI {
    pub fn new() -> Self {
        Self {
            inner: MockProvider::new(),
        }
    }

    pub fn with_response(self, response: ChatCompletionResponse) -> Self {
        // Mock implementation
        self
    }

    pub fn with_error(self, error: ProviderError) -> Self {
        // Mock implementation
        self
    }

    pub fn with_latency(self, latency: Duration) -> Self {
        // Mock implementation
        self
    }
}

/// Mock Anthropic provider
pub struct MockAnthropic {
    inner: MockProvider,
}

impl MockAnthropic {
    pub fn new() -> Self {
        Self {
            inner: MockProvider::new(),
        }
    }

    pub fn with_response(self, response: ChatCompletionResponse) -> Self {
        // Mock implementation
        self
    }

    pub fn with_error(self, error: ProviderError) -> Self {
        // Mock implementation
        self
    }
}

/// Provider error types
#[derive(Debug, Clone)]
pub enum ProviderError {
    Unauthorized,
    RateLimited,
    ServiceUnavailable,
    Timeout,
    InvalidRequest(String),
    Unknown(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::ServiceUnavailable => write!(f, "Service unavailable"),
            Self::Timeout => write!(f, "Timeout"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Mock HTTP server for provider endpoints
pub struct MockProviderServer {
    addr: String,
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn() -> String + Send + Sync>>>>,
}

impl MockProviderServer {
    pub fn new() -> Self {
        Self {
            addr: "http://localhost:8081".to_string(),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn mount(&self, path: &str, handler: impl Fn() -> String + Send + Sync + 'static) {
        // Mock implementation
    }

    pub fn url(&self) -> &str {
        &self.addr
    }

    pub async fn reset(&self) {
        self.handlers.write().await.clear();
    }
}

/// Wiremock-style expectations for providers
pub struct MockProviderExpectation {
    request_matcher: Option<Box<dyn Fn(&ChatCompletionRequest) -> bool + Send + Sync>>,
    response: Option<ChatCompletionResponse>,
    error: Option<ProviderError>,
    delay: Option<Duration>,
}

impl MockProviderExpectation {
    pub fn new() -> Self {
        Self {
            request_matcher: None,
            response: None,
            error: None,
            delay: None,
        }
    }

    pub fn when<F>(mut self, matcher: F) -> Self
    where
        F: Fn(&ChatCompletionRequest) -> bool + Send + Sync + 'static,
    {
        self.request_matcher = Some(Box::new(matcher));
        self
    }

    pub fn respond_with(mut self, response: ChatCompletionResponse) -> Self {
        self.response = Some(response);
        self
    }

    pub fn respond_with_error(mut self, error: ProviderError) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}
