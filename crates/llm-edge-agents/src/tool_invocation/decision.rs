//! Decision logic for Tool Invocation Agent
//!
//! This module provides deterministic decision-making for tool invocation
//! requests. Decisions are based on validation results and constraints.

use crate::contracts::{Constraint, ConstraintType};
use crate::error::{AgentError, AgentResult};

use super::types::{
    ExecutionHints, ToolInvocationConfig, ToolInvocationDecision, ToolInvocationRequest,
    ValidationResult,
};

/// Context for making a decision
pub struct DecisionContext<'a> {
    pub request: &'a ToolInvocationRequest,
    pub config: &'a ToolInvocationConfig,
    pub validation: &'a ValidationResult,
}

/// Make a tool invocation decision
///
/// This function is deterministic: given the same inputs, it will always
/// produce the same output. It does NOT:
/// - Perform orchestration
/// - Trigger retries
/// - Emit alerts
/// - Modify policies
pub fn make_decision(context: DecisionContext) -> AgentResult<ToolInvocationDecision> {
    let DecisionContext {
        request,
        config,
        validation,
    } = context;

    let mut constraints_evaluated = Vec::new();

    // Evaluate schema constraint
    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::SchemaValidation,
        name: "schema_validation".to_string(),
        passed: validation.schema_valid,
        details: if validation.schema_valid {
            Some("Schema validation passed".to_string())
        } else {
            Some(format!(
                "{} schema errors found",
                validation.errors.iter().filter(|e| e.code.contains("SCHEMA")).count()
            ))
        },
        severity: if validation.schema_valid { None } else { Some(0.6) },
    });

    // Evaluate security constraint
    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::Security,
        name: "security_check".to_string(),
        passed: validation.security_valid,
        details: if validation.security_valid {
            Some("Security checks passed".to_string())
        } else {
            Some(format!(
                "{} security violations found",
                validation.errors.iter().filter(|e|
                    e.code.contains("BLOCKED") || e.code.contains("DANGEROUS")
                ).count()
            ))
        },
        severity: if validation.security_valid { None } else { Some(0.9) },
    });

    // Evaluate resource constraints
    let constraints = request
        .constraints
        .as_ref()
        .unwrap_or(&config.default_constraints);

    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::Resource,
        name: "resource_constraints".to_string(),
        passed: validation.constraints_valid,
        details: if validation.constraints_valid {
            Some("Resource constraints satisfied".to_string())
        } else {
            Some("Resource constraints violated".to_string())
        },
        severity: if validation.constraints_valid { None } else { Some(0.5) },
    });

    // Evaluate guard constraints (capability requirements)
    let capabilities_satisfied = constraints.required_capabilities.iter().all(|cap| {
        request.tool.capabilities.contains(cap)
    });

    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::Guard,
        name: "capability_guard".to_string(),
        passed: capabilities_satisfied,
        details: if capabilities_satisfied {
            Some("All required capabilities present".to_string())
        } else {
            Some(format!(
                "Missing capabilities: {:?}",
                constraints
                    .required_capabilities
                    .iter()
                    .filter(|c| !request.tool.capabilities.contains(c))
                    .collect::<Vec<_>>()
            ))
        },
        severity: if capabilities_satisfied { None } else { Some(0.6) },
    });

    // Calculate overall decision
    let all_passed = constraints_evaluated.iter().all(|c| c.passed);
    let allowed = validation.valid && all_passed;

    // Calculate confidence based on constraint results
    let confidence = calculate_confidence(&constraints_evaluated, validation);

    // Build reason string
    let reason = if allowed {
        format!(
            "Tool '{}' invocation allowed - all {} constraints passed",
            request.tool.name,
            constraints_evaluated.len()
        )
    } else {
        let failed_constraints: Vec<&str> = constraints_evaluated
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.name.as_str())
            .collect();

        format!(
            "Tool '{}' invocation blocked - failed constraints: {}",
            request.tool.name,
            failed_constraints.join(", ")
        )
    };

    // Build suggested modifications if blocked
    let suggested_modifications = if !allowed {
        Some(build_suggestions(validation, &constraints_evaluated))
    } else {
        None
    };

    // Build execution hints if allowed
    let execution_hints = if allowed {
        Some(ExecutionHints {
            recommended_timeout_ms: constraints.max_execution_time_ms,
            max_memory_bytes: constraints.max_memory_bytes,
            priority: Some(5), // Default priority
            custom: std::collections::HashMap::new(),
        })
    } else {
        None
    };

    Ok(ToolInvocationDecision {
        allowed,
        reason,
        confidence,
        constraints_evaluated,
        suggested_modifications,
        execution_hints,
    })
}

/// Calculate decision confidence
///
/// Confidence is based on:
/// - Number of constraints evaluated
/// - Severity of any failures
/// - Validation error count
fn calculate_confidence(constraints: &[Constraint], validation: &ValidationResult) -> f64 {
    // Start with base confidence
    let mut confidence = 1.0;

    // Reduce confidence for each failed constraint based on severity
    for constraint in constraints {
        if !constraint.passed {
            if let Some(severity) = constraint.severity {
                confidence -= severity * 0.2;
            } else {
                confidence -= 0.1;
            }
        }
    }

    // Reduce confidence for validation warnings
    confidence -= (validation.warnings.len() as f64) * 0.05;

    // Ensure confidence is in valid range
    confidence.max(0.0).min(1.0)
}

/// Build suggestions for blocked invocations
fn build_suggestions(validation: &ValidationResult, constraints: &[Constraint]) -> Vec<String> {
    let mut suggestions = Vec::new();

    // Add suggestions based on validation errors
    for error in &validation.errors {
        match error.code.as_str() {
            "TOOL_BLOCKED" => {
                suggestions.push("Use an alternative tool that is not blocked".to_string());
            }
            "TOOL_NOT_ALLOWED" => {
                suggestions.push("Request tool to be added to the allowed list".to_string());
            }
            "DANGEROUS_PATTERN" => {
                suggestions.push("Remove dangerous patterns from arguments".to_string());
            }
            "MISSING_REQUIRED_PARAM" => {
                if let Some(field) = &error.field {
                    suggestions.push(format!("Provide required parameter: {}", field));
                }
            }
            "ARGUMENT_SIZE_EXCEEDED" => {
                suggestions.push("Reduce argument size or use streaming".to_string());
            }
            _ => {}
        }
    }

    // Add suggestions based on failed constraints
    for constraint in constraints.iter().filter(|c| !c.passed) {
        match constraint.constraint_type {
            ConstraintType::Guard => {
                suggestions.push("Ensure tool has required capabilities".to_string());
            }
            ConstraintType::Resource => {
                suggestions.push("Adjust resource constraints or use a smaller payload".to_string());
            }
            _ => {}
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::tool_invocation::types::{ToolDefinition, ToolInvocationRequest};

    fn create_context<'a>(
        request: &'a ToolInvocationRequest,
        config: &'a ToolInvocationConfig,
        validation: &'a ValidationResult,
    ) -> DecisionContext<'a> {
        DecisionContext {
            request,
            config,
            validation,
        }
    }

    #[test]
    fn test_allow_decision() {
        let request = ToolInvocationRequest {
            tool: ToolDefinition {
                name: "calculator".to_string(),
                description: None,
                parameters: vec![],
                capabilities: vec![],
            },
            arguments: serde_json::json!({}),
            context: Some(ExecutionContext::new()),
            constraints: None,
        };
        let config = ToolInvocationConfig::default();
        let validation = ValidationResult::default();

        let context = create_context(&request, &config, &validation);
        let decision = make_decision(context).unwrap();

        assert!(decision.allowed);
        assert!(decision.confidence > 0.9);
        assert!(decision.execution_hints.is_some());
    }

    #[test]
    fn test_block_decision() {
        let request = ToolInvocationRequest {
            tool: ToolDefinition {
                name: "eval".to_string(),
                description: None,
                parameters: vec![],
                capabilities: vec![],
            },
            arguments: serde_json::json!({}),
            context: Some(ExecutionContext::new()),
            constraints: None,
        };
        let config = ToolInvocationConfig::default();
        let mut validation = ValidationResult::default();
        validation.security_valid = false;
        validation.valid = false;
        validation.errors.push(crate::tool_invocation::types::ValidationError {
            code: "TOOL_BLOCKED".to_string(),
            message: "Tool is blocked".to_string(),
            field: Some("tool.name".to_string()),
            severity: 0.9,
        });

        let context = create_context(&request, &config, &validation);
        let decision = make_decision(context).unwrap();

        assert!(!decision.allowed);
        assert!(decision.suggested_modifications.is_some());
        assert!(decision.execution_hints.is_none());
    }

    #[test]
    fn test_confidence_calculation() {
        let constraints = vec![
            Constraint {
                constraint_type: ConstraintType::Security,
                name: "security".to_string(),
                passed: true,
                details: None,
                severity: None,
            },
            Constraint {
                constraint_type: ConstraintType::Guard,
                name: "guard".to_string(),
                passed: false,
                details: None,
                severity: Some(0.5),
            },
        ];
        let validation = ValidationResult::default();

        let confidence = calculate_confidence(&constraints, &validation);
        assert!(confidence < 1.0);
        assert!(confidence > 0.5);
    }
}
