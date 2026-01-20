//! Policy definitions and constraints
//!
//! Agents consume policies but NEVER modify them.
//! Policy definitions come from LLM-Policy-Engine.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Policy constraint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    /// Rate limiting constraint
    RateLimit,
    /// Circuit breaker threshold
    CircuitBreaker,
    /// Provider routing preference
    Routing,
    /// Cache policy
    Cache,
    /// Security guard
    Security,
    /// Cost limit
    Cost,
    /// Model access control
    ModelAccess,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PolicyRule {
    /// Unique rule identifier
    #[validate(length(min = 1, max = 64))]
    pub rule_id: String,

    /// Rule name
    #[validate(length(min = 1, max = 128))]
    pub name: String,

    /// Constraint type
    pub constraint_type: ConstraintType,

    /// Rule configuration (type-specific JSON)
    pub config: serde_json::Value,

    /// Priority (lower = higher priority)
    #[validate(range(min = 0, max = 1000))]
    pub priority: u32,

    /// Whether the rule is enabled
    pub enabled: bool,

    /// Scope (tenant, user, global)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Policy constraint applied during evaluation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PolicyConstraint {
    /// Rule that created this constraint
    #[validate(length(min = 1, max = 64))]
    pub rule_id: String,

    /// Constraint type
    pub constraint_type: ConstraintType,

    /// Whether the constraint was satisfied
    pub satisfied: bool,

    /// Constraint value/limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<serde_json::Value>,

    /// Current value being evaluated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<serde_json::Value>,

    /// Message explaining the constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Policy violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Rule that was violated
    pub rule_id: String,

    /// Constraint type
    pub constraint_type: ConstraintType,

    /// Violation message
    pub message: String,

    /// Severity (info, warning, error, critical)
    pub severity: ViolationSeverity,

    /// Suggested action
    pub action: ViolationAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationAction {
    Allow,
    Warn,
    Block,
    Throttle,
    Failover,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_constraint_serialization() {
        let constraint = PolicyConstraint {
            rule_id: "cb-001".to_string(),
            constraint_type: ConstraintType::CircuitBreaker,
            satisfied: true,
            limit: Some(serde_json::json!({"threshold": 5})),
            current: Some(serde_json::json!({"failures": 3})),
            message: Some("Within threshold".to_string()),
        };

        let json = serde_json::to_string(&constraint).unwrap();
        let parsed: PolicyConstraint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.rule_id, "cb-001");
    }
}
