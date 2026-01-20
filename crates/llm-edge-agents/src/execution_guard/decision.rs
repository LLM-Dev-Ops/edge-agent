//! Decision logic for Execution Guard Agent
//!
//! This module provides deterministic decision-making for execution guard
//! requests. Decisions are based on validation results and resource limits.
//!
//! # Decision Types
//!
//! - `allow`: All safety constraints passed
//! - `block`: One or more safety constraints violated
//!
//! # Non-Responsibilities (MUST NEVER DO)
//!
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Persist state locally (use ruvector-service only)

use crate::contracts::{Constraint, ConstraintType};
use crate::error::AgentResult;

use super::types::{
    ExecutionGuardConfig, ExecutionGuardDecision, ExecutionGuardRequest, ExecutionHints,
    GuardViolation, ResourceLimits, ValidationResult,
};

/// Context for making a guard decision
pub struct GuardDecisionContext<'a> {
    pub request: &'a ExecutionGuardRequest,
    pub config: &'a ExecutionGuardConfig,
    pub validation: &'a ValidationResult,
}

/// Make an execution guard decision
///
/// This function is deterministic: given the same inputs, it will always
/// produce the same output. It does NOT:
/// - Perform orchestration
/// - Trigger retries
/// - Emit alerts
/// - Modify policies
pub fn make_guard_decision(context: GuardDecisionContext) -> AgentResult<ExecutionGuardDecision> {
    let GuardDecisionContext {
        request,
        config,
        validation,
    } = context;

    let limits = request.limits.as_ref().unwrap_or(&config.default_limits);
    let mut constraints_evaluated = Vec::new();
    let mut violations = Vec::new();

    // Evaluate token constraint
    let token_constraint = Constraint {
        constraint_type: ConstraintType::Resource,
        name: "token_limits".to_string(),
        passed: validation.tokens_valid,
        details: if validation.tokens_valid {
            Some("Token limits satisfied".to_string())
        } else {
            Some(format!(
                "{} token limit violations",
                validation.errors.iter().filter(|e| e.code.contains("TOKEN")).count()
            ))
        },
        severity: if validation.tokens_valid { None } else { Some(0.8) },
    };
    constraints_evaluated.push(token_constraint);

    // Add token violations
    if !validation.tokens_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("TOKEN")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate time constraint
    let time_constraint = Constraint {
        constraint_type: ConstraintType::Resource,
        name: "time_limits".to_string(),
        passed: validation.time_valid,
        details: if validation.time_valid {
            Some("Time limits satisfied".to_string())
        } else {
            Some("Execution time limit exceeded".to_string())
        },
        severity: if validation.time_valid { None } else { Some(0.7) },
    };
    constraints_evaluated.push(time_constraint);

    if !validation.time_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("DURATION")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate memory constraint
    let memory_constraint = Constraint {
        constraint_type: ConstraintType::Resource,
        name: "memory_limits".to_string(),
        passed: validation.memory_valid,
        details: if validation.memory_valid {
            Some("Memory limits satisfied".to_string())
        } else {
            Some("Memory limit exceeded".to_string())
        },
        severity: if validation.memory_valid { None } else { Some(0.8) },
    };
    constraints_evaluated.push(memory_constraint);

    if !validation.memory_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("MEMORY")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate cost constraint
    let cost_constraint = Constraint {
        constraint_type: ConstraintType::Resource,
        name: "cost_limits".to_string(),
        passed: validation.cost_valid,
        details: if validation.cost_valid {
            Some("Cost limits satisfied".to_string())
        } else {
            Some("Cost limit exceeded".to_string())
        },
        severity: if validation.cost_valid { None } else { Some(0.9) },
    };
    constraints_evaluated.push(cost_constraint);

    if !validation.cost_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("COST")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate rate limit constraint
    let rate_constraint = Constraint {
        constraint_type: ConstraintType::RateLimit,
        name: "rate_limits".to_string(),
        passed: validation.rate_valid,
        details: if validation.rate_valid {
            Some("Rate limits satisfied".to_string())
        } else {
            Some("Rate limit exceeded".to_string())
        },
        severity: if validation.rate_valid { None } else { Some(0.7) },
    };
    constraints_evaluated.push(rate_constraint);

    if !validation.rate_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("RATE")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate concurrency constraint
    let concurrency_constraint = Constraint {
        constraint_type: ConstraintType::Resource,
        name: "concurrency_limits".to_string(),
        passed: validation.concurrency_valid,
        details: if validation.concurrency_valid {
            Some("Concurrency limits satisfied".to_string())
        } else {
            Some("Concurrency limit exceeded".to_string())
        },
        severity: if validation.concurrency_valid { None } else { Some(0.7) },
    };
    constraints_evaluated.push(concurrency_constraint);

    if !validation.concurrency_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("CONCURRENCY")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Evaluate model constraint
    let model_constraint = Constraint {
        constraint_type: ConstraintType::Guard,
        name: "model_guard".to_string(),
        passed: validation.model_valid,
        details: if validation.model_valid {
            Some("Model is allowed".to_string())
        } else {
            Some("Model is blocked or not in allowlist".to_string())
        },
        severity: if validation.model_valid { None } else { Some(0.9) },
    };
    constraints_evaluated.push(model_constraint);

    if !validation.model_valid {
        for error in validation.errors.iter().filter(|e| e.code.contains("MODEL")) {
            violations.push(build_violation(error, limits));
        }
    }

    // Calculate overall decision
    let allowed = validation.valid;

    // Calculate confidence - guard decisions are deterministic (threshold-based)
    let confidence = calculate_confidence(&constraints_evaluated);

    // Build reason string
    let reason = if allowed {
        format!(
            "Execution allowed - all {} safety constraints passed",
            constraints_evaluated.len()
        )
    } else {
        let failed_count = constraints_evaluated.iter().filter(|c| !c.passed).count();
        format!(
            "Execution blocked - {} of {} safety constraints violated",
            failed_count,
            constraints_evaluated.len()
        )
    };

    // Build suggested adjustments if blocked
    let suggested_adjustments = if !allowed {
        Some(build_suggestions(&violations, limits))
    } else {
        None
    };

    // Build execution hints if allowed
    let execution_hints = if allowed {
        Some(build_execution_hints(request, limits))
    } else {
        None
    };

    Ok(ExecutionGuardDecision {
        allowed,
        reason,
        confidence,
        constraints_evaluated,
        violations,
        suggested_adjustments,
        execution_hints,
    })
}

/// Build a violation from a validation error
fn build_violation(
    error: &super::types::ValidationError,
    limits: &ResourceLimits,
) -> GuardViolation {
    let (actual_value, limit_value) = extract_values_from_error(error, limits);

    GuardViolation {
        constraint_type: error.code.clone(),
        description: error.message.clone(),
        actual_value,
        limit_value,
        severity: error.severity,
    }
}

/// Extract actual and limit values from error message
fn extract_values_from_error(
    error: &super::types::ValidationError,
    limits: &ResourceLimits,
) -> (String, String) {
    match error.code.as_str() {
        "INPUT_TOKENS_EXCEEDED" => (
            error.message.split_whitespace().nth(2).unwrap_or("N/A").to_string(),
            limits.max_input_tokens.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
        ),
        "OUTPUT_TOKENS_EXCEEDED" => (
            error.message.split_whitespace().nth(3).unwrap_or("N/A").to_string(),
            limits.max_output_tokens.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
        ),
        "TOTAL_TOKENS_EXCEEDED" => (
            error.message.split_whitespace().nth(2).unwrap_or("N/A").to_string(),
            limits.max_total_tokens.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
        ),
        "COST_EXCEEDED" => (
            "over budget".to_string(),
            limits.max_cost_micros.map(|v| format!("${:.4}", v as f64 / 1_000_000.0)).unwrap_or("N/A".to_string()),
        ),
        "CONCURRENCY_LIMIT_EXCEEDED" => (
            "at capacity".to_string(),
            limits.max_concurrency.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
        ),
        "RATE_LIMIT_EXCEEDED" => (
            "rate exceeded".to_string(),
            limits.rate_limit_requests.map(|v| v.to_string()).unwrap_or("N/A".to_string()),
        ),
        _ => ("N/A".to_string(), "N/A".to_string()),
    }
}

/// Calculate decision confidence
///
/// Guard decisions are deterministic (threshold-based), so confidence
/// is typically 1.0 unless there are edge cases or warnings.
fn calculate_confidence(constraints: &[Constraint]) -> f64 {
    // Start with full confidence - guard decisions are deterministic
    let mut confidence: f64 = 1.0;

    // Minor reduction for each failed constraint (to indicate certainty of block)
    // This doesn't mean "uncertain" - it means "definitely blocked for these reasons"
    for constraint in constraints {
        if !constraint.passed {
            // Each violation slightly reduces confidence only if severity is low
            if let Some(severity) = constraint.severity {
                if severity < 0.5 {
                    confidence -= 0.01; // Minor reduction for low-severity issues
                }
            }
        }
    }

    // Guard decisions are deterministic, so confidence stays high
    confidence.max(0.95)
}

/// Build suggestions for blocked requests
fn build_suggestions(violations: &[GuardViolation], limits: &ResourceLimits) -> Vec<String> {
    let mut suggestions = Vec::new();

    for violation in violations {
        match violation.constraint_type.as_str() {
            "INPUT_TOKENS_EXCEEDED" => {
                suggestions.push(format!(
                    "Reduce input tokens to below {}",
                    limits.max_input_tokens.unwrap_or(0)
                ));
                suggestions.push("Consider chunking or summarizing input".to_string());
            }
            "OUTPUT_TOKENS_EXCEEDED" => {
                suggestions.push(format!(
                    "Reduce max_output_tokens to below {}",
                    limits.max_output_tokens.unwrap_or(0)
                ));
            }
            "TOTAL_TOKENS_EXCEEDED" => {
                suggestions.push("Reduce total token count (input + output)".to_string());
                suggestions.push("Consider using a smaller model".to_string());
            }
            "COST_EXCEEDED" => {
                suggestions.push("Use a cheaper model or reduce token count".to_string());
                suggestions.push("Request higher cost limits if authorized".to_string());
            }
            "CONCURRENCY_LIMIT_EXCEEDED" => {
                suggestions.push("Wait for current executions to complete".to_string());
                suggestions.push("Implement request queuing".to_string());
            }
            "RATE_LIMIT_EXCEEDED" => {
                suggestions.push("Wait for rate limit window to reset".to_string());
                suggestions.push("Implement exponential backoff".to_string());
            }
            "MODEL_BLOCKED" | "MODEL_NOT_ALLOWED" => {
                suggestions.push("Use an allowed model".to_string());
                if !limits.allowed_models.is_empty() {
                    suggestions.push(format!(
                        "Allowed models: {:?}",
                        limits.allowed_models
                    ));
                }
            }
            "DURATION_EXCEEDED" => {
                suggestions.push("Reduce request complexity or token count".to_string());
                suggestions.push("Consider streaming responses".to_string());
            }
            "MEMORY_EXCEEDED" => {
                suggestions.push("Reduce request size".to_string());
                suggestions.push("Consider processing in batches".to_string());
            }
            _ => {}
        }
    }

    // Deduplicate suggestions
    suggestions.sort();
    suggestions.dedup();
    suggestions
}

/// Build execution hints for allowed requests
fn build_execution_hints(
    request: &ExecutionGuardRequest,
    limits: &ResourceLimits,
) -> ExecutionHints {
    let envelope = &request.envelope;

    // Calculate remaining budgets
    let cost_remaining = limits.max_cost_micros.and_then(|max| {
        envelope.estimated_cost_micros.map(|est| max.saturating_sub(est))
    });

    let concurrency_remaining = limits.max_concurrency.map(|max| {
        max.saturating_sub(envelope.current_concurrency)
    });

    let rate_remaining = limits.rate_limit_requests.and_then(|max| {
        request.current_rate.map(|current| max.saturating_sub(current))
    });

    ExecutionHints {
        recommended_timeout_ms: limits.max_duration_ms,
        recommended_max_tokens: limits.max_output_tokens,
        cost_budget_remaining_micros: cost_remaining,
        concurrency_remaining,
        rate_limit_remaining: rate_remaining,
        priority: Some(5), // Default priority
        custom: std::collections::HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::execution_guard::types::{ExecutionEnvelope, ExecutionGuardConfig};

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
            metadata: std::collections::HashMap::new(),
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

    #[test]
    fn test_allow_decision() {
        let request = create_test_request();
        let config = ExecutionGuardConfig::default();
        let validation = ValidationResult::default();

        let context = GuardDecisionContext {
            request: &request,
            config: &config,
            validation: &validation,
        };

        let decision = make_guard_decision(context).unwrap();

        assert!(decision.allowed);
        assert!(decision.confidence >= 0.95);
        assert!(decision.execution_hints.is_some());
        assert!(decision.violations.is_empty());
    }

    #[test]
    fn test_block_decision() {
        let request = create_test_request();
        let config = ExecutionGuardConfig::default();

        let mut validation = ValidationResult::default();
        validation.valid = false;
        validation.tokens_valid = false;
        validation.errors.push(super::super::types::ValidationError {
            code: "INPUT_TOKENS_EXCEEDED".to_string(),
            message: "Input tokens 200000 exceeds limit 100000".to_string(),
            field: Some("envelope.input_tokens".to_string()),
            severity: 0.8,
        });

        let context = GuardDecisionContext {
            request: &request,
            config: &config,
            validation: &validation,
        };

        let decision = make_guard_decision(context).unwrap();

        assert!(!decision.allowed);
        assert!(!decision.violations.is_empty());
        assert!(decision.suggested_adjustments.is_some());
        assert!(decision.execution_hints.is_none());
    }

    #[test]
    fn test_execution_hints() {
        let mut request = create_test_request();
        request.envelope.estimated_cost_micros = Some(5_000_000); // $5
        request.current_rate = Some(50);

        let config = ExecutionGuardConfig::default();
        let validation = ValidationResult::default();

        let context = GuardDecisionContext {
            request: &request,
            config: &config,
            validation: &validation,
        };

        let decision = make_guard_decision(context).unwrap();
        let hints = decision.execution_hints.unwrap();

        // Should have $5 remaining (10 - 5)
        assert_eq!(hints.cost_budget_remaining_micros, Some(5_000_000));
        // Should have 50 rate limit remaining (100 - 50)
        assert_eq!(hints.rate_limit_remaining, Some(50));
    }

    #[test]
    fn test_confidence_is_deterministic() {
        let validation = ValidationResult::default();
        let constraints = vec![
            Constraint {
                constraint_type: ConstraintType::Resource,
                name: "test".to_string(),
                passed: true,
                details: None,
                severity: None,
            },
        ];

        let confidence = calculate_confidence(&constraints);
        assert!(confidence >= 0.95);
    }
}
