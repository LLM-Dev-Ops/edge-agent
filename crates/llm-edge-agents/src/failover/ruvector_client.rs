//! RuVector-service client for DecisionEvent persistence
//!
//! This client communicates with ruvector-service to persist DecisionEvents.
//! LLM-Edge-Agent does NOT own persistence - all execution decisions are
//! persisted via ruvector-service which is backed by Google SQL (Postgres).
//!
//! # Important
//!
//! - LLM-Edge-Agent NEVER connects directly to Google SQL
//! - LLM-Edge-Agent NEVER executes SQL
//! - All persistence occurs via ruvector-service client calls only

use crate::{AgentError, AgentResult, DecisionEvent};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// HTTP client for ruvector-service communication
#[derive(Debug, Clone)]
pub struct RuVectorClient {
    /// HTTP client
    client: Client,

    /// ruvector-service base URL
    base_url: String,

    /// Request timeout
    timeout: Duration,
}

impl RuVectorClient {
    /// Create a new RuVector client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(10),
        }
    }

    /// Create with custom configuration
    pub fn with_config(base_url: &str, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout,
        }
    }

    /// Emit a DecisionEvent to ruvector-service
    ///
    /// Endpoint: POST /api/v1/agents/{agent_id}/events
    ///
    /// # Persistence Requirements (per Infrastructure Constitution)
    ///
    /// The DecisionEvent MUST include:
    /// - agent_id
    /// - agent_version
    /// - decision_type
    /// - inputs_hash
    /// - outputs
    /// - confidence
    /// - constraints_applied
    /// - execution_ref
    /// - timestamp (UTC)
    ///
    /// # What MUST NOT be persisted
    ///
    /// - Raw request/response payloads (PII risk)
    /// - Provider API keys or credentials
    /// - User session tokens
    /// - Internal service addresses
    #[instrument(
        skip(self, event),
        fields(
            agent_id = %event.agent.agent_id,
            event_id = %event.event_id,
            execution_ref = %event.execution_ref
        )
    )]
    pub async fn emit_event(&self, event: DecisionEvent) -> AgentResult<()> {
        let url = format!(
            "{}/api/v1/agents/{}/events",
            self.base_url, event.agent.agent_id
        );

        debug!(url = %url, "Emitting DecisionEvent to ruvector-service");

        let response = self
            .client
            .post(&url)
            .json(&event)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| AgentError::HttpClient(format!("Request failed: {}", e)))?;

        if response.status().is_success() {
            debug!(
                status = %response.status(),
                "DecisionEvent emitted successfully"
            );
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            warn!(
                status = %status,
                body = %body,
                "Failed to emit DecisionEvent"
            );

            Err(AgentError::EventEmission(format!(
                "ruvector-service returned {}: {}",
                status, body
            )))
        }
    }

    /// Health check for ruvector-service
    ///
    /// Endpoint: GET /health
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> AgentResult<bool> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| AgentError::HttpClient(format!("Health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Event persistence data that is allowed to be stored
#[derive(Debug, Clone, serde::Serialize)]
pub struct PersistableEventData {
    /// Event ID
    pub event_id: String,

    /// Agent ID
    pub agent_id: String,

    /// Agent version
    pub agent_version: String,

    /// Decision type
    pub decision_type: String,

    /// Inputs hash (SHA-256)
    pub inputs_hash: String,

    /// Structured outputs (sanitized)
    pub outputs: serde_json::Value,

    /// Confidence score
    pub confidence: f64,

    /// Constraints applied
    pub constraints_applied: serde_json::Value,

    /// Execution reference
    pub execution_ref: String,

    /// Timestamp (UTC)
    pub timestamp: String,

    /// Duration in microseconds
    pub duration_us: u64,
}

impl From<&DecisionEvent> for PersistableEventData {
    fn from(event: &DecisionEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            agent_id: event.agent.agent_id.clone(),
            agent_version: event.agent.agent_version.clone(),
            decision_type: format!("{:?}", event.decision_type),
            inputs_hash: event.inputs_hash.clone(),
            outputs: event.outputs.clone(),
            confidence: event.confidence,
            constraints_applied: serde_json::to_value(&event.constraints_applied)
                .unwrap_or(serde_json::Value::Null),
            execution_ref: event.execution_ref.clone(),
            timestamp: event.timestamp.to_rfc3339(),
            duration_us: event.duration_us,
        }
    }
}

/// Data that MUST NOT be persisted (for documentation)
#[allow(dead_code)]
pub mod non_persistable {
    /// Raw request payloads (may contain PII)
    pub const RAW_REQUEST_PAYLOADS: &str = "MUST NOT persist raw request payloads";

    /// Raw response payloads (may contain PII)
    pub const RAW_RESPONSE_PAYLOADS: &str = "MUST NOT persist raw response payloads";

    /// Provider API keys
    pub const API_KEYS: &str = "MUST NOT persist API keys";

    /// User session tokens
    pub const SESSION_TOKENS: &str = "MUST NOT persist session tokens";

    /// Provider credentials
    pub const CREDENTIALS: &str = "MUST NOT persist credentials";

    /// Internal service addresses
    pub const INTERNAL_ADDRESSES: &str = "MUST NOT persist internal service addresses";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentMetadata, AgentType, DecisionType};

    #[test]
    fn test_client_creation() {
        let client = RuVectorClient::new("http://localhost:8080");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_client_url_normalization() {
        let client = RuVectorClient::new("http://localhost:8080/");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_persistable_event_data_conversion() {
        let agent = AgentMetadata {
            agent_id: "failover-agent".to_string(),
            agent_version: "0.1.0".to_string(),
            agent_type: AgentType::Routing,
        };

        let event = DecisionEvent::new(
            agent,
            DecisionType::Failover,
            "abc123".to_string(),
            serde_json::json!({"outcome": "use_primary"}),
            1.0,
            vec![],
            "trace-123".to_string(),
            100,
        );

        let persistable: PersistableEventData = (&event).into();

        assert_eq!(persistable.agent_id, "failover-agent");
        assert_eq!(persistable.inputs_hash, "abc123");
        assert_eq!(persistable.confidence, 1.0);
    }
}
