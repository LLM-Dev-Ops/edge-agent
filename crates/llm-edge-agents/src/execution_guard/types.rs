//! Types for Execution Guard Agent
//!
//! These types define the request/response contracts for the Execution Guard
//! Agent. All types are designed to be:
//! - Serializable (for HTTP API)
//! - Validatable (using validator crate)
//! - Deterministic (same inputs = same outputs)
//!
//! # Decision Types
//!
//! - `allow`: Execution envelope passes all safety constraints
//! - `block`: Execution envelope violates one or more safety constraints
//!
//! # Constraint Types Enforced
//!
//! - Token limits (max input/output tokens)
//! - Time limits (max execution duration)
//! - Memory limits (max memory usage)
//! - Cost limits (max cost per request)
//! - Rate limits (requests per time window)
//! - Concurrency limits (max concurrent executions)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::contracts::{Constraint, ExecutionContext};

/// Execution envelope to be guarded
///
/// This represents the execution request that needs safety validation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExecutionEnvelope {
    /// Unique identifier for this execution request
    #[validate(length(min = 1, max = 128))]
    pub request_id: String,

    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    #[validate(length(min = 1, max = 128))]
    pub model: String,

    /// Provider identifier (e.g., "openai", "anthropic")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Estimated input tokens
    pub input_tokens: u64,

    /// Maximum output tokens requested
    pub max_output_tokens: u64,

    /// Estimated execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,

    /// Estimated memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_memory_bytes: Option<u64>,

    /// Estimated cost in USD (microdollars for precision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_micros: Option<u64>,

    /// Current concurrent executions for this tenant
    #[serde(default)]
    pub current_concurrency: u32,

    /// Request metadata
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Resource limits configuration
///
/// Defines the safety constraints that the Execution Guard enforces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum input tokens allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u64>,

    /// Maximum output tokens allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,

    /// Maximum total tokens (input + output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_tokens: Option<u64>,

    /// Maximum execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration_ms: Option<u64>,

    /// Maximum memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,

    /// Maximum cost per request in microdollars
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_micros: Option<u64>,

    /// Maximum concurrent executions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<u32>,

    /// Rate limit: max requests per window
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_requests: Option<u32>,

    /// Rate limit window in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_window_secs: Option<u32>,

    /// Blocked models (blacklist)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_models: Vec<String>,

    /// Allowed models (whitelist, empty = all allowed)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_models: Vec<String>,

    /// Custom limits
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_tokens: Some(100_000),      // 100K tokens
            max_output_tokens: Some(16_000),      // 16K tokens
            max_total_tokens: Some(128_000),      // 128K tokens
            max_duration_ms: Some(120_000),       // 2 minutes
            max_memory_bytes: Some(1_073_741_824), // 1GB
            max_cost_micros: Some(10_000_000),    // $10.00
            max_concurrency: Some(10),
            rate_limit_requests: Some(100),
            rate_limit_window_secs: Some(60),
            blocked_models: Vec::new(),
            allowed_models: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// Execution guard request
///
/// Input schema for the Execution Guard Agent.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExecutionGuardRequest {
    /// Execution envelope to validate
    #[validate(nested)]
    pub envelope: ExecutionEnvelope,

    /// Execution context for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ExecutionContext>,

    /// Override resource limits for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<ResourceLimits>,

    /// Current rate limit state (requests in current window)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_rate: Option<u32>,
}

/// Execution guard decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGuardDecision {
    /// Whether the execution is allowed
    pub allowed: bool,

    /// Reason for the decision
    pub reason: String,

    /// Decision confidence (0.0 = uncertain, 1.0 = certain)
    /// For guard decisions, this is typically 1.0 (deterministic threshold-based)
    pub confidence: f64,

    /// Constraints that were evaluated
    pub constraints_evaluated: Vec<Constraint>,

    /// Which constraints failed (if blocked)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<GuardViolation>,

    /// Suggested adjustments (if blocked but could be allowed with changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_adjustments: Option<Vec<String>>,

    /// Execution hints (if allowed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_hints: Option<ExecutionHints>,
}

/// A specific constraint violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardViolation {
    /// Constraint type that was violated
    pub constraint_type: String,

    /// Human-readable description of the violation
    pub description: String,

    /// Actual value that caused the violation
    pub actual_value: String,

    /// Limit that was exceeded
    pub limit_value: String,

    /// Severity of the violation (0.0 = warning, 1.0 = critical)
    pub severity: f64,
}

/// Execution hints for allowed requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHints {
    /// Recommended timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_timeout_ms: Option<u64>,

    /// Recommended max output tokens (may be adjusted down)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_max_tokens: Option<u64>,

    /// Cost budget remaining (microdollars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_budget_remaining_micros: Option<u64>,

    /// Concurrency slots remaining
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_remaining: Option<u32>,

    /// Rate limit remaining
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_remaining: Option<u32>,

    /// Priority level (0 = lowest, 10 = highest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,

    /// Additional hints
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Execution guard response
///
/// Output schema for the Execution Guard Agent.
/// This is deterministic and machine-readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGuardResponse {
    /// Request ID for correlation
    pub request_id: String,

    /// Execution reference for distributed tracing
    pub execution_ref: String,

    /// The decision made
    pub decision: ExecutionGuardDecision,

    /// Validation results
    pub validation: ValidationResult,

    /// Processing time in microseconds
    pub processing_time_us: u64,
}

/// Validation result details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Token limits valid
    pub tokens_valid: bool,

    /// Time limits valid
    pub time_valid: bool,

    /// Memory limits valid
    pub memory_valid: bool,

    /// Cost limits valid
    pub cost_valid: bool,

    /// Rate limits valid
    pub rate_valid: bool,

    /// Concurrency limits valid
    pub concurrency_valid: bool,

    /// Model allowed
    pub model_valid: bool,

    /// Validation errors (if any)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationError>,

    /// Validation warnings (non-blocking)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ValidationWarning>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            tokens_valid: true,
            time_valid: true,
            memory_valid: true,
            cost_valid: true,
            rate_valid: true,
            concurrency_valid: true,
            model_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Field that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,

    /// Severity (0.0 = info, 1.0 = critical)
    pub severity: f64,
}

/// Validation warning (non-blocking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Recommendation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGuardConfig {
    /// Default resource limits if not specified in request
    pub default_limits: ResourceLimits,

    /// Enable strict validation (fail on any warning)
    #[serde(default)]
    pub strict_mode: bool,

    /// Enable cost tracking
    #[serde(default = "default_true")]
    pub enable_cost_tracking: bool,

    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub enable_rate_limiting: bool,

    /// Enable concurrency limiting
    #[serde(default = "default_true")]
    pub enable_concurrency_limiting: bool,

    /// Warn threshold for tokens (percentage of limit)
    #[serde(default = "default_warn_threshold")]
    pub token_warn_threshold: f64,

    /// Warn threshold for cost (percentage of limit)
    #[serde(default = "default_warn_threshold")]
    pub cost_warn_threshold: f64,
}

fn default_true() -> bool {
    true
}

fn default_warn_threshold() -> f64 {
    0.8 // Warn at 80% of limit
}

impl Default for ExecutionGuardConfig {
    fn default() -> Self {
        Self {
            default_limits: ResourceLimits::default(),
            strict_mode: false,
            enable_cost_tracking: true,
            enable_rate_limiting: true,
            enable_concurrency_limiting: true,
            token_warn_threshold: 0.8,
            cost_warn_threshold: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_envelope_serialization() {
        let envelope = ExecutionEnvelope {
            request_id: "req-123".to_string(),
            model: "gpt-4".to_string(),
            provider: Some("openai".to_string()),
            input_tokens: 1000,
            max_output_tokens: 2000,
            estimated_duration_ms: Some(5000),
            estimated_memory_bytes: None,
            estimated_cost_micros: Some(50000), // $0.05
            current_concurrency: 3,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_input_tokens, Some(100_000));
        assert_eq!(limits.max_output_tokens, Some(16_000));
        assert_eq!(limits.max_concurrency, Some(10));
    }

    #[test]
    fn test_config_defaults() {
        let config = ExecutionGuardConfig::default();
        assert!(config.enable_cost_tracking);
        assert!(config.enable_rate_limiting);
        assert!(!config.strict_mode);
        assert_eq!(config.token_warn_threshold, 0.8);
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.valid);
        assert!(result.tokens_valid);
        assert!(result.cost_valid);
        assert!(result.rate_valid);
    }
}
