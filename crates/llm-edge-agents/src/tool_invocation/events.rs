//! DecisionEvent emission for Tool Invocation Agent
//!
//! This module handles the emission of DecisionEvents to ruvector-service.
//! All persistence occurs via ruvector-service - the agent NEVER connects
//! directly to the database.
//!
//! # Persistence Rules
//!
//! Data that IS persisted:
//! - Agent ID and version
//! - Decision type and outcome
//! - Inputs hash (SHA-256 for deduplication)
//! - Constraint results
//! - Execution reference (trace ID)
//! - Timestamp and duration
//!
//! Data that is NOT persisted:
//! - Raw arguments (only hash)
//! - Full request payload
//! - Sensitive context data
//! - PII or credentials

use sha2::{Digest, Sha256};

use crate::contracts::{AgentType, Constraint, DecisionEvent, DecisionType, AgentMetadata};
use crate::error::{AgentError, AgentResult};
use crate::AGENT_VERSION;

use super::types::{ToolInvocationDecision, ToolInvocationRequest, ValidationResult};
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

/// Create a DecisionEvent from a tool invocation decision
///
/// This function creates the canonical event format for persistence.
pub fn emit_decision_event(
    request: &ToolInvocationRequest,
    decision: &ToolInvocationDecision,
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

    // Build outputs
    let outputs = serde_json::json!({
        "allowed": decision.allowed,
        "reason": decision.reason,
        "tool_name": request.tool.name,
        "validation": {
            "valid": validation.valid,
            "schema_valid": validation.schema_valid,
            "security_valid": validation.security_valid,
            "error_count": validation.errors.len(),
            "warning_count": validation.warnings.len(),
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
            agent_type: AgentType::ExecutionControl,
        },
        DecisionType::ToolInvocationDecision,
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
/// - Tool name
/// - Arguments (canonicalized)
/// - Constraint identifiers
fn hash_inputs(request: &ToolInvocationRequest) -> String {
    let mut hasher = Sha256::new();

    // Hash tool name
    hasher.update(request.tool.name.as_bytes());

    // Hash arguments (canonicalized JSON)
    if let Ok(canonical) = serde_json::to_string(&request.arguments) {
        hasher.update(canonical.as_bytes());
    }

    // Hash constraint identifiers
    if let Some(ref constraints) = request.constraints {
        if !constraints.allowed_tools.is_empty() {
            for tool in &constraints.allowed_tools {
                hasher.update(tool.as_bytes());
            }
        }
        if !constraints.blocked_tools.is_empty() {
            for tool in &constraints.blocked_tools {
                hasher.update(tool.as_bytes());
            }
        }
    }

    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::tool_invocation::types::{ToolDefinition, ToolInvocationRequest};

    fn create_test_request() -> ToolInvocationRequest {
        ToolInvocationRequest {
            tool: ToolDefinition {
                name: "test_tool".to_string(),
                description: Some("Test tool".to_string()),
                parameters: vec![],
                capabilities: vec![],
            },
            arguments: serde_json::json!({"key": "value"}),
            context: Some(ExecutionContext::new()),
            constraints: None,
        }
    }

    fn create_test_decision() -> ToolInvocationDecision {
        ToolInvocationDecision {
            allowed: true,
            reason: "Test decision".to_string(),
            confidence: 1.0,
            constraints_evaluated: vec![],
            suggested_modifications: None,
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

        request1.arguments = serde_json::json!({"a": 1});
        request2.arguments = serde_json::json!({"a": 2});

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
        assert_eq!(event.decision_type, DecisionType::ToolInvocationDecision);
        assert!(!event.inputs_hash.is_empty());
        assert!(event.outputs.get("allowed").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_ruvector_config_defaults() {
        let config = RuvectorConfig::default();
        assert!(config.base_url.contains("localhost") || !config.base_url.is_empty());
        assert!(config.timeout_ms > 0);
    }
}
