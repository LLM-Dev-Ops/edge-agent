//! Validation logic for Tool Invocation Agent
//!
//! This module provides deterministic validation of tool invocation requests.
//! Validation is performed against:
//! - Schema constraints
//! - Security policies
//! - Resource constraints

use regex::Regex;
use std::collections::HashSet;
use validator::Validate;

use crate::error::{AgentError, AgentResult};

use super::types::{
    ParameterType, ToolInvocationConfig, ToolInvocationRequest, ValidationError,
    ValidationResult, ValidationWarning,
};

/// Validate a tool invocation request
///
/// This function performs comprehensive validation of the request:
/// 1. Schema validation (tool definition, parameters)
/// 2. Security validation (blocked tools, dangerous patterns)
/// 3. Constraint validation (size limits, capabilities)
///
/// Returns a ValidationResult with detailed information about each check.
pub fn validate_invocation(
    request: &ToolInvocationRequest,
    config: &ToolInvocationConfig,
) -> AgentResult<ValidationResult> {
    let mut result = ValidationResult::default();

    // Step 1: Schema validation
    validate_schema(request, config, &mut result)?;

    // Step 2: Security validation
    if config.enable_security_checks {
        validate_security(request, config, &mut result)?;
    }

    // Step 3: Constraint validation
    validate_constraints(request, config, &mut result)?;

    // Update overall validity
    result.valid = result.schema_valid && result.security_valid && result.constraints_valid;

    Ok(result)
}

/// Validate request schema
fn validate_schema(
    request: &ToolInvocationRequest,
    config: &ToolInvocationConfig,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    // Validate tool definition using validator crate
    if let Err(e) = request.tool.validate() {
        result.schema_valid = false;
        for (field, errors) in e.field_errors() {
            for error in errors {
                result.errors.push(ValidationError {
                    code: "SCHEMA_VALIDATION".to_string(),
                    message: error.message.clone().map(|m| m.to_string()).unwrap_or_else(|| {
                        format!("Validation failed for field: {}", field)
                    }),
                    field: Some(field.to_string()),
                    severity: 0.6,
                });
            }
        }
    }

    // Validate tool name is not empty
    if request.tool.name.trim().is_empty() {
        result.schema_valid = false;
        result.errors.push(ValidationError {
            code: "EMPTY_TOOL_NAME".to_string(),
            message: "Tool name cannot be empty".to_string(),
            field: Some("tool.name".to_string()),
            severity: 0.8,
        });
    }

    // Validate arguments size
    let arg_size = serde_json::to_string(&request.arguments)
        .map(|s| s.len())
        .unwrap_or(0);

    if arg_size > config.max_argument_size_bytes {
        result.schema_valid = false;
        result.errors.push(ValidationError {
            code: "ARGUMENT_SIZE_EXCEEDED".to_string(),
            message: format!(
                "Arguments size {} exceeds maximum {}",
                arg_size, config.max_argument_size_bytes
            ),
            field: Some("arguments".to_string()),
            severity: 0.7,
        });
    }

    // Validate parameter types if arguments is an object
    if let Some(args_obj) = request.arguments.as_object() {
        if config.strict_schema_validation {
            validate_parameter_types(&request.tool.parameters, args_obj, result);
        }
    }

    Ok(())
}

/// Validate parameter types against schema
fn validate_parameter_types(
    parameters: &[super::types::ToolParameter],
    args: &serde_json::Map<String, serde_json::Value>,
    result: &mut ValidationResult,
) {
    let param_names: HashSet<&str> = parameters.iter().map(|p| p.name.as_str()).collect();

    // Check for required parameters
    for param in parameters {
        if param.required && !args.contains_key(&param.name) {
            result.schema_valid = false;
            result.errors.push(ValidationError {
                code: "MISSING_REQUIRED_PARAM".to_string(),
                message: format!("Required parameter '{}' is missing", param.name),
                field: Some(format!("arguments.{}", param.name)),
                severity: 0.7,
            });
            continue;
        }

        // Validate type if parameter is provided
        if let Some(value) = args.get(&param.name) {
            if !validate_param_type(value, &param.param_type) {
                result.schema_valid = false;
                result.errors.push(ValidationError {
                    code: "INVALID_PARAM_TYPE".to_string(),
                    message: format!(
                        "Parameter '{}' has invalid type, expected {:?}",
                        param.name, param.param_type
                    ),
                    field: Some(format!("arguments.{}", param.name)),
                    severity: 0.6,
                });
            }
        }
    }

    // Warn about unknown parameters
    for key in args.keys() {
        if !param_names.contains(key.as_str()) {
            result.warnings.push(ValidationWarning {
                code: "UNKNOWN_PARAM".to_string(),
                message: format!("Unknown parameter: {}", key),
                recommendation: Some("Remove this parameter or update the tool definition".to_string()),
            });
        }
    }
}

/// Check if a value matches the expected parameter type
fn validate_param_type(value: &serde_json::Value, expected: &ParameterType) -> bool {
    match expected {
        ParameterType::String => value.is_string(),
        ParameterType::Number => value.is_number(),
        ParameterType::Integer => value.is_i64() || value.is_u64(),
        ParameterType::Boolean => value.is_boolean(),
        ParameterType::Object => value.is_object(),
        ParameterType::Array => value.is_array(),
        ParameterType::Null => value.is_null(),
        ParameterType::Any => true,
    }
}

/// Validate security constraints
fn validate_security(
    request: &ToolInvocationRequest,
    config: &ToolInvocationConfig,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    let tool_name = &request.tool.name;

    // Check if tool is globally blocked
    if config.global_blocked_tools.contains(tool_name) {
        result.security_valid = false;
        result.errors.push(ValidationError {
            code: "TOOL_BLOCKED".to_string(),
            message: format!("Tool '{}' is blocked by security policy", tool_name),
            field: Some("tool.name".to_string()),
            severity: 0.9,
        });
        return Ok(());
    }

    // Check if tool is in allowlist (if allowlist is configured)
    if !config.global_allowed_tools.is_empty()
        && !config.global_allowed_tools.contains(tool_name)
    {
        result.security_valid = false;
        result.errors.push(ValidationError {
            code: "TOOL_NOT_ALLOWED".to_string(),
            message: format!("Tool '{}' is not in the allowed tools list", tool_name),
            field: Some("tool.name".to_string()),
            severity: 0.8,
        });
        return Ok(());
    }

    // Check request-level constraints
    if let Some(ref constraints) = request.constraints {
        // Check blocked tools in request constraints
        if constraints.blocked_tools.contains(tool_name) {
            result.security_valid = false;
            result.errors.push(ValidationError {
                code: "TOOL_BLOCKED_BY_REQUEST".to_string(),
                message: format!("Tool '{}' is blocked by request constraints", tool_name),
                field: Some("tool.name".to_string()),
                severity: 0.8,
            });
            return Ok(());
        }

        // Check allowed tools in request constraints
        if !constraints.allowed_tools.is_empty()
            && !constraints.allowed_tools.contains(tool_name)
        {
            result.security_valid = false;
            result.errors.push(ValidationError {
                code: "TOOL_NOT_IN_REQUEST_ALLOWLIST".to_string(),
                message: format!(
                    "Tool '{}' is not in the request's allowed tools list",
                    tool_name
                ),
                field: Some("tool.name".to_string()),
                severity: 0.7,
            });
            return Ok(());
        }
    }

    // Check for dangerous patterns in arguments
    let args_str = serde_json::to_string(&request.arguments).unwrap_or_default();
    for pattern in &config.dangerous_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&args_str) {
                result.security_valid = false;
                result.errors.push(ValidationError {
                    code: "DANGEROUS_PATTERN".to_string(),
                    message: format!(
                        "Arguments contain dangerous pattern: {}",
                        pattern
                    ),
                    field: Some("arguments".to_string()),
                    severity: 1.0,
                });
            }
        }
    }

    Ok(())
}

/// Validate resource constraints
fn validate_constraints(
    request: &ToolInvocationRequest,
    config: &ToolInvocationConfig,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    let constraints = request
        .constraints
        .as_ref()
        .unwrap_or(&config.default_constraints);

    // Check required capabilities
    for required_cap in &constraints.required_capabilities {
        if !request.tool.capabilities.contains(required_cap) {
            result.constraints_valid = false;
            result.errors.push(ValidationError {
                code: "MISSING_CAPABILITY".to_string(),
                message: format!(
                    "Tool is missing required capability: {}",
                    required_cap
                ),
                field: Some("tool.capabilities".to_string()),
                severity: 0.6,
            });
        }
    }

    // Warn about large arguments that might cause timeout
    let arg_size = serde_json::to_string(&request.arguments)
        .map(|s| s.len())
        .unwrap_or(0);

    if arg_size > 100_000 {
        // 100KB
        result.warnings.push(ValidationWarning {
            code: "LARGE_ARGUMENTS".to_string(),
            message: format!("Arguments are large ({} bytes)", arg_size),
            recommendation: Some("Consider streaming or chunking large payloads".to_string()),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::tool_invocation::types::ToolDefinition;

    fn create_test_request(tool_name: &str, args: serde_json::Value) -> ToolInvocationRequest {
        ToolInvocationRequest {
            tool: ToolDefinition {
                name: tool_name.to_string(),
                description: Some("Test tool".to_string()),
                parameters: vec![],
                capabilities: vec![],
            },
            arguments: args,
            context: Some(ExecutionContext::new()),
            constraints: None,
        }
    }

    #[test]
    fn test_valid_request() {
        let config = ToolInvocationConfig::default();
        let request = create_test_request("calculator", serde_json::json!({"x": 1}));

        let result = validate_invocation(&request, &config).unwrap();
        assert!(result.valid);
        assert!(result.schema_valid);
        assert!(result.security_valid);
    }

    #[test]
    fn test_blocked_tool() {
        let config = ToolInvocationConfig::default();
        let request = create_test_request("eval", serde_json::json!({}));

        let result = validate_invocation(&request, &config).unwrap();
        assert!(!result.valid);
        assert!(!result.security_valid);
        assert!(result.errors.iter().any(|e| e.code == "TOOL_BLOCKED"));
    }

    #[test]
    fn test_dangerous_pattern() {
        let config = ToolInvocationConfig::default();
        let request = create_test_request(
            "shell",
            serde_json::json!({"command": "rm -rf /"}),
        );

        let result = validate_invocation(&request, &config).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "DANGEROUS_PATTERN" || e.code == "TOOL_BLOCKED"));
    }

    #[test]
    fn test_empty_tool_name() {
        let config = ToolInvocationConfig::default();
        let request = create_test_request("", serde_json::json!({}));

        let result = validate_invocation(&request, &config).unwrap();
        assert!(!result.valid);
        assert!(!result.schema_valid);
    }
}
