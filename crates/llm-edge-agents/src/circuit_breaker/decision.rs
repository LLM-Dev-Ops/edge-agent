//! Circuit Breaker Decision Engine
//!
//! Core decision logic for the circuit breaker agent.
//! This module contains the deterministic, threshold-based decision logic.

use chrono::Utc;
use tracing::{debug, info, warn};

use super::types::*;

/// Circuit Breaker Decision Engine
///
/// Implements the core decision logic for circuit breaker operations.
/// All decisions are deterministic given the same inputs (confidence = 1.0).
pub struct CircuitBreakerDecisionEngine;

impl CircuitBreakerDecisionEngine {
    /// Evaluate circuit breaker state and make a decision
    ///
    /// This is the core decision function. It is:
    /// - Deterministic: same inputs always produce same output
    /// - Threshold-based: uses configured thresholds for decisions
    /// - Stateless: all state is passed in, no internal state
    pub fn evaluate(request: &CircuitBreakerRequest) -> CircuitBreakerResponse {
        let start = std::time::Instant::now();

        // Start with current values
        let mut new_state = request.current_state;
        let mut new_failure_count = request.failure_count;
        let mut new_success_count = request.success_count;
        let mut new_last_failure_time = request.last_failure_time;
        let mut state_transition: Option<StateTransition> = None;

        // Process failure signal if present
        if let Some(ref failure) = request.failure_signal {
            debug!(
                provider_id = %request.provider_id,
                failure_type = %failure.failure_type,
                "Processing failure signal"
            );

            new_failure_count += 1;
            new_success_count = 0; // Reset success count on failure
            new_last_failure_time = Some(failure.timestamp);
        }

        // Process success signal if present
        if let Some(ref _success) = request.success_signal {
            debug!(
                provider_id = %request.provider_id,
                "Processing success signal"
            );

            new_success_count += 1;
            // Don't reset failure count on success - it resets when circuit closes
        }

        // Evaluate state transitions
        match request.current_state {
            CircuitState::Closed => {
                // Check if we should open the circuit
                if new_failure_count >= request.config.failure_threshold {
                    info!(
                        provider_id = %request.provider_id,
                        failure_count = new_failure_count,
                        threshold = request.config.failure_threshold,
                        "Circuit opening due to failure threshold"
                    );

                    state_transition = Some(StateTransition {
                        from: CircuitState::Closed,
                        to: CircuitState::Open,
                        trigger: format!(
                            "Failure threshold exceeded: {} >= {}",
                            new_failure_count, request.config.failure_threshold
                        ),
                        timestamp: Utc::now(),
                    });
                    new_state = CircuitState::Open;
                }
            }

            CircuitState::Open => {
                // Check if timeout has elapsed -> transition to half-open
                if let Some(last_failure) = request.last_failure_time {
                    let elapsed = (request.timestamp - last_failure).num_seconds();
                    if elapsed >= request.config.timeout_seconds as i64 {
                        info!(
                            provider_id = %request.provider_id,
                            elapsed_seconds = elapsed,
                            timeout_seconds = request.config.timeout_seconds,
                            "Circuit transitioning to half-open after timeout"
                        );

                        state_transition = Some(StateTransition {
                            from: CircuitState::Open,
                            to: CircuitState::HalfOpen,
                            trigger: format!(
                                "Timeout elapsed: {} >= {} seconds",
                                elapsed, request.config.timeout_seconds
                            ),
                            timestamp: Utc::now(),
                        });
                        new_state = CircuitState::HalfOpen;
                        new_success_count = 0; // Reset for half-open testing
                    }
                }
            }

            CircuitState::HalfOpen => {
                // Check for success -> close circuit
                if new_success_count >= request.config.success_threshold {
                    info!(
                        provider_id = %request.provider_id,
                        success_count = new_success_count,
                        threshold = request.config.success_threshold,
                        "Circuit closing due to success threshold"
                    );

                    state_transition = Some(StateTransition {
                        from: CircuitState::HalfOpen,
                        to: CircuitState::Closed,
                        trigger: format!(
                            "Success threshold reached: {} >= {}",
                            new_success_count, request.config.success_threshold
                        ),
                        timestamp: Utc::now(),
                    });
                    new_state = CircuitState::Closed;
                    new_failure_count = 0; // Reset on circuit close
                    new_success_count = 0;
                }
                // Check for failure -> reopen circuit
                else if request.failure_signal.is_some() {
                    warn!(
                        provider_id = %request.provider_id,
                        "Circuit reopening due to failure in half-open state"
                    );

                    state_transition = Some(StateTransition {
                        from: CircuitState::HalfOpen,
                        to: CircuitState::Open,
                        trigger: "Failure during half-open probe".to_string(),
                        timestamp: Utc::now(),
                    });
                    new_state = CircuitState::Open;
                }
            }
        }

        // Make decision based on new state
        let (decision, reason) = match new_state {
            CircuitState::Closed => (
                CircuitBreakerDecision::Allow,
                format!(
                    "Circuit closed: {} failures (threshold: {})",
                    new_failure_count, request.config.failure_threshold
                ),
            ),
            CircuitState::Open => (
                CircuitBreakerDecision::Block,
                format!(
                    "Circuit open: blocking requests until timeout ({} seconds)",
                    request.config.timeout_seconds
                ),
            ),
            CircuitState::HalfOpen => (
                CircuitBreakerDecision::AllowWithProbe,
                format!(
                    "Circuit half-open: probe request {} of {}",
                    new_success_count + 1, request.config.success_threshold
                ),
            ),
        };

        // Calculate retry-after for blocked requests
        let retry_after_seconds = if new_state == CircuitState::Open {
            if let Some(last_failure) = new_last_failure_time {
                let elapsed = (request.timestamp - last_failure).num_seconds();
                let remaining = request.config.timeout_seconds as i64 - elapsed;
                if remaining > 0 {
                    Some(remaining as u32)
                } else {
                    Some(0)
                }
            } else {
                Some(request.config.timeout_seconds)
            }
        } else {
            None
        };

        let duration_us = start.elapsed().as_micros() as u64;

        debug!(
            provider_id = %request.provider_id,
            decision = %decision,
            new_state = %new_state,
            duration_us = duration_us,
            "Circuit breaker decision made"
        );

        CircuitBreakerResponse {
            decision,
            new_state,
            new_failure_count,
            new_success_count,
            new_last_failure_time,
            retry_after_seconds,
            confidence: 1.0, // Deterministic, threshold-based
            reason,
            state_transition,
            duration_us,
        }
    }

    /// Validate input against contract requirements
    pub fn validate_input(request: &CircuitBreakerRequest) -> Result<(), String> {
        if request.execution_ref.is_empty() {
            return Err("execution_ref is required".to_string());
        }
        if request.provider_id.is_empty() {
            return Err("provider_id is required".to_string());
        }
        if request.config.failure_threshold == 0 {
            return Err("failure_threshold must be > 0".to_string());
        }
        if request.config.success_threshold == 0 {
            return Err("success_threshold must be > 0".to_string());
        }
        if request.config.timeout_seconds == 0 {
            return Err("timeout_seconds must be > 0".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closed_circuit_allows_requests() {
        let request = CircuitBreakerRequest::check("openai", "exec-123")
            .with_state(CircuitState::Closed, 2, 0);

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.decision, CircuitBreakerDecision::Allow);
        assert_eq!(response.new_state, CircuitState::Closed);
        assert_eq!(response.confidence, 1.0);
    }

    #[test]
    fn test_failure_threshold_opens_circuit() {
        let signal = FailureSignal::new("openai", "timeout");
        let request = CircuitBreakerRequest::record_failure("openai", "exec-123", signal)
            .with_state(CircuitState::Closed, 4, 0)
            .with_config(CircuitBreakerConfig {
                provider_id: "openai".to_string(),
                failure_threshold: 5,
                success_threshold: 3,
                timeout_seconds: 30,
                ..Default::default()
            });

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.new_state, CircuitState::Open);
        assert_eq!(response.new_failure_count, 5);
        assert!(response.state_transition.is_some());
    }

    #[test]
    fn test_open_circuit_blocks_requests() {
        let now = Utc::now();
        let recent_failure = now - chrono::Duration::seconds(10);

        let mut request = CircuitBreakerRequest::check("openai", "exec-123")
            .with_state(CircuitState::Open, 5, 0)
            .with_config(CircuitBreakerConfig {
                provider_id: "openai".to_string(),
                failure_threshold: 5,
                success_threshold: 3,
                timeout_seconds: 30,
                ..Default::default()
            });
        request.last_failure_time = Some(recent_failure);

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.decision, CircuitBreakerDecision::Block);
        assert_eq!(response.new_state, CircuitState::Open);
        assert!(response.retry_after_seconds.is_some());
    }

    #[test]
    fn test_timeout_transitions_to_half_open() {
        let now = Utc::now();
        let old_failure = now - chrono::Duration::seconds(60);

        let mut request = CircuitBreakerRequest::check("openai", "exec-123")
            .with_state(CircuitState::Open, 5, 0)
            .with_config(CircuitBreakerConfig {
                provider_id: "openai".to_string(),
                failure_threshold: 5,
                success_threshold: 3,
                timeout_seconds: 30,
                ..Default::default()
            });
        request.last_failure_time = Some(old_failure);

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.new_state, CircuitState::HalfOpen);
        assert_eq!(response.decision, CircuitBreakerDecision::AllowWithProbe);
    }

    #[test]
    fn test_half_open_success_closes_circuit() {
        let signal = SuccessSignal::new("openai");
        let request = CircuitBreakerRequest::record_success("openai", "exec-123", signal)
            .with_state(CircuitState::HalfOpen, 5, 2)
            .with_config(CircuitBreakerConfig {
                provider_id: "openai".to_string(),
                failure_threshold: 5,
                success_threshold: 3,
                timeout_seconds: 30,
                ..Default::default()
            });

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.new_state, CircuitState::Closed);
        assert_eq!(response.decision, CircuitBreakerDecision::Allow);
        assert_eq!(response.new_failure_count, 0); // Reset on close
    }

    #[test]
    fn test_half_open_failure_reopens_circuit() {
        let signal = FailureSignal::new("openai", "error");
        let request = CircuitBreakerRequest::record_failure("openai", "exec-123", signal)
            .with_state(CircuitState::HalfOpen, 5, 1);

        let response = CircuitBreakerDecisionEngine::evaluate(&request);

        assert_eq!(response.new_state, CircuitState::Open);
        assert_eq!(response.decision, CircuitBreakerDecision::Block);
    }

    #[test]
    fn test_input_validation() {
        let mut request = CircuitBreakerRequest::check("openai", "exec-123");
        assert!(CircuitBreakerDecisionEngine::validate_input(&request).is_ok());

        request.execution_ref = String::new();
        assert!(CircuitBreakerDecisionEngine::validate_input(&request).is_err());
    }
}
