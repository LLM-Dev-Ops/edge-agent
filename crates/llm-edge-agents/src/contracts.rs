//! Agent contracts and schemas
//!
//! This module defines the contracts that all LLM-Edge-Agent agents must follow.
//! These contracts are designed to be compatible with agentics-contracts and
//! define the standard interfaces for agent communication.
//!
//! # Contract Principles
//!
//! 1. All inputs and outputs MUST be validated against these schemas
//! 2. DecisionEvents MUST be emitted for every invocation
//! 3. Agents MUST be deterministic given the same inputs
//! 4. All contracts are versioned for compatibility

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Decision types for agent outcomes
///
/// Each agent emits exactly one decision type per invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Allow execution to proceed
    Allow,
    /// Block execution
    Block,
    /// Route to a specific provider/handler
    Route,
    /// Return cached result
    Cache,
    /// Failover to alternative provider
    Failover,
    /// Tool invocation validation decision
    ToolInvocationDecision,
    /// Execution guard decision (resource limits, token limits, time limits)
    ExecutionGuardDecision,
    /// Cache strategy decision (cache/bypass/write/invalidate)
    CacheStrategyDecision,
}

/// Constraint types that can be applied to decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    /// Policy-based constraint
    Policy,
    /// Routing constraint
    Routing,
    /// Guard/validation constraint
    Guard,
    /// Cache-related constraint
    Cache,
    /// Rate limiting constraint
    RateLimit,
    /// Security constraint
    Security,
    /// Schema validation constraint
    SchemaValidation,
    /// Resource constraint
    Resource,
}

/// A constraint applied during decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Type of constraint
    pub constraint_type: ConstraintType,

    /// Human-readable name
    pub name: String,

    /// Whether this constraint passed
    pub passed: bool,

    /// Details about the constraint evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Severity if constraint failed (0.0 = info, 1.0 = critical)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<f64>,
}

/// Execution context for agent invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution reference (trace ID)
    pub execution_ref: String,

    /// Request ID for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Tenant/customer ID for multi-tenancy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// User ID who initiated the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Session ID for conversational context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Source system/service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Custom tags for attribution
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context with a generated execution_ref
    pub fn new() -> Self {
        Self {
            execution_ref: Uuid::new_v4().to_string(),
            request_id: None,
            tenant_id: None,
            user_id: None,
            session_id: None,
            source: None,
            tags: HashMap::new(),
        }
    }

    /// Create from an existing execution reference
    pub fn with_execution_ref(execution_ref: impl Into<String>) -> Self {
        Self {
            execution_ref: execution_ref.into(),
            ..Self::new()
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent metadata for DecisionEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Unique agent identifier
    pub agent_id: String,

    /// Agent version
    pub agent_version: String,

    /// Agent type/classification
    pub agent_type: AgentType,
}

/// Agent type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Routing agent - directs requests to providers
    Routing,
    /// Execution control agent - allow/block decisions
    ExecutionControl,
    /// Protection/guard agent - security enforcement
    Protection,
    /// Cache agent - caching decisions
    Cache,
}

/// DecisionEvent - emitted by every agent invocation
///
/// This is the canonical event format for ruvector-service persistence.
/// Every agent MUST emit exactly one DecisionEvent per invocation.
///
/// # Schema Requirements
///
/// - `agent_id`: Unique identifier for the agent type
/// - `agent_version`: Semantic version of the agent
/// - `decision_type`: The type of decision made
/// - `inputs_hash`: SHA-256 hash of the input for deduplication
/// - `outputs`: Structured output of the decision
/// - `confidence`: Decision confidence (0.0 to 1.0)
/// - `constraints_applied`: List of constraints evaluated
/// - `execution_ref`: Trace ID for distributed tracing
/// - `timestamp`: UTC timestamp of the decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Unique event ID
    pub event_id: String,

    /// Agent metadata
    pub agent: AgentMetadata,

    /// Decision type
    pub decision_type: DecisionType,

    /// SHA-256 hash of the input payload (for deduplication)
    pub inputs_hash: String,

    /// Structured output of the decision
    pub outputs: serde_json::Value,

    /// Decision confidence (0.0 = uncertain, 1.0 = certain)
    pub confidence: f64,

    /// Constraints that were applied/evaluated
    pub constraints_applied: Vec<Constraint>,

    /// Execution reference for distributed tracing
    pub execution_ref: String,

    /// UTC timestamp of the decision
    pub timestamp: DateTime<Utc>,

    /// Processing duration in microseconds
    pub duration_us: u64,

    /// Additional metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DecisionEvent {
    /// Create a new DecisionEvent
    pub fn new(
        agent: AgentMetadata,
        decision_type: DecisionType,
        inputs_hash: String,
        outputs: serde_json::Value,
        confidence: f64,
        constraints_applied: Vec<Constraint>,
        execution_ref: String,
        duration_us: u64,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            agent,
            decision_type,
            inputs_hash,
            outputs,
            confidence,
            constraints_applied,
            execution_ref,
            timestamp: Utc::now(),
            duration_us,
            metadata: HashMap::new(),
        }
    }
}

/// Constraint for tool invocation validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationConstraint {
    /// Maximum execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_execution_time_ms: Option<u64>,

    /// Maximum memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,

    /// Allowed tool names (whitelist)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,

    /// Blocked tool names (blacklist)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_tools: Vec<String>,

    /// Maximum number of concurrent invocations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,

    /// Require specific capabilities
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,

    /// Custom constraints
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for InvocationConstraint {
    fn default() -> Self {
        Self {
            max_execution_time_ms: Some(30_000), // 30 seconds default
            max_memory_bytes: None,
            allowed_tools: Vec::new(),
            blocked_tools: Vec::new(),
            max_concurrent: None,
            required_capabilities: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_new() {
        let ctx = ExecutionContext::new();
        assert!(!ctx.execution_ref.is_empty());
    }

    #[test]
    fn test_decision_event_serialization() {
        let agent = AgentMetadata {
            agent_id: "tool-invocation".to_string(),
            agent_version: "0.1.0".to_string(),
            agent_type: AgentType::ExecutionControl,
        };

        let event = DecisionEvent::new(
            agent,
            DecisionType::Allow,
            "abc123".to_string(),
            serde_json::json!({"allowed": true}),
            1.0,
            vec![],
            "trace-123".to_string(),
            100,
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool-invocation"));
        assert!(json.contains("allow"));
    }

    #[test]
    fn test_constraint_types() {
        let constraint = Constraint {
            constraint_type: ConstraintType::SchemaValidation,
            name: "tool-schema".to_string(),
            passed: true,
            details: Some("Schema validated successfully".to_string()),
            severity: None,
        };

        assert!(constraint.passed);
        assert_eq!(constraint.constraint_type, ConstraintType::SchemaValidation);
    }
}
