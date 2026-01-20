//! Failover Agent contracts and schemas
//!
//! These schemas are designed to be compatible with agentics-contracts.
//! All inputs and outputs MUST be validated against these schemas.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Provider health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealth {
    /// Provider is healthy and accepting requests
    Healthy,
    /// Provider is degraded but operational
    Degraded,
    /// Provider is unhealthy
    Unhealthy,
    /// Provider health is unknown
    Unknown,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Circuit breaker state for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Circuit is closed, normal operation
    Closed,
    /// Circuit is open, fail fast
    Open,
    /// Circuit is half-open, testing recovery
    HalfOpen,
}

impl Default for CircuitState {
    fn default() -> Self {
        Self::Closed
    }
}

/// Target provider definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TargetProvider {
    /// Unique provider identifier
    #[validate(length(min = 1, max = 64))]
    pub provider_id: String,

    /// Provider name (e.g., "openai", "anthropic")
    #[validate(length(min = 1, max = 64))]
    pub provider_name: String,

    /// Model to use for this provider
    #[validate(length(min = 1, max = 128))]
    pub model: String,

    /// Provider endpoint URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Provider priority (lower = higher priority)
    #[validate(range(min = 0, max = 100))]
    #[serde(default)]
    pub priority: u32,

    /// Current circuit breaker state
    #[serde(default)]
    pub circuit_state: CircuitState,

    /// Current health status
    #[serde(default)]
    pub health: ProviderHealth,

    /// Provider-specific metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TargetProvider {
    /// Create a new target provider
    pub fn new(provider_id: impl Into<String>, provider_name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_name: provider_name.into(),
            model: model.into(),
            endpoint: None,
            priority: 0,
            circuit_state: CircuitState::Closed,
            health: ProviderHealth::Unknown,
            metadata: HashMap::new(),
        }
    }

    /// Check if provider is available for routing
    pub fn is_available(&self) -> bool {
        self.circuit_state != CircuitState::Open && self.health != ProviderHealth::Unhealthy
    }
}

/// Failover policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FailoverPolicy {
    /// Policy identifier
    #[validate(length(min = 1, max = 64))]
    pub policy_id: String,

    /// Policy name
    #[validate(length(min = 1, max = 128))]
    pub policy_name: String,

    /// Enable automatic failover
    #[serde(default = "default_true")]
    pub auto_failover_enabled: bool,

    /// Maximum number of failover attempts
    #[validate(range(min = 0, max = 10))]
    #[serde(default = "default_max_failover_attempts")]
    pub max_failover_attempts: u32,

    /// Failover on degraded health
    #[serde(default)]
    pub failover_on_degraded: bool,

    /// Require explicit alternate list
    #[serde(default)]
    pub require_explicit_alternates: bool,

    /// Allowed provider types for failover
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_providers: Vec<String>,

    /// Blocked provider types for failover
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_providers: Vec<String>,

    /// Policy metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

fn default_max_failover_attempts() -> u32 {
    3
}

impl Default for FailoverPolicy {
    fn default() -> Self {
        Self {
            policy_id: "default".to_string(),
            policy_name: "Default Failover Policy".to_string(),
            auto_failover_enabled: true,
            max_failover_attempts: 3,
            failover_on_degraded: false,
            require_explicit_alternates: false,
            allowed_providers: Vec::new(),
            blocked_providers: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Failover decision outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverOutcome {
    /// Use the primary target
    UsePrimary,
    /// Use an alternate target
    UseAlternate,
    /// No viable target available
    NoViableTarget,
}

/// Failover decision with details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverDecision {
    /// Decision outcome
    pub outcome: FailoverOutcome,

    /// Selected target provider (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<TargetProvider>,

    /// Human-readable reason for the decision
    pub reason: String,

    /// Decision confidence (0.0 to 1.0)
    pub confidence: f64,

    /// Failover attempt number (0 = primary)
    pub attempt_number: u32,

    /// Evaluated targets with their availability status
    #[serde(default)]
    pub evaluated_targets: Vec<EvaluatedTarget>,
}

/// Target evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedTarget {
    /// Provider identifier
    pub provider_id: String,

    /// Whether the target was selected
    pub selected: bool,

    /// Reason for selection/rejection
    pub reason: String,

    /// Target availability score (0.0 to 1.0)
    pub availability_score: f64,
}

/// Failover request input schema
///
/// This schema is validated against agentics-contracts.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FailoverRequest {
    /// Execution context for tracing
    pub execution_context: crate::ExecutionContext,

    /// Primary target provider
    #[validate(nested)]
    pub primary_target: TargetProvider,

    /// Alternate target providers (ordered by preference)
    #[validate(nested)]
    pub alternate_targets: Vec<TargetProvider>,

    /// Failover policy to apply
    #[validate(nested)]
    #[serde(default)]
    pub policy: FailoverPolicy,

    /// Current circuit breaker states by provider ID
    #[serde(default)]
    pub circuit_states: HashMap<String, CircuitState>,

    /// Current health status by provider ID
    #[serde(default)]
    pub health_status: HashMap<String, ProviderHealth>,

    /// Current failover attempt number (0 = initial routing)
    #[serde(default)]
    pub current_attempt: u32,

    /// Request metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl FailoverRequest {
    /// Create a new failover request
    pub fn new(
        execution_context: crate::ExecutionContext,
        primary_target: TargetProvider,
        alternate_targets: Vec<TargetProvider>,
    ) -> Self {
        Self {
            execution_context,
            primary_target,
            alternate_targets,
            policy: FailoverPolicy::default(),
            circuit_states: HashMap::new(),
            health_status: HashMap::new(),
            current_attempt: 0,
            metadata: HashMap::new(),
        }
    }

    /// Update target with circuit state and health status
    pub fn with_provider_status(
        mut self,
        provider_id: &str,
        circuit_state: CircuitState,
        health: ProviderHealth,
    ) -> Self {
        self.circuit_states.insert(provider_id.to_string(), circuit_state);
        self.health_status.insert(provider_id.to_string(), health);
        self
    }
}

/// Failover response output schema
///
/// This schema is returned by the Failover Agent and persisted to ruvector-service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverResponse {
    /// Failover decision
    pub decision: FailoverDecision,

    /// Constraints that were applied
    pub constraints_applied: Vec<crate::Constraint>,

    /// Whether the decision was successful
    pub success: bool,

    /// Error message if decision failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Processing duration in microseconds
    pub duration_us: u64,

    /// Response metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl FailoverResponse {
    /// Create a successful failover response
    pub fn success(
        decision: FailoverDecision,
        constraints_applied: Vec<crate::Constraint>,
        duration_us: u64,
    ) -> Self {
        Self {
            decision,
            constraints_applied,
            success: true,
            error: None,
            duration_us,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed failover response
    pub fn failure(error: impl Into<String>, duration_us: u64) -> Self {
        Self {
            decision: FailoverDecision {
                outcome: FailoverOutcome::NoViableTarget,
                selected_target: None,
                reason: "Failover decision failed".to_string(),
                confidence: 0.0,
                attempt_number: 0,
                evaluated_targets: Vec::new(),
            },
            constraints_applied: Vec::new(),
            success: false,
            error: Some(error.into()),
            duration_us,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionContext;

    #[test]
    fn test_target_provider_availability() {
        let mut provider = TargetProvider::new("openai-1", "openai", "gpt-4");
        assert!(provider.is_available());

        provider.circuit_state = CircuitState::Open;
        assert!(!provider.is_available());

        provider.circuit_state = CircuitState::Closed;
        provider.health = ProviderHealth::Unhealthy;
        assert!(!provider.is_available());
    }

    #[test]
    fn test_failover_request_serialization() {
        let ctx = ExecutionContext::new();
        let primary = TargetProvider::new("openai-1", "openai", "gpt-4");
        let alternates = vec![TargetProvider::new("anthropic-1", "anthropic", "claude-3")];

        let request = FailoverRequest::new(ctx, primary, alternates);
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("openai"));
        assert!(json.contains("anthropic"));
    }

    #[test]
    fn test_failover_policy_default() {
        let policy = FailoverPolicy::default();
        assert!(policy.auto_failover_enabled);
        assert_eq!(policy.max_failover_attempts, 3);
    }
}
