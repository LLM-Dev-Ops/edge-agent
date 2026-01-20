//! Types for Tool Invocation Agent
//!
//! These types define the request/response contracts for the Tool Invocation
//! Agent. All types are designed to be:
//! - Serializable (for HTTP API)
//! - Validatable (using validator crate)
//! - Deterministic (same inputs = same outputs)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::contracts::{Constraint, ExecutionContext, InvocationConstraint};

/// Tool definition schema
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    #[validate(length(min = 1, max = 256))]
    pub name: String,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Tool parameters schema
    #[serde(default)]
    pub parameters: Vec<ToolParameter>,

    /// Required capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolParameter {
    /// Parameter name
    #[validate(length(min = 1, max = 128))]
    pub name: String,

    /// Parameter type (string, number, boolean, object, array)
    pub param_type: ParameterType,

    /// Whether this parameter is required
    #[serde(default)]
    pub required: bool,

    /// Parameter description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,

    /// Validation constraints (min, max, pattern, etc.)
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub constraints: HashMap<String, serde_json::Value>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Integer,
    Boolean,
    Object,
    Array,
    Null,
    Any,
}

/// Tool invocation request
///
/// Input schema for the Tool Invocation Agent.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolInvocationRequest {
    /// Tool to invoke
    #[validate(nested)]
    pub tool: ToolDefinition,

    /// Arguments to pass to the tool
    pub arguments: serde_json::Value,

    /// Execution context for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ExecutionContext>,

    /// Override constraints for this invocation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<InvocationConstraint>,
}

/// Tool invocation decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocationDecision {
    /// Whether the tool invocation is allowed
    pub allowed: bool,

    /// Reason for the decision
    pub reason: String,

    /// Decision confidence (0.0 = uncertain, 1.0 = certain)
    pub confidence: f64,

    /// Constraints that were evaluated
    pub constraints_evaluated: Vec<Constraint>,

    /// Suggested modifications (if blocked but could be allowed with changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_modifications: Option<Vec<String>>,

    /// Execution hints (if allowed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_hints: Option<ExecutionHints>,
}

/// Execution hints for allowed invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHints {
    /// Recommended timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_timeout_ms: Option<u64>,

    /// Maximum memory recommendation in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,

    /// Priority level (0 = lowest, 10 = highest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,

    /// Additional hints
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Tool invocation response
///
/// Output schema for the Tool Invocation Agent.
/// This is deterministic and machine-readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocationResponse {
    /// Request ID for correlation
    pub request_id: String,

    /// Execution reference for distributed tracing
    pub execution_ref: String,

    /// The decision made
    pub decision: ToolInvocationDecision,

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

    /// Schema validation status
    pub schema_valid: bool,

    /// Security validation status
    pub security_valid: bool,

    /// Constraint validation status
    pub constraints_valid: bool,

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
            schema_valid: true,
            security_valid: true,
            constraints_valid: true,
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
pub struct ToolInvocationConfig {
    /// Default constraints if not specified in request
    pub default_constraints: InvocationConstraint,

    /// Global tool whitelist (empty = allow all not blocked)
    #[serde(default)]
    pub global_allowed_tools: Vec<String>,

    /// Global tool blacklist
    #[serde(default)]
    pub global_blocked_tools: Vec<String>,

    /// Enable strict schema validation
    #[serde(default = "default_true")]
    pub strict_schema_validation: bool,

    /// Maximum argument size in bytes
    #[serde(default = "default_max_arg_size")]
    pub max_argument_size_bytes: usize,

    /// Enable security checks
    #[serde(default = "default_true")]
    pub enable_security_checks: bool,

    /// Known dangerous patterns to block
    #[serde(default)]
    pub dangerous_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_max_arg_size() -> usize {
    1024 * 1024 // 1MB default
}

impl Default for ToolInvocationConfig {
    fn default() -> Self {
        Self {
            default_constraints: InvocationConstraint::default(),
            global_allowed_tools: Vec::new(),
            global_blocked_tools: vec![
                "eval".to_string(),
                "exec".to_string(),
                "system".to_string(),
                "shell".to_string(),
                "os.system".to_string(),
                "subprocess".to_string(),
            ],
            strict_schema_validation: true,
            max_argument_size_bytes: 1024 * 1024,
            enable_security_checks: true,
            dangerous_patterns: vec![
                r"rm\s+-rf".to_string(),
                r"DROP\s+TABLE".to_string(),
                r";\s*rm\s+".to_string(),
                r"\$\(.*\)".to_string(),
                r"`.*`".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "calculator".to_string(),
            description: Some("A simple calculator".to_string()),
            parameters: vec![ToolParameter {
                name: "expression".to_string(),
                param_type: ParameterType::String,
                required: true,
                description: Some("Math expression".to_string()),
                default: None,
                constraints: HashMap::new(),
            }],
            capabilities: vec!["math".to_string()],
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("calculator"));
        assert!(json.contains("expression"));
    }

    #[test]
    fn test_config_defaults() {
        let config = ToolInvocationConfig::default();
        assert!(config.global_blocked_tools.contains(&"eval".to_string()));
        assert!(config.strict_schema_validation);
        assert!(config.enable_security_checks);
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.valid);
        assert!(result.schema_valid);
        assert!(result.security_valid);
        assert!(result.constraints_valid);
    }
}
