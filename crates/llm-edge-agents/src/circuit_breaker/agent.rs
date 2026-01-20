//! Circuit Breaker Agent Implementation
//!
//! This is the main agent that ties together:
//! - Decision engine (deterministic logic)
//! - RuVector client (persistence)
//! - Telemetry emission
//! - DecisionEvent creation

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use sha2::{Sha256, Digest};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::contracts::{
    AgentMetadata, AgentType, Constraint, ConstraintType, DecisionEvent, DecisionType,
};
use crate::error::{AgentError, AgentResult};

use super::decision::CircuitBreakerDecisionEngine;
use super::types::*;

/// Agent identifier
pub const AGENT_ID: &str = "circuit_breaker_agent";

/// Agent version
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Circuit Breaker Agent
///
/// # Purpose
/// Prevent cascading failures by blocking execution paths when thresholds are exceeded.
///
/// # Classification
/// - Type: PROTECTION / GUARD
/// - Decision Type: `circuit_breaker_decision`
///
/// # Invocation
/// This agent is invoked by LLM-Orchestrator during execution flow.
///
/// # Persistence
/// All state is persisted via ruvector-service client calls.
/// This agent NEVER persists state locally.
pub struct CircuitBreakerAgent {
    /// RuVector client for persistence (optional)
    ruvector_client: Option<RuVectorClient>,
    /// Agent metadata
    metadata: AgentMetadata,
}

/// RuVector client for persistence
pub struct RuVectorClient {
    endpoint: String,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl RuVectorClient {
    pub fn new(endpoint: impl Into<String>, api_key: Option<String>) -> AgentResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| AgentError::ConfigurationError(e.to_string()))?;

        Ok(Self {
            endpoint: endpoint.into(),
            client,
            api_key,
        })
    }

    pub fn from_env() -> AgentResult<Self> {
        let endpoint = std::env::var("RUVECTOR_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let api_key = std::env::var("RUVECTOR_API_KEY").ok();
        Self::new(endpoint, api_key)
    }

    /// Persist a DecisionEvent
    pub async fn persist_decision(&self, event: &DecisionEvent) -> AgentResult<()> {
        let url = format!("{}/api/v1/decisions", self.endpoint);

        let mut request = self.client.post(&url).json(event);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            debug!("DecisionEvent persisted: {}", event.event_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::PersistenceError(format!(
                "Failed to persist decision: {} - {}",
                status, body
            )))
        }
    }

    /// Persist circuit breaker state
    pub async fn persist_state(&self, state: &CircuitBreakerPersistence) -> AgentResult<()> {
        let url = format!(
            "{}/api/v1/circuit-breaker/{}",
            self.endpoint, state.provider_id
        );

        let mut request = self.client.put(&url).json(state);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            debug!("Circuit breaker state persisted: {}", state.provider_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::PersistenceError(format!(
                "Failed to persist state: {} - {}",
                status, body
            )))
        }
    }

    /// Load circuit breaker state
    pub async fn load_state(&self, provider_id: &str) -> AgentResult<Option<CircuitBreakerPersistence>> {
        let url = format!("{}/api/v1/circuit-breaker/{}", self.endpoint, provider_id);

        let mut request = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let state = response.json().await?;
            Ok(Some(state))
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentError::PersistenceError(format!(
                "Failed to load state: {} - {}",
                status, body
            )))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.endpoint);
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

impl CircuitBreakerAgent {
    /// Create a new Circuit Breaker Agent
    pub fn new(ruvector_client: Option<RuVectorClient>) -> Self {
        Self {
            ruvector_client,
            metadata: AgentMetadata {
                agent_id: AGENT_ID.to_string(),
                agent_version: AGENT_VERSION.to_string(),
                agent_type: AgentType::Protection,
            },
        }
    }

    /// Create from environment configuration
    pub fn from_env() -> AgentResult<Self> {
        let ruvector = if std::env::var("RUVECTOR_ENABLED")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false)
        {
            Some(RuVectorClient::from_env()?)
        } else {
            None
        };

        Ok(Self::new(ruvector))
    }

    /// Invoke the agent with a request
    ///
    /// This is the main entry point for agent invocation.
    /// It:
    /// 1. Validates input
    /// 2. Evaluates circuit breaker decision
    /// 3. Emits DecisionEvent to ruvector-service
    /// 4. Returns response
    #[instrument(skip(self), fields(provider_id = %request.provider_id, execution_ref = %request.execution_ref))]
    pub async fn invoke(&self, request: CircuitBreakerRequest) -> AgentResult<CircuitBreakerResponse> {
        let start = Instant::now();

        // 1. Validate input
        CircuitBreakerDecisionEngine::validate_input(&request)
            .map_err(|e| AgentError::ValidationError(e))?;

        info!(
            provider_id = %request.provider_id,
            current_state = %request.current_state,
            "Processing circuit breaker request"
        );

        // 2. Evaluate decision
        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        // 3. Build constraints applied
        let constraints = self.build_constraints(&request, &response);

        // 4. Compute inputs hash
        let inputs_hash = self.compute_inputs_hash(&request);

        // 5. Create DecisionEvent
        let decision_event = self.create_decision_event(
            &request,
            &response,
            constraints,
            inputs_hash,
            start.elapsed().as_micros() as u64,
        );

        // 6. Persist to ruvector-service (non-blocking, best-effort)
        if let Some(ref client) = self.ruvector_client {
            // Persist DecisionEvent
            if let Err(e) = client.persist_decision(&decision_event).await {
                error!("Failed to persist DecisionEvent: {}", e);
                // Continue - persistence failure should not block response
            }

            // Persist updated state
            let persistence = CircuitBreakerPersistence {
                provider_id: request.provider_id.clone(),
                state: response.new_state,
                failure_count: response.new_failure_count,
                success_count: response.new_success_count,
                last_failure_time: response.new_last_failure_time,
                last_state_change: response
                    .state_transition
                    .as_ref()
                    .map(|t| t.timestamp)
                    .unwrap_or_else(Utc::now),
                updated_at: Utc::now(),
            };

            if let Err(e) = client.persist_state(&persistence).await {
                error!("Failed to persist circuit breaker state: {}", e);
                // Continue - persistence failure should not block response
            }
        }

        // 7. Emit telemetry (via tracing, picked up by observability layer)
        if let Some(ref transition) = response.state_transition {
            info!(
                provider_id = %request.provider_id,
                from_state = %transition.from,
                to_state = %transition.to,
                trigger = %transition.trigger,
                "Circuit state transition"
            );
        }

        info!(
            provider_id = %request.provider_id,
            decision = %response.decision,
            new_state = %response.new_state,
            duration_us = response.duration_us,
            "Circuit breaker decision complete"
        );

        Ok(response)
    }

    /// Build constraints applied during evaluation
    fn build_constraints(
        &self,
        request: &CircuitBreakerRequest,
        response: &CircuitBreakerResponse,
    ) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        // Failure threshold constraint
        constraints.push(Constraint {
            constraint_type: ConstraintType::Guard,
            name: "failure_threshold".to_string(),
            passed: response.new_failure_count < request.config.failure_threshold,
            details: Some(format!(
                "failures: {} / threshold: {}",
                response.new_failure_count, request.config.failure_threshold
            )),
            severity: if response.new_failure_count >= request.config.failure_threshold {
                Some(0.8)
            } else {
                None
            },
        });

        // Success threshold constraint (for half-open)
        if request.current_state == CircuitState::HalfOpen {
            constraints.push(Constraint {
                constraint_type: ConstraintType::Guard,
                name: "success_threshold".to_string(),
                passed: response.new_success_count >= request.config.success_threshold,
                details: Some(format!(
                    "successes: {} / threshold: {}",
                    response.new_success_count, request.config.success_threshold
                )),
                severity: None,
            });
        }

        // Timeout constraint
        if request.current_state == CircuitState::Open {
            let timeout_elapsed = request.last_failure_time.map(|lf| {
                (request.timestamp - lf).num_seconds() >= request.config.timeout_seconds as i64
            }).unwrap_or(false);

            constraints.push(Constraint {
                constraint_type: ConstraintType::Guard,
                name: "timeout_elapsed".to_string(),
                passed: timeout_elapsed,
                details: Some(format!(
                    "timeout: {} seconds",
                    request.config.timeout_seconds
                )),
                severity: None,
            });
        }

        constraints
    }

    /// Compute SHA-256 hash of inputs for deduplication
    fn compute_inputs_hash(&self, request: &CircuitBreakerRequest) -> String {
        let mut hasher = Sha256::new();

        // Hash deterministic fields only
        hasher.update(request.provider_id.as_bytes());
        hasher.update(format!("{:?}", request.current_state).as_bytes());
        hasher.update(request.failure_count.to_le_bytes());
        hasher.update(request.success_count.to_le_bytes());

        if let Some(ref failure) = request.failure_signal {
            hasher.update(failure.signal_id.as_bytes());
            hasher.update(failure.failure_type.as_bytes());
        }

        if let Some(ref success) = request.success_signal {
            hasher.update(success.signal_id.as_bytes());
        }

        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    /// Create DecisionEvent for persistence
    fn create_decision_event(
        &self,
        request: &CircuitBreakerRequest,
        response: &CircuitBreakerResponse,
        constraints: Vec<Constraint>,
        inputs_hash: String,
        duration_us: u64,
    ) -> DecisionEvent {
        let outputs = serde_json::json!({
            "decision": response.decision,
            "new_state": response.new_state,
            "new_failure_count": response.new_failure_count,
            "new_success_count": response.new_success_count,
            "retry_after_seconds": response.retry_after_seconds,
            "reason": response.reason,
            "state_transition": response.state_transition,
        });

        DecisionEvent::new(
            self.metadata.clone(),
            DecisionType::Block, // Using Block as closest match; consider adding CircuitBreakerDecision
            inputs_hash,
            outputs,
            response.confidence,
            constraints,
            request.execution_ref.clone(),
            duration_us,
        )
    }

    /// Health check
    pub async fn health_check(&self) -> bool {
        if let Some(ref client) = self.ruvector_client {
            client.health_check().await
        } else {
            true // Healthy if no persistence configured
        }
    }

    /// Get agent metadata
    pub fn metadata(&self) -> &AgentMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = CircuitBreakerAgent::new(None);
        assert_eq!(agent.metadata.agent_id, AGENT_ID);
        assert_eq!(agent.metadata.agent_type, AgentType::Protection);
    }

    #[tokio::test]
    async fn test_invoke_closed_circuit() {
        let agent = CircuitBreakerAgent::new(None);
        let request = CircuitBreakerRequest::check("openai", "exec-123");

        let response = agent.invoke(request).await.unwrap();

        assert_eq!(response.decision, CircuitBreakerDecision::Allow);
        assert_eq!(response.new_state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_invoke_with_failure() {
        let agent = CircuitBreakerAgent::new(None);
        let signal = FailureSignal::new("openai", "timeout");
        let request = CircuitBreakerRequest::record_failure("openai", "exec-123", signal)
            .with_state(CircuitState::Closed, 4, 0)
            .with_config(CircuitBreakerConfig {
                provider_id: "openai".to_string(),
                failure_threshold: 5,
                ..Default::default()
            });

        let response = agent.invoke(request).await.unwrap();

        assert_eq!(response.new_state, CircuitState::Open);
        assert_eq!(response.decision, CircuitBreakerDecision::Block);
    }

    #[test]
    fn test_compute_inputs_hash() {
        let agent = CircuitBreakerAgent::new(None);
        let request = CircuitBreakerRequest::check("openai", "exec-123");

        let hash1 = agent.compute_inputs_hash(&request);
        let hash2 = agent.compute_inputs_hash(&request);

        // Same inputs should produce same hash
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
    }
}
