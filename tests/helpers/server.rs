//! Test server implementation for integration tests

use super::config::TestConfig;
use super::metrics::TestMetrics;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Test server wrapper
pub struct TestServer {
    config: TestConfig,
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    metrics: Arc<TestMetrics>,
    state: Arc<RwLock<TestServerState>>,
}

/// Internal server state
struct TestServerState {
    running: bool,
    provider_states: HashMap<String, ProviderState>,
}

struct ProviderState {
    healthy: bool,
    failing: bool,
    latency: std::time::Duration,
}

impl TestServer {
    /// Create new test server
    pub async fn new(config: TestConfig) -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // Random port
        let metrics = Arc::new(TestMetrics::new());

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let state = Arc::new(RwLock::new(TestServerState {
            running: true,
            provider_states: HashMap::new(),
        }));

        // Initialize default providers
        let mut state_lock = state.write().await;
        state_lock.provider_states.insert(
            "openai".to_string(),
            ProviderState {
                healthy: true,
                failing: false,
                latency: std::time::Duration::from_millis(10),
            },
        );
        state_lock.provider_states.insert(
            "anthropic".to_string(),
            ProviderState {
                healthy: true,
                failing: false,
                latency: std::time::Duration::from_millis(10),
            },
        );
        drop(state_lock);

        // TODO: Start actual server
        // For now, this is a mock implementation

        Self {
            config,
            addr,
            shutdown_tx: Some(shutdown_tx),
            metrics,
            state,
        }
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        // In real implementation, would check actual server status
        self.shutdown_tx.is_some()
    }

    /// Get server metrics
    pub fn metrics(&self) -> Arc<TestMetrics> {
        self.metrics.clone()
    }

    /// Get cache manager (mock)
    pub fn cache_manager(&self) -> MockCacheManager {
        MockCacheManager::new()
    }

    /// Get tracer (mock)
    pub fn tracer(&self) -> MockTracer {
        MockTracer::new()
    }

    /// Get log capture (mock)
    pub fn log_capture(&self) -> MockLogCapture {
        MockLogCapture::new()
    }

    /// Make GET request
    pub async fn get(&self, path: &str) -> TestResponse {
        self.request("GET", path).await
    }

    /// Make POST request
    pub fn post(&self, path: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self, "POST", path)
    }

    /// Make generic request
    async fn request(&self, method: &str, path: &str) -> TestResponse {
        // Mock implementation
        TestResponse::new(200, serde_json::json!({
            "status": "healthy",
            "version": "0.1.0",
            "service": "llm-edge-agent"
        }))
    }

    /// Set provider latency
    pub async fn set_provider_latency(&self, provider: &str, latency: std::time::Duration) {
        let mut state = self.state.write().await;
        if let Some(provider_state) = state.provider_states.get_mut(provider) {
            provider_state.latency = latency;
        }
    }

    /// Set provider healthy status
    pub async fn set_provider_healthy(&self, provider: &str, healthy: bool) {
        let mut state = self.state.write().await;
        if let Some(provider_state) = state.provider_states.get_mut(provider) {
            provider_state.healthy = healthy;
        }
    }

    /// Set provider failing status
    pub async fn set_provider_failing(&self, provider: &str, failing: bool) {
        let mut state = self.state.write().await;
        if let Some(provider_state) = state.provider_states.get_mut(provider) {
            provider_state.failing = failing;
        }
    }

    /// Get circuit breaker state
    pub async fn circuit_breaker_state(&self, provider: &str) -> super::CircuitBreakerState {
        // Mock implementation
        super::CircuitBreakerState::Closed
    }

    /// Register mock provider
    pub fn register_mock_provider(&self, name: &str, mock: impl std::any::Any) {
        // Mock implementation
    }

    /// Shutdown server
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        let mut state = self.state.write().await;
        state.running = false;
    }
}

/// Test request builder
pub struct TestRequestBuilder<'a> {
    server: &'a TestServer,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<serde_json::Value>,
}

impl<'a> TestRequestBuilder<'a> {
    fn new(server: &'a TestServer, method: &str, path: &str) -> Self {
        Self {
            server,
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn json(mut self, json: &impl serde::Serialize) -> Self {
        self.body = Some(serde_json::to_value(json).unwrap());
        self.headers.insert("content-type".to_string(), "application/json".to_string());
        self
    }

    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers.insert("authorization".to_string(), format!("Bearer {}", token));
        self
    }

    pub async fn send(self) -> TestResponse {
        // Mock implementation - in real tests would make actual HTTP request
        let status = if self.path == "/v1/chat/completions" {
            200
        } else if self.path == "/health" {
            200
        } else if self.path == "/metrics" {
            200
        } else {
            404
        };

        let response_body = match self.path.as_str() {
            "/health" => serde_json::json!({
                "status": "healthy",
                "version": "0.1.0",
                "service": "llm-edge-agent"
            }),
            "/metrics" => serde_json::json!("# TYPE llm_requests_total counter\nllm_requests_total 0\n"),
            "/v1/chat/completions" => serde_json::json!({
                "id": "chatcmpl-123",
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Test response"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                },
                "provider": "openai"
            }),
            _ => serde_json::json!({"error": "Not found"})
        };

        TestResponse::new(status, response_body)
    }
}

/// Test response
pub struct TestResponse {
    status: u16,
    body: serde_json::Value,
    headers: HashMap<String, String>,
}

impl TestResponse {
    fn new(status: u16, body: serde_json::Value) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        Self {
            status,
            body,
            headers,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub async fn json<T: serde::de::DeserializeOwned>(self) -> T {
        serde_json::from_value(self.body).unwrap()
    }

    pub async fn text(self) -> String {
        self.body.to_string()
    }
}

// Mock implementations for testing infrastructure

pub struct MockCacheManager;

impl MockCacheManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn store_l2_only(&self, request: &super::ChatCompletionRequest, response: super::ChatCompletionResponse) {
        // Mock implementation
    }
}

pub struct MockTracer {
    traces: Arc<RwLock<Vec<Trace>>>,
}

impl MockTracer {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_traces(&self) -> Vec<Trace> {
        self.traces.read().await.clone()
    }
}

#[derive(Clone)]
pub struct Trace {
    pub spans: Vec<Span>,
}

#[derive(Clone)]
pub struct Span {
    pub name: String,
}

pub struct MockLogCapture {
    logs: Arc<RwLock<Vec<String>>>,
}

impl MockLogCapture {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_logs(&self) -> Vec<String> {
        self.logs.read().await.clone()
    }

    pub async fn get_logs_at_level(&self, level: &str) -> Vec<String> {
        self.logs.read().await
            .iter()
            .filter(|log| log.contains(level))
            .cloned()
            .collect()
    }
}
