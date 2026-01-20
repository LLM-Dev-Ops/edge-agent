//! RuVector Service Client
//!
//! Async, non-blocking client for persisting DecisionEvents to ruvector-service.
//! LLM-Edge-Agent NEVER connects directly to Google SQL - all persistence
//! flows through this client.

use std::time::Duration;
use agentics_contracts::DecisionEvent;
use crate::circuit_breaker::CircuitBreakerPersistence;
use tracing::{debug, error, info, warn};
use crate::error::{AgentError, AgentResult};

/// Configuration for RuVector service client
#[derive(Debug, Clone)]
pub struct RuVectorConfig {
    /// RuVector service endpoint
    pub endpoint: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retries for transient failures
    pub max_retries: u32,
    /// Base delay between retries
    pub retry_delay: Duration,
}

impl Default for RuVectorConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:3000".to_string(),
            api_key: None,
            timeout: Duration::from_secs(5),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

impl RuVectorConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("RUVECTOR_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").ok(),
            timeout: std::env::var("RUVECTOR_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            max_retries: std::env::var("RUVECTOR_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            retry_delay: std::env::var("RUVECTOR_RETRY_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_millis)
                .unwrap_or(Duration::from_millis(100)),
        }
    }
}

/// RuVector service client for persisting agent decisions
///
/// This client provides async, non-blocking writes to ruvector-service.
/// All persistence in LLM-Edge-Agent flows through this client.
pub struct RuVectorClient {
    client: reqwest::Client,
    config: RuVectorConfig,
}

impl RuVectorClient {
    /// Create a new RuVector client
    pub fn new(config: RuVectorConfig) -> AgentResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| AgentError::ConfigurationError(format!("Failed to create HTTP client: {}", e)))?;

        info!("RuVector client initialized with endpoint: {}", config.endpoint);

        Ok(Self { client, config })
    }

    /// Create a new RuVector client from environment variables
    pub fn from_env() -> AgentResult<Self> {
        Self::new(RuVectorConfig::from_env())
    }

    /// Persist a DecisionEvent to ruvector-service
    ///
    /// This is an async, non-blocking operation. Failures are logged but
    /// do not block the calling agent's response.
    pub async fn persist_decision(&self, event: &DecisionEvent) -> AgentResult<()> {
        let url = format!("{}/api/v1/decisions", self.config.endpoint);

        debug!(
            "Persisting DecisionEvent: agent={}, type={:?}, execution_ref={}",
            event.agent_id, event.decision_type, event.execution_ref
        );

        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(self.config.retry_delay * attempt).await;
                debug!("Retrying persist_decision, attempt {}", attempt + 1);
            }

            let mut request = self.client
                .post(&url)
                .json(event);

            if let Some(ref api_key) = self.config.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        debug!("DecisionEvent persisted successfully: {}", event.event_id);
                        return Ok(());
                    }

                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();

                    if status.is_client_error() {
                        // Client errors are not retryable
                        return Err(AgentError::RuVectorError(
                            format!("Client error {}: {}", status, body)
                        ));
                    }

                    last_error = Some(format!("Server error {}: {}", status, body));
                }
                Err(e) => {
                    last_error = Some(format!("Request failed: {}", e));
                }
            }
        }

        let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
        error!("Failed to persist DecisionEvent after {} attempts: {}", self.config.max_retries + 1, error_msg);
        Err(AgentError::RuVectorError(error_msg))
    }

    /// Persist circuit breaker state to ruvector-service
    pub async fn persist_circuit_breaker_state(&self, state: &CircuitBreakerPersistence) -> AgentResult<()> {
        let url = format!("{}/api/v1/circuit-breaker/{}", self.config.endpoint, state.provider_id);

        debug!(
            "Persisting circuit breaker state: provider={}, state={:?}",
            state.provider_id, state.state
        );

        let mut request = self.client
            .put(&url)
            .json(state);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            debug!("Circuit breaker state persisted: {}", state.provider_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::RuVectorError(
                format!("Failed to persist circuit breaker state: {} - {}", status, body)
            ))
        }
    }

    /// Load circuit breaker state from ruvector-service
    pub async fn load_circuit_breaker_state(&self, provider_id: &str) -> AgentResult<Option<CircuitBreakerPersistence>> {
        let url = format!("{}/api/v1/circuit-breaker/{}", self.config.endpoint, provider_id);

        debug!("Loading circuit breaker state for provider: {}", provider_id);

        let mut request = self.client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let state: CircuitBreakerPersistence = response.json().await?;
            debug!("Circuit breaker state loaded: {}", provider_id);
            Ok(Some(state))
        } else if response.status().as_u16() == 404 {
            debug!("No circuit breaker state found for: {}", provider_id);
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::RuVectorError(
                format!("Failed to load circuit breaker state: {} - {}", status, body)
            ))
        }
    }

    /// Health check for ruvector-service
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.endpoint);

        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(e) => {
                warn!("RuVector health check failed: {}", e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RuVectorConfig::default();
        assert_eq!(config.endpoint, "http://localhost:3000");
        assert_eq!(config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = RuVectorConfig::default();
        let client = RuVectorClient::new(config);
        assert!(client.is_ok());
    }
}
