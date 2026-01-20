//! DecisionEvent schema for persisting agent decisions to ruvector-service
//!
//! Every LLM-Edge-Agent agent MUST emit exactly ONE DecisionEvent per invocation.
//! This schema defines the canonical structure for all decision persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Decision types supported by LLM-Edge-Agent agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Routing decision (select provider, model, endpoint)
    Route,
    /// Block execution (guard triggered)
    Block,
    /// Allow execution (guard passed)
    Allow,
    /// Cache hit (return cached response)
    Cache,
    /// Failover (switch to backup provider)
    Failover,
    /// Circuit breaker specific decision
    CircuitBreakerDecision,
    /// Cache strategy decision (cache/bypass/write/invalidate)
    CacheStrategyDecision,
}

/// Outcome of a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOutcome {
    /// Decision was successful
    Success,
    /// Decision failed
    Failure,
    /// Decision was skipped (not applicable)
    Skipped,
    /// Decision timed out
    Timeout,
}

/// Constraint applied during decision making
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AppliedConstraint {
    /// Constraint type (policy, routing, cache, guard)
    pub constraint_type: String,
    /// Constraint identifier
    pub constraint_id: String,
    /// Whether the constraint was satisfied
    pub satisfied: bool,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// DecisionEvent schema - MUST be emitted by every agent per invocation
///
/// This is the canonical schema for persisting decisions to ruvector-service.
/// Agents MUST NOT persist state locally - all persistence flows through this event.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DecisionEvent {
    /// Unique event identifier
    pub event_id: Uuid,

    /// Agent identifier (e.g., "circuit_breaker_agent")
    #[validate(length(min = 1, max = 64))]
    pub agent_id: String,

    /// Agent version (semver format)
    #[validate(length(min = 1, max = 32))]
    pub agent_version: String,

    /// Type of decision made
    pub decision_type: DecisionType,

    /// Hash of input parameters (for deduplication/caching)
    #[validate(length(min = 1, max = 128))]
    pub inputs_hash: String,

    /// Decision outputs (agent-specific structure, serialized as JSON)
    pub outputs: serde_json::Value,

    /// Confidence score (0.0 to 1.0)
    /// - 1.0 = deterministic decision (threshold-based)
    /// - <1.0 = heuristic/probabilistic decision
    #[validate(range(min = 0.0, max = 1.0))]
    pub confidence: f64,

    /// Outcome of the decision
    pub outcome: DecisionOutcome,

    /// Constraints applied during decision making
    pub constraints_applied: Vec<AppliedConstraint>,

    /// Reference to the execution context
    pub execution_ref: String,

    /// Timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Duration of decision in microseconds
    pub duration_us: u64,

    /// Optional correlation ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Optional metadata (agent-specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl DecisionEvent {
    /// Create a new DecisionEvent builder
    pub fn builder() -> DecisionEventBuilder {
        DecisionEventBuilder::default()
    }
}

/// Builder for DecisionEvent
#[derive(Default)]
pub struct DecisionEventBuilder {
    agent_id: Option<String>,
    agent_version: Option<String>,
    decision_type: Option<DecisionType>,
    inputs_hash: Option<String>,
    outputs: Option<serde_json::Value>,
    confidence: Option<f64>,
    outcome: Option<DecisionOutcome>,
    constraints_applied: Vec<AppliedConstraint>,
    execution_ref: Option<String>,
    duration_us: Option<u64>,
    correlation_id: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl DecisionEventBuilder {
    pub fn agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    pub fn agent_version(mut self, version: impl Into<String>) -> Self {
        self.agent_version = Some(version.into());
        self
    }

    pub fn decision_type(mut self, dt: DecisionType) -> Self {
        self.decision_type = Some(dt);
        self
    }

    pub fn inputs_hash(mut self, hash: impl Into<String>) -> Self {
        self.inputs_hash = Some(hash.into());
        self
    }

    pub fn outputs(mut self, outputs: serde_json::Value) -> Self {
        self.outputs = Some(outputs);
        self
    }

    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn outcome(mut self, outcome: DecisionOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn add_constraint(mut self, constraint: AppliedConstraint) -> Self {
        self.constraints_applied.push(constraint);
        self
    }

    pub fn execution_ref(mut self, exec_ref: impl Into<String>) -> Self {
        self.execution_ref = Some(exec_ref.into());
        self
    }

    pub fn duration_us(mut self, duration: u64) -> Self {
        self.duration_us = Some(duration);
        self
    }

    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Result<DecisionEvent, ContractError> {
        Ok(DecisionEvent {
            event_id: Uuid::new_v4(),
            agent_id: self.agent_id.ok_or(ContractError::MissingField("agent_id"))?,
            agent_version: self.agent_version.ok_or(ContractError::MissingField("agent_version"))?,
            decision_type: self.decision_type.ok_or(ContractError::MissingField("decision_type"))?,
            inputs_hash: self.inputs_hash.ok_or(ContractError::MissingField("inputs_hash"))?,
            outputs: self.outputs.unwrap_or(serde_json::Value::Null),
            confidence: self.confidence.unwrap_or(1.0),
            outcome: self.outcome.unwrap_or(DecisionOutcome::Success),
            constraints_applied: self.constraints_applied,
            execution_ref: self.execution_ref.ok_or(ContractError::MissingField("execution_ref"))?,
            timestamp: Utc::now(),
            duration_us: self.duration_us.unwrap_or(0),
            correlation_id: self.correlation_id,
            metadata: self.metadata,
        })
    }
}

/// Contract validation errors
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid field value: {0}")]
    InvalidValue(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_event_builder() {
        let event = DecisionEvent::builder()
            .agent_id("circuit_breaker_agent")
            .agent_version("0.1.0")
            .decision_type(DecisionType::Block)
            .inputs_hash("sha256:abc123")
            .execution_ref("exec-001")
            .confidence(1.0)
            .outcome(DecisionOutcome::Success)
            .build()
            .unwrap();

        assert_eq!(event.agent_id, "circuit_breaker_agent");
        assert_eq!(event.decision_type, DecisionType::Block);
        assert_eq!(event.confidence, 1.0);
    }

    #[test]
    fn test_decision_event_serialization() {
        let event = DecisionEvent::builder()
            .agent_id("test_agent")
            .agent_version("0.1.0")
            .decision_type(DecisionType::Allow)
            .inputs_hash("hash123")
            .execution_ref("exec-002")
            .build()
            .unwrap();

        let json = serde_json::to_string(&event).unwrap();
        let parsed: DecisionEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.agent_id, event.agent_id);
    }
}
