//! Validation logic for Execution Guard Agent
//!
//! This module provides deterministic validation of execution envelopes
//! against resource limits. Validation is performed against:
//! - Token limits (input, output, total)
//! - Time limits (execution duration)
//! - Memory limits
//! - Cost limits
//! - Rate limits
//! - Concurrency limits
//! - Model restrictions

use crate::error::{AgentError, AgentResult};

use super::types::{
    ExecutionGuardConfig, ExecutionGuardRequest, ResourceLimits, ValidationError,
    ValidationResult, ValidationWarning,
};

/// Validate an execution guard request
///
/// This function performs comprehensive validation of the execution envelope:
/// 1. Token validation (input, output, total)
/// 2. Time validation (execution duration)
/// 3. Memory validation
/// 4. Cost validation
/// 5. Rate limit validation
/// 6. Concurrency validation
/// 7. Model validation (allowed/blocked)
///
/// Returns a ValidationResult with detailed information about each check.
pub fn validate_execution(
    request: &ExecutionGuardRequest,
    config: &ExecutionGuardConfig,
) -> AgentResult<ValidationResult> {
    let mut result = ValidationResult::default();
    let limits = request.limits.as_ref().unwrap_or(&config.default_limits);

    // Step 1: Token validation
    validate_tokens(&request.envelope, limits, config, &mut result)?;

    // Step 2: Time validation
    validate_time(&request.envelope, limits, &mut result)?;

    // Step 3: Memory validation
    validate_memory(&request.envelope, limits, &mut result)?;

    // Step 4: Cost validation
    if config.enable_cost_tracking {
        validate_cost(&request.envelope, limits, config, &mut result)?;
    }

    // Step 5: Rate limit validation
    if config.enable_rate_limiting {
        validate_rate_limit(request, limits, &mut result)?;
    }

    // Step 6: Concurrency validation
    if config.enable_concurrency_limiting {
        validate_concurrency(&request.envelope, limits, &mut result)?;
    }

    // Step 7: Model validation
    validate_model(&request.envelope, limits, &mut result)?;

    // Update overall validity
    result.valid = result.tokens_valid
        && result.time_valid
        && result.memory_valid
        && result.cost_valid
        && result.rate_valid
        && result.concurrency_valid
        && result.model_valid;

    // In strict mode, warnings also cause failure
    if config.strict_mode && !result.warnings.is_empty() {
        result.valid = false;
    }

    Ok(result)
}

/// Validate token limits
fn validate_tokens(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    config: &ExecutionGuardConfig,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    // Validate input tokens
    if let Some(max_input) = limits.max_input_tokens {
        if envelope.input_tokens > max_input {
            result.tokens_valid = false;
            result.errors.push(ValidationError {
                code: "INPUT_TOKENS_EXCEEDED".to_string(),
                message: format!(
                    "Input tokens {} exceeds limit {}",
                    envelope.input_tokens, max_input
                ),
                field: Some("envelope.input_tokens".to_string()),
                severity: 0.8,
            });
        } else {
            // Check warn threshold
            let ratio = envelope.input_tokens as f64 / max_input as f64;
            if ratio > config.token_warn_threshold {
                result.warnings.push(ValidationWarning {
                    code: "INPUT_TOKENS_HIGH".to_string(),
                    message: format!(
                        "Input tokens {} is at {:.0}% of limit {}",
                        envelope.input_tokens,
                        ratio * 100.0,
                        max_input
                    ),
                    recommendation: Some("Consider reducing input size".to_string()),
                });
            }
        }
    }

    // Validate output tokens
    if let Some(max_output) = limits.max_output_tokens {
        if envelope.max_output_tokens > max_output {
            result.tokens_valid = false;
            result.errors.push(ValidationError {
                code: "OUTPUT_TOKENS_EXCEEDED".to_string(),
                message: format!(
                    "Requested output tokens {} exceeds limit {}",
                    envelope.max_output_tokens, max_output
                ),
                field: Some("envelope.max_output_tokens".to_string()),
                severity: 0.8,
            });
        }
    }

    // Validate total tokens
    if let Some(max_total) = limits.max_total_tokens {
        let total_tokens = envelope.input_tokens + envelope.max_output_tokens;
        if total_tokens > max_total {
            result.tokens_valid = false;
            result.errors.push(ValidationError {
                code: "TOTAL_TOKENS_EXCEEDED".to_string(),
                message: format!(
                    "Total tokens {} (input: {} + output: {}) exceeds limit {}",
                    total_tokens, envelope.input_tokens, envelope.max_output_tokens, max_total
                ),
                field: Some("envelope.total_tokens".to_string()),
                severity: 0.9,
            });
        }
    }

    Ok(())
}

/// Validate time limits
fn validate_time(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    if let (Some(estimated), Some(max_duration)) =
        (envelope.estimated_duration_ms, limits.max_duration_ms)
    {
        if estimated > max_duration {
            result.time_valid = false;
            result.errors.push(ValidationError {
                code: "DURATION_EXCEEDED".to_string(),
                message: format!(
                    "Estimated duration {}ms exceeds limit {}ms",
                    estimated, max_duration
                ),
                field: Some("envelope.estimated_duration_ms".to_string()),
                severity: 0.7,
            });
        }
    }

    Ok(())
}

/// Validate memory limits
fn validate_memory(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    if let (Some(estimated), Some(max_memory)) =
        (envelope.estimated_memory_bytes, limits.max_memory_bytes)
    {
        if estimated > max_memory {
            result.memory_valid = false;
            result.errors.push(ValidationError {
                code: "MEMORY_EXCEEDED".to_string(),
                message: format!(
                    "Estimated memory {} bytes exceeds limit {} bytes",
                    estimated, max_memory
                ),
                field: Some("envelope.estimated_memory_bytes".to_string()),
                severity: 0.8,
            });
        }
    }

    Ok(())
}

/// Validate cost limits
fn validate_cost(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    config: &ExecutionGuardConfig,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    if let (Some(estimated), Some(max_cost)) =
        (envelope.estimated_cost_micros, limits.max_cost_micros)
    {
        if estimated > max_cost {
            result.cost_valid = false;
            result.errors.push(ValidationError {
                code: "COST_EXCEEDED".to_string(),
                message: format!(
                    "Estimated cost ${:.4} exceeds limit ${:.4}",
                    estimated as f64 / 1_000_000.0,
                    max_cost as f64 / 1_000_000.0
                ),
                field: Some("envelope.estimated_cost_micros".to_string()),
                severity: 0.9,
            });
        } else {
            // Check warn threshold
            let ratio = estimated as f64 / max_cost as f64;
            if ratio > config.cost_warn_threshold {
                result.warnings.push(ValidationWarning {
                    code: "COST_HIGH".to_string(),
                    message: format!(
                        "Estimated cost ${:.4} is at {:.0}% of limit ${:.4}",
                        estimated as f64 / 1_000_000.0,
                        ratio * 100.0,
                        max_cost as f64 / 1_000_000.0
                    ),
                    recommendation: Some("Consider using a cheaper model or reducing tokens".to_string()),
                });
            }
        }
    }

    Ok(())
}

/// Validate rate limits
fn validate_rate_limit(
    request: &ExecutionGuardRequest,
    limits: &ResourceLimits,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    if let (Some(current_rate), Some(max_requests)) =
        (request.current_rate, limits.rate_limit_requests)
    {
        if current_rate >= max_requests {
            result.rate_valid = false;
            result.errors.push(ValidationError {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                message: format!(
                    "Rate limit exceeded: {} requests in current window (limit: {})",
                    current_rate, max_requests
                ),
                field: Some("current_rate".to_string()),
                severity: 0.7,
            });
        } else if current_rate as f64 / max_requests as f64 > 0.9 {
            result.warnings.push(ValidationWarning {
                code: "RATE_LIMIT_APPROACHING".to_string(),
                message: format!(
                    "Rate limit approaching: {} of {} requests used",
                    current_rate, max_requests
                ),
                recommendation: Some("Consider spacing out requests".to_string()),
            });
        }
    }

    Ok(())
}

/// Validate concurrency limits
fn validate_concurrency(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    if let Some(max_concurrency) = limits.max_concurrency {
        // current_concurrency + 1 (this request)
        if envelope.current_concurrency >= max_concurrency {
            result.concurrency_valid = false;
            result.errors.push(ValidationError {
                code: "CONCURRENCY_LIMIT_EXCEEDED".to_string(),
                message: format!(
                    "Concurrency limit exceeded: {} concurrent executions (limit: {})",
                    envelope.current_concurrency, max_concurrency
                ),
                field: Some("envelope.current_concurrency".to_string()),
                severity: 0.7,
            });
        }
    }

    Ok(())
}

/// Validate model restrictions
fn validate_model(
    envelope: &super::types::ExecutionEnvelope,
    limits: &ResourceLimits,
    result: &mut ValidationResult,
) -> AgentResult<()> {
    let model = &envelope.model;

    // Check if model is blocked
    if limits.blocked_models.iter().any(|b| model.contains(b)) {
        result.model_valid = false;
        result.errors.push(ValidationError {
            code: "MODEL_BLOCKED".to_string(),
            message: format!("Model '{}' is blocked by policy", model),
            field: Some("envelope.model".to_string()),
            severity: 0.9,
        });
        return Ok(());
    }

    // Check if model is in allowlist (if configured)
    if !limits.allowed_models.is_empty() {
        let is_allowed = limits.allowed_models.iter().any(|a| model.contains(a));
        if !is_allowed {
            result.model_valid = false;
            result.errors.push(ValidationError {
                code: "MODEL_NOT_ALLOWED".to_string(),
                message: format!(
                    "Model '{}' is not in the allowed models list",
                    model
                ),
                field: Some("envelope.model".to_string()),
                severity: 0.8,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use crate::execution_guard::types::ExecutionEnvelope;

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

    fn create_test_request(envelope: ExecutionEnvelope) -> ExecutionGuardRequest {
        ExecutionGuardRequest {
            envelope,
            context: Some(ExecutionContext::new()),
            limits: None,
            current_rate: Some(10),
        }
    }

    #[test]
    fn test_valid_request() {
        let config = ExecutionGuardConfig::default();
        let request = create_test_request(create_test_envelope());

        let result = validate_execution(&request, &config).unwrap();
        assert!(result.valid);
        assert!(result.tokens_valid);
        assert!(result.cost_valid);
        assert!(result.model_valid);
    }

    #[test]
    fn test_token_limit_exceeded() {
        let config = ExecutionGuardConfig::default();
        let mut envelope = create_test_envelope();
        envelope.input_tokens = 200_000; // Exceeds 100K limit

        let request = create_test_request(envelope);
        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.tokens_valid);
        assert!(result.errors.iter().any(|e| e.code == "INPUT_TOKENS_EXCEEDED"));
    }

    #[test]
    fn test_cost_limit_exceeded() {
        let config = ExecutionGuardConfig::default();
        let mut envelope = create_test_envelope();
        envelope.estimated_cost_micros = Some(20_000_000); // $20, exceeds $10 limit

        let request = create_test_request(envelope);
        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.cost_valid);
        assert!(result.errors.iter().any(|e| e.code == "COST_EXCEEDED"));
    }

    #[test]
    fn test_concurrency_limit() {
        let config = ExecutionGuardConfig::default();
        let mut envelope = create_test_envelope();
        envelope.current_concurrency = 15; // Exceeds default 10

        let request = create_test_request(envelope);
        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.concurrency_valid);
        assert!(result.errors.iter().any(|e| e.code == "CONCURRENCY_LIMIT_EXCEEDED"));
    }

    #[test]
    fn test_model_blocked() {
        let mut config = ExecutionGuardConfig::default();
        config.default_limits.blocked_models = vec!["gpt-4".to_string()];

        let request = create_test_request(create_test_envelope());
        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.model_valid);
        assert!(result.errors.iter().any(|e| e.code == "MODEL_BLOCKED"));
    }

    #[test]
    fn test_rate_limit() {
        let config = ExecutionGuardConfig::default();
        let envelope = create_test_envelope();
        let mut request = create_test_request(envelope);
        request.current_rate = Some(150); // Exceeds default 100

        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.rate_valid);
        assert!(result.errors.iter().any(|e| e.code == "RATE_LIMIT_EXCEEDED"));
    }

    #[test]
    fn test_warnings_at_threshold() {
        let config = ExecutionGuardConfig::default();
        let mut envelope = create_test_envelope();
        envelope.input_tokens = 85_000; // 85% of 100K limit

        let request = create_test_request(envelope);
        let result = validate_execution(&request, &config).unwrap();

        assert!(result.valid); // Still valid
        assert!(result.warnings.iter().any(|w| w.code == "INPUT_TOKENS_HIGH"));
    }

    #[test]
    fn test_strict_mode_fails_on_warnings() {
        let mut config = ExecutionGuardConfig::default();
        config.strict_mode = true;

        let mut envelope = create_test_envelope();
        envelope.input_tokens = 85_000; // 85% triggers warning

        let request = create_test_request(envelope);
        let result = validate_execution(&request, &config).unwrap();

        assert!(!result.valid); // Fails in strict mode
    }
}
