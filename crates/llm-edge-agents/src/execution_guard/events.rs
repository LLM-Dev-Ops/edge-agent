//! DecisionEvent emission for Execution Guard Agent
//!
//! This module handles the emission of DecisionEvents to ruvector-service.
//! All persistence occurs via ruvector-service - the agent NEVER connects
//! directly to the database.
//!
//! # Persistence Rules
//!
//! Data that IS persisted:
//! - Agent ID and version
//! - Decision type (allow/block)
//! - Inputs hash (SHA-256 for deduplication)
//! - Constraint results (which limits were checked)
//! - Violation details (what failed)
//! - Execution reference (trace ID)
//! - Timestamp and duration
//!
//! Data that is NOT persisted:
//! - Raw request payload
//! - Full execution envelope details
//! - Sensitive context data
//! - PII or credentials
//! - Provider API keys

use sha2::{Digest, Sha256};

use crate::contracts::{AgentMetadata, AgentType, Constraint, DecisionEvent, DecisionType};
use crate::error::{AgentError, AgentResult};
use crate::AGENT_VERSION;

use super::types::{ExecutionGuardDecision, ExecutionGuardRequest, ValidationResult};
use super::AGENT_ID;

/// Configuration for ruvector-service client
#[derive(Debug, Clone)]
pub struct RuvectorConfig {
    /// Base URL for ruvector-service
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Request timeout in milliseconds
    pub timeout_ms: u64,

    /// Enable async (non-blocking) writes
    pub async_writes: bool,
}

impl Default for RuvectorConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("RUVECTOR_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").ok(),
            timeout_ms: 5000,
            async_writes: true,
        }
    }
}

/// Client for ruvector-service
#[derive(Debug, Clone)]
pub struct RuvectorClient {
    config: RuvectorConfig,
    http_client: reqwest::Client,
}

impl RuvectorClient {
    /// Create a new ruvector-service client
    pub fn new(config: RuvectorConfig) -> AgentResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AgentError::HttpClient(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create with default configuration
    pub fn from_env() -> AgentResult<Self> {
        Self::new(RuvectorConfig::default())
    }

    /// Emit a DecisionEvent to ruvector-service
    ///
    /// This is an async, non-blocking write. The agent does NOT wait for
    /// confirmation - fire and forget semantics.
    pub async fn emit(&self, event: DecisionEvent) -> AgentResult<()> {
        let url = format!("{}/api/v1/events", self.config.base_url);

        let mut request = self.http_client.post(&url).json(&event);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        // Add tracing headers
        request = request
            .header("X-Execution-Ref", &event.execution_ref)
            .header("X-Agent-ID", AGENT_ID)
            .header("X-Agent-Version", AGENT_VERSION);

        let response = request
            .send()
            .await
            .map_err(|e| AgentError::HttpClient(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(AgentError::EventEmission(format!(
                "ruvector-service returned {}: {}",
                status, body
            )));
        }

        tracing::debug!(
            execution_ref = %event.execution_ref,
            event_id = %event.event_id,
            "DecisionEvent emitted successfully"
        );

        Ok(())
    }
}

/// Create a DecisionEvent from an execution guard decision
///
/// This function creates the canonical event format for persistence.
/// It ensures that only appropriate data is included in the event.
pub fn emit_decision_event(
    request: &ExecutionGuardRequest,
    decision: &ExecutionGuardDecision,
    validation: &ValidationResult,
    duration_us: u64,
) -> DecisionEvent {
    // Calculate inputs hash (SHA-256)
    let inputs_hash = hash_inputs(request);

    // Get execution reference
    let execution_ref = request
        .context
        .as_ref()
        .map(|c| c.execution_ref.clone())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Build outputs - only include decision-relevant data, not raw inputs
    let outputs = serde_json::json!({
        "allowed": decision.allowed,
        "reason": decision.reason,
        "model": request.envelope.model,
        "request_id": request.envelope.request_id,
        "violations_count": decision.violations.len(),
        "validation": {
            "valid": validation.valid,
            "tokens_valid": validation.tokens_valid,
            "time_valid": validation.time_valid,
            "memory_valid": validation.memory_valid,
            "cost_valid": validation.cost_valid,
            "rate_valid": validation.rate_valid,
            "concurrency_valid": validation.concurrency_valid,
            "model_valid": validation.model_valid,
            "error_count": validation.errors.len(),
            "warning_count": validation.warnings.len(),
        },
        "resource_usage": {
            "input_tokens": request.envelope.input_tokens,
            "max_output_tokens": request.envelope.max_output_tokens,
            "current_concurrency": request.envelope.current_concurrency,
        }
    });

    // Convert decision constraints to contract constraints
    let constraints_applied: Vec<Constraint> = decision
        .constraints_evaluated
        .iter()
        .cloned()
        .collect();

    DecisionEvent::new(
        AgentMetadata {
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            agent_type: AgentType::Protection,
        },
        DecisionType::ExecutionGuardDecision,
        inputs_hash,
        outputs,
        decision.confidence,
        constraints_applied,
        execution_ref,
        duration_us,
    )
}

/// Calculate SHA-256 hash of request inputs
///
/// Only hashes the structural parts needed for deduplication:
/// - Request ID
/// - Model
/// - Token counts
/// Does NOT include sensitive data.
fn hash_inputs(request: &ExecutionGuardRequest) -> String {
    let mut hasher = Sha256::new();

    // Hash request identifier
    hasher.update(request.envelope.request_id.as_bytes());

    // Hash model
    hasher.update(request.envelope.model.as_bytes());

    // Hash token counts
    hasher.update(request.envelope.input_tokens.to_le_bytes());
    hasher.update(request.envelope.max_output_tokens.to_le_bytes());

    // Hash provider if present
    if let Some(ref provider) = request.envelope.provider {
        hasher.update(provider.as_bytes());
    }

    // Hash tenant/user for isolation
    if let Some(ref ctx) = request.context {
        if let Some(ref tenant) = ctx.tenant_id {
            hasher.update(tenant.as_bytes());
        }
        if let Some(ref user) = ctx.user_id {
            hasher.update(user.as_bytes());
        }
    }

    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::execution_guard::types::{ExecutionEnvelope, ExecutionGuardRequest};
    use std::collections::HashMap;

    fn create_test_envelope() -> ExecutionEnvelope {
        ExecutionEnvelope {
            request_id: "test-123".to_string(),
            model: "gpt-4".to_string(),
            provider: Some("openai".to_string()),
            input_tokens: 1000,
            max_output_tokens: 2000,
            estimated_duration_ms: Some(5000),
            estimated_memory_bytes: None,
            estimated_cost_micros: Some(50000),
            current_concurrency: 2,
            metadata: HashMap::new(),
        }
    }

    fn create_test_request() -> ExecutionGuardRequest {
        ExecutionGuardRequest {
            envelope: create_test_envelope(),
            context: Some(ExecutionContext::new()),
            limits: None,
            current_rate: Some(10),
        }
    }

    fn create_test_decision() -> ExecutionGuardDecision {
        ExecutionGuardDecision {
            allowed: true,
            reason: "All constraints passed".to_string(),
            confidence: 1.0,
            constraints_evaluated: vec![],
            violations: vec![],
            suggested_adjustments: None,
            execution_hints: None,
        }
    }

    #[test]
    fn test_hash_inputs() {
        let request = create_test_request();
        let hash = hash_inputs(&request);

        // Hash should be consistent
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
        assert_eq!(hash, hash_inputs(&request));
    }

    #[test]
    fn test_hash_inputs_different() {
        let mut request1 = create_test_request();
        let mut request2 = create_test_request();

        request1.envelope.input_tokens = 1000;
        request2.envelope.input_tokens = 2000;

        let hash1 = hash_inputs(&request1);
        let hash2 = hash_inputs(&request2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_emit_decision_event() {
        let request = create_test_request();
        let decision = create_test_decision();
        let validation = ValidationResult::default();

        let event = emit_decision_event(&request, &decision, &validation, 100);

        assert_eq!(event.agent.agent_id, AGENT_ID);
        assert_eq!(event.agent.agent_type, AgentType::Protection);
        assert_eq!(event.decision_type, DecisionType::ExecutionGuardDecision);
        assert!(!event.inputs_hash.is_empty());
        assert!(event.outputs.get("allowed").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_event_does_not_contain_sensitive_data() {
        let mut request = create_test_request();
        // Add some "sensitive" metadata
        request.envelope.metadata.insert(
            "api_key".to_string(),
            serde_json::json!("sk-secret-key"),
        );

        let decision = create_test_decision();
        let validation = ValidationResult::default();

        let event = emit_decision_event(&request, &decision, &validation, 100);

        // Outputs should not contain the api_key
        let outputs_str = serde_json::to_string(&event.outputs).unwrap();
        assert!(!outputs_str.contains("sk-secret-key"));
        assert!(!outputs_str.contains("api_key"));
    }

    #[test]
    fn test_ruvector_config_defaults() {
        let config = RuvectorConfig::default();
        assert!(!config.base_url.is_empty());
        assert!(config.timeout_ms > 0);
        assert!(config.async_writes);
    }
}
