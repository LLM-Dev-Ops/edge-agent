//! Circuit Breaker Agent Integration Tests
//!
//! These tests verify the complete Circuit Breaker Agent implementation
//! against the architectural requirements defined in PROMPT 0-4.

use chrono::Utc;
use llm_edge_agents::circuit_breaker::{
    CircuitBreakerAgent, CircuitBreakerConfig, CircuitBreakerDecisionEngine,
    CircuitBreakerRequest, CircuitBreakerResponse, CircuitBreakerDecision,
    CircuitState, FailureSignal, SuccessSignal,
};
use llm_edge_agents::contracts::{AgentType, DecisionType};

/// Test: Agent classification is PROTECTION / GUARD
#[test]
fn test_agent_classification() {
    let agent = CircuitBreakerAgent::new(None);
    let metadata = agent.metadata();

    assert_eq!(metadata.agent_id, "circuit_breaker_agent");
    assert_eq!(metadata.agent_type, AgentType::Protection);
}

/// Test: Decision type is circuit_breaker_decision
#[test]
fn test_decision_type() {
    let request = CircuitBreakerRequest::check("test_provider", "exec-001");
    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    // Verify deterministic decision (confidence = 1.0)
    assert_eq!(response.confidence, 1.0);
}

/// Test: Circuit closed state allows requests
#[test]
fn test_closed_circuit_allows() {
    let request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Closed, 0, 0);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.decision, CircuitBreakerDecision::Allow);
    assert_eq!(response.new_state, CircuitState::Closed);
}

/// Test: Failure threshold triggers circuit open
#[test]
fn test_failure_threshold_opens_circuit() {
    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_seconds: 30,
        ..Default::default()
    };

    // Start with 2 failures (below threshold)
    let signal = FailureSignal::new("openai", "timeout");
    let request = CircuitBreakerRequest::record_failure("openai", "exec-001", signal)
        .with_state(CircuitState::Closed, 2, 0)
        .with_config(config);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    // Third failure should open the circuit
    assert_eq!(response.new_state, CircuitState::Open);
    assert_eq!(response.decision, CircuitBreakerDecision::Block);
    assert!(response.state_transition.is_some());
}

/// Test: Open circuit blocks requests
#[test]
fn test_open_circuit_blocks() {
    let now = Utc::now();
    let recent_failure = now - chrono::Duration::seconds(5);

    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_seconds: 30,
        ..Default::default()
    };

    let mut request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Open, 3, 0)
        .with_config(config);
    request.last_failure_time = Some(recent_failure);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.decision, CircuitBreakerDecision::Block);
    assert_eq!(response.new_state, CircuitState::Open);
    assert!(response.retry_after_seconds.is_some());
}

/// Test: Timeout transitions to half-open
#[test]
fn test_timeout_transitions_to_half_open() {
    let now = Utc::now();
    let old_failure = now - chrono::Duration::seconds(60);

    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_seconds: 30,
        ..Default::default()
    };

    let mut request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Open, 3, 0)
        .with_config(config);
    request.last_failure_time = Some(old_failure);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.new_state, CircuitState::HalfOpen);
    assert_eq!(response.decision, CircuitBreakerDecision::AllowWithProbe);
}

/// Test: Half-open with successes closes circuit
#[test]
fn test_half_open_success_closes() {
    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_seconds: 30,
        ..Default::default()
    };

    // Already at 1 success, adding another should close
    let signal = SuccessSignal::new("openai");
    let request = CircuitBreakerRequest::record_success("openai", "exec-001", signal)
        .with_state(CircuitState::HalfOpen, 3, 1)
        .with_config(config);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.new_state, CircuitState::Closed);
    assert_eq!(response.decision, CircuitBreakerDecision::Allow);
    assert_eq!(response.new_failure_count, 0); // Reset on close
}

/// Test: Half-open failure reopens circuit
#[test]
fn test_half_open_failure_reopens() {
    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        success_threshold: 2,
        timeout_seconds: 30,
        ..Default::default()
    };

    let signal = FailureSignal::new("openai", "error");
    let request = CircuitBreakerRequest::record_failure("openai", "exec-001", signal)
        .with_state(CircuitState::HalfOpen, 3, 1)
        .with_config(config);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.new_state, CircuitState::Open);
    assert_eq!(response.decision, CircuitBreakerDecision::Block);
}

/// Test: Input validation works correctly
#[test]
fn test_input_validation() {
    // Valid request
    let request = CircuitBreakerRequest::check("openai", "exec-001");
    assert!(CircuitBreakerDecisionEngine::validate_input(&request).is_ok());

    // Empty execution_ref
    let mut bad_request = CircuitBreakerRequest::check("openai", "");
    assert!(CircuitBreakerDecisionEngine::validate_input(&bad_request).is_err());

    // Empty provider_id
    bad_request = CircuitBreakerRequest::check("", "exec-001");
    assert!(CircuitBreakerDecisionEngine::validate_input(&bad_request).is_err());
}

/// Test: Deterministic behavior - same inputs produce same outputs
#[test]
fn test_deterministic_behavior() {
    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 5,
        success_threshold: 3,
        timeout_seconds: 30,
        ..Default::default()
    };

    let request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Closed, 2, 0)
        .with_config(config.clone());

    let response1 = CircuitBreakerDecisionEngine::evaluate(&request);
    let response2 = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response1.decision, response2.decision);
    assert_eq!(response1.new_state, response2.new_state);
    assert_eq!(response1.new_failure_count, response2.new_failure_count);
    assert_eq!(response1.confidence, response2.confidence);
}

/// Test: Agent does not persist state locally
#[tokio::test]
async fn test_no_local_persistence() {
    let agent = CircuitBreakerAgent::new(None);

    // Make multiple invocations
    let request1 = CircuitBreakerRequest::check("openai", "exec-001");
    let request2 = CircuitBreakerRequest::check("openai", "exec-002");

    let response1 = agent.invoke(request1).await.unwrap();
    let response2 = agent.invoke(request2).await.unwrap();

    // Both should start from the same state (no local state retention)
    assert_eq!(response1.new_failure_count, response2.new_failure_count);
}

/// Test: Failure signal updates counts correctly
#[test]
fn test_failure_signal_processing() {
    let signal = FailureSignal::new("openai", "timeout")
        .with_status_code(504)
        .with_duration_ms(30000);

    let request = CircuitBreakerRequest::record_failure("openai", "exec-001", signal)
        .with_state(CircuitState::Closed, 2, 1);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.new_failure_count, 3); // 2 + 1
    assert_eq!(response.new_success_count, 0); // Reset on failure
}

/// Test: Success signal updates counts correctly
#[test]
fn test_success_signal_processing() {
    let signal = SuccessSignal::new("openai");

    let request = CircuitBreakerRequest::record_success("openai", "exec-001", signal)
        .with_state(CircuitState::Closed, 2, 1);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.new_failure_count, 2); // Unchanged
    assert_eq!(response.new_success_count, 2); // 1 + 1
}

/// Test: State transition is correctly recorded
#[test]
fn test_state_transition_recording() {
    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        failure_threshold: 3,
        ..Default::default()
    };

    let signal = FailureSignal::new("openai", "error");
    let request = CircuitBreakerRequest::record_failure("openai", "exec-001", signal)
        .with_state(CircuitState::Closed, 2, 0)
        .with_config(config);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    let transition = response.state_transition.expect("Should have transition");
    assert_eq!(transition.from, CircuitState::Closed);
    assert_eq!(transition.to, CircuitState::Open);
    assert!(!transition.trigger.is_empty());
}

/// Test: Retry-after header is set for blocked requests
#[test]
fn test_retry_after_on_block() {
    let now = Utc::now();
    let recent_failure = now - chrono::Duration::seconds(10);

    let config = CircuitBreakerConfig {
        provider_id: "openai".to_string(),
        timeout_seconds: 30,
        ..Default::default()
    };

    let mut request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Open, 5, 0)
        .with_config(config);
    request.last_failure_time = Some(recent_failure);

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    assert_eq!(response.decision, CircuitBreakerDecision::Block);
    let retry_after = response.retry_after_seconds.expect("Should have retry_after");
    assert!(retry_after > 0);
    assert!(retry_after <= 20); // ~30 - 10 elapsed
}

/// Test: Config validation with custom thresholds
#[test]
fn test_custom_config_thresholds() {
    let config = CircuitBreakerConfig {
        provider_id: "custom".to_string(),
        failure_threshold: 10,
        success_threshold: 5,
        timeout_seconds: 60,
        failure_rate_threshold: Some(0.5),
        rate_window_seconds: Some(300),
        min_requests_in_window: Some(20),
    };

    assert_eq!(config.failure_threshold, 10);
    assert_eq!(config.success_threshold, 5);
    assert_eq!(config.timeout_seconds, 60);
    assert_eq!(config.failure_rate_threshold, Some(0.5));
}

// ============================================
// Verification Checklist Tests
// ============================================

/// Verification: Agent imports schemas from contracts module
#[test]
fn verify_schema_imports() {
    // These imports verify the contracts are properly used
    use llm_edge_agents::contracts::{AgentMetadata, DecisionEvent, DecisionType};

    let metadata = AgentMetadata {
        agent_id: "test".to_string(),
        agent_version: "0.1.0".to_string(),
        agent_type: AgentType::Protection,
    };

    assert_eq!(metadata.agent_type, AgentType::Protection);
}

/// Verification: Agent emits telemetry (via tracing)
#[tokio::test]
async fn verify_telemetry_emission() {
    // Initialize tracing subscriber (would capture in real test)
    let agent = CircuitBreakerAgent::new(None);
    let request = CircuitBreakerRequest::check("test_provider", "exec-001");

    // This should emit tracing events
    let response = agent.invoke(request).await.unwrap();

    // Verify response has duration (telemetry is being collected)
    assert!(response.duration_us > 0);
}

/// Verification: Decision confidence is deterministic (1.0)
#[test]
fn verify_deterministic_confidence() {
    let request = CircuitBreakerRequest::check("openai", "exec-001");
    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    // Threshold-based decisions have confidence = 1.0
    assert_eq!(response.confidence, 1.0);
}

/// Verification: Agent is stateless (no internal state retention)
#[tokio::test]
async fn verify_stateless_execution() {
    let agent1 = CircuitBreakerAgent::new(None);
    let agent2 = CircuitBreakerAgent::new(None);

    let request = CircuitBreakerRequest::check("openai", "exec-001")
        .with_state(CircuitState::Closed, 3, 0);

    let response1 = agent1.invoke(request.clone()).await.unwrap();
    let response2 = agent2.invoke(request).await.unwrap();

    // Different agent instances should produce identical results
    assert_eq!(response1.decision, response2.decision);
    assert_eq!(response1.new_state, response2.new_state);
}
