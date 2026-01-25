//! Phase 3 - Automation & Resilience (Layer 1) Integration Tests
//!
//! Tests for Phase 3 startup hardening, performance budgets, signals, and guards.

use llm_edge_agents::phase3::{
    Phase3Config, BudgetEnforcer, PerformanceBudget, BudgetViolation,
    ExecutionRoleGuard, AgentRole, ActionContext,
    Phase3Signal, SignalType, SignalReference, SignalValidationError,
};

// =============================================================================
// Startup Configuration Tests
// =============================================================================

#[test]
fn test_phase3_config_validation_phase() {
    std::env::set_var("AGENT_PHASE", "phase2");
    std::env::set_var("AGENT_LAYER", "layer1");
    std::env::set_var("RUVECTOR_SERVICE_URL", "http://test");
    std::env::set_var("RUVECTOR_API_KEY", "test-key");

    let result = Phase3Config::from_env();
    assert!(result.is_err());

    // Cleanup
    std::env::remove_var("AGENT_PHASE");
    std::env::remove_var("AGENT_LAYER");
    std::env::remove_var("RUVECTOR_SERVICE_URL");
    std::env::remove_var("RUVECTOR_API_KEY");
}

#[test]
fn test_phase3_config_validation_layer() {
    std::env::set_var("AGENT_PHASE", "phase3");
    std::env::set_var("AGENT_LAYER", "layer2");
    std::env::set_var("RUVECTOR_SERVICE_URL", "http://test");
    std::env::set_var("RUVECTOR_API_KEY", "test-key");

    let result = Phase3Config::from_env();
    assert!(result.is_err());

    // Cleanup
    std::env::remove_var("AGENT_PHASE");
    std::env::remove_var("AGENT_LAYER");
    std::env::remove_var("RUVECTOR_SERVICE_URL");
    std::env::remove_var("RUVECTOR_API_KEY");
}

#[test]
fn test_phase3_config_validation_ruvector_required() {
    std::env::set_var("AGENT_PHASE", "phase3");
    std::env::set_var("AGENT_LAYER", "layer1");
    std::env::remove_var("RUVECTOR_SERVICE_URL");
    std::env::remove_var("RUVECTOR_ENDPOINT");
    std::env::set_var("RUVECTOR_API_KEY", "test-key");

    let result = Phase3Config::from_env();
    assert!(result.is_err());

    // Cleanup
    std::env::remove_var("AGENT_PHASE");
    std::env::remove_var("AGENT_LAYER");
    std::env::remove_var("RUVECTOR_API_KEY");
}

#[test]
fn test_phase3_config_validation_api_key_required() {
    std::env::set_var("AGENT_PHASE", "phase3");
    std::env::set_var("AGENT_LAYER", "layer1");
    std::env::set_var("RUVECTOR_SERVICE_URL", "http://test");
    std::env::remove_var("RUVECTOR_API_KEY");

    let result = Phase3Config::from_env();
    assert!(result.is_err());

    // Cleanup
    std::env::remove_var("AGENT_PHASE");
    std::env::remove_var("AGENT_LAYER");
    std::env::remove_var("RUVECTOR_SERVICE_URL");
}

// =============================================================================
// Performance Budget Tests
// =============================================================================

#[test]
fn test_performance_budget_defaults() {
    let budget = PerformanceBudget::phase3_defaults();
    assert_eq!(budget.max_tokens, 1500);
    assert_eq!(budget.max_latency_ms, 3000);
    assert_eq!(budget.max_calls_per_run, 4);
}

#[test]
fn test_token_budget_enforcement() {
    let enforcer = BudgetEnforcer::phase3();

    // Should succeed within budget
    assert!(enforcer.record_tokens(500).is_ok());
    assert!(enforcer.record_tokens(500).is_ok());
    assert!(enforcer.record_tokens(400).is_ok());

    // Should fail over budget (1500 max)
    let result = enforcer.record_tokens(200);
    assert!(matches!(result, Err(BudgetViolation::TokenBudgetExceeded { .. })));
}

#[test]
fn test_call_budget_enforcement() {
    let enforcer = BudgetEnforcer::phase3();

    // Should succeed within budget (4 calls max)
    assert!(enforcer.record_call().is_ok());
    assert!(enforcer.record_call().is_ok());
    assert!(enforcer.record_call().is_ok());
    assert!(enforcer.record_call().is_ok());

    // Should fail on 5th call
    let result = enforcer.record_call();
    assert!(matches!(result, Err(BudgetViolation::CallBudgetExceeded { .. })));
}

#[test]
fn test_budget_usage_tracking() {
    let enforcer = BudgetEnforcer::phase3();
    enforcer.record_tokens(100).unwrap();
    enforcer.record_tokens(200).unwrap();
    enforcer.record_call().unwrap();
    enforcer.record_call().unwrap();

    let usage = enforcer.usage();
    assert_eq!(usage.tokens_used, 300);
    assert_eq!(usage.tokens_max, 1500);
    assert_eq!(usage.calls_made, 2);
    assert_eq!(usage.calls_max, 4);
}

// =============================================================================
// Execution Role Guard Tests
// =============================================================================

#[test]
fn test_allowed_roles() {
    let guard = ExecutionRoleGuard::new("test_agent");

    assert!(guard.is_role_allowed(AgentRole::Coordinate));
    assert!(guard.is_role_allowed(AgentRole::Route));
    assert!(guard.is_role_allowed(AgentRole::Optimize));
    assert!(guard.is_role_allowed(AgentRole::Escalate));
}

#[test]
fn test_coordination_action_allowed() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ExecutionRoleGuard::coordination_action("coordinate failover between providers");

    assert!(guard.validate_action(&action).is_ok());
}

#[test]
fn test_routing_action_allowed() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ExecutionRoleGuard::routing_action("route request to optimal provider");

    assert!(guard.validate_action(&action).is_ok());
}

#[test]
fn test_optimization_action_allowed() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ExecutionRoleGuard::optimization_action("optimize cache strategy");

    assert!(guard.validate_action(&action).is_ok());
}

#[test]
fn test_escalation_action_allowed() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ExecutionRoleGuard::escalation_action(
        "escalate circuit breaker trip",
        "layer2_coordinator"
    );

    assert!(guard.validate_action(&action).is_ok());
}

#[test]
fn test_forbidden_final_decision() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ActionContext {
        role: AgentRole::Coordinate,
        description: "final_decision: approve request".to_string(),
        is_critical: false,
        has_escalation: false,
        is_state_modification: false,
        is_coordinated: true,
        bypasses_escalation: false,
    };

    let result = guard.validate_action(&action);
    assert!(result.is_err());
}

#[test]
fn test_forbidden_executive_conclusion() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ActionContext {
        role: AgentRole::Coordinate,
        description: "executive_conclusion: reject all pending".to_string(),
        is_critical: false,
        has_escalation: false,
        is_state_modification: false,
        is_coordinated: true,
        bypasses_escalation: false,
    };

    let result = guard.validate_action(&action);
    assert!(result.is_err());
}

#[test]
fn test_critical_action_requires_escalation() {
    let guard = ExecutionRoleGuard::new("test_agent");
    let action = ActionContext {
        role: AgentRole::Coordinate,
        description: "critical operation without escalation".to_string(),
        is_critical: true,
        has_escalation: false,
        is_state_modification: false,
        is_coordinated: true,
        bypasses_escalation: false,
    };

    let result = guard.validate_action(&action);
    assert!(result.is_err());
}

// =============================================================================
// Signal Tests
// =============================================================================

#[test]
fn test_execution_strategy_signal() {
    let signal = Phase3Signal::execution_strategy(
        "test_agent",
        "0.1.0",
        "failover_primary",
        "openai_provider",
        "High latency detected",
        0.85,
        vec![SignalReference::metric("latency_p99", "P99 latency exceeded 2000ms")],
    );

    assert_eq!(signal.signal_type, SignalType::ExecutionStrategySignal);
    assert_eq!(signal.confidence, 0.85);
    assert_eq!(signal.phase, "phase3");
    assert_eq!(signal.layer, "layer1");
    assert!(signal.validate().is_ok());
}

#[test]
fn test_optimization_signal() {
    let signal = Phase3Signal::optimization(
        "test_agent",
        "0.1.0",
        "performance",
        "cache_layer",
        "Increase cache TTL for stable responses",
        0.75,
        vec![SignalReference::metric("cache_hit_rate", "Cache hit rate dropped below 70%")],
    );

    assert_eq!(signal.signal_type, SignalType::OptimizationSignal);
    assert_eq!(signal.confidence, 0.75);
    assert!(signal.validate().is_ok());
}

#[test]
fn test_incident_signal() {
    let signal = Phase3Signal::incident(
        "test_agent",
        "0.1.0",
        "high",
        "availability",
        "Primary provider unavailable",
        vec!["openai_adapter".to_string(), "routing_engine".to_string()],
        0.95,
        vec![
            SignalReference::event("health_check", "Health check failed 3 times"),
            SignalReference::log("error_log", "Connection timeout"),
        ],
    );

    assert_eq!(signal.signal_type, SignalType::IncidentSignal);
    assert_eq!(signal.confidence, 0.95);
    assert!(signal.validate().is_ok());
}

#[test]
fn test_signal_requires_references() {
    let signal = Phase3Signal::execution_strategy(
        "test_agent",
        "0.1.0",
        "strategy",
        "target",
        "reason",
        0.8,
        vec![], // Empty references - should fail validation
    );

    let result = signal.validate();
    assert!(matches!(result, Err(SignalValidationError::MissingReferences)));
}

#[test]
fn test_signal_confidence_bounds() {
    let signal = Phase3Signal::execution_strategy(
        "test_agent",
        "0.1.0",
        "strategy",
        "target",
        "reason",
        1.5, // Invalid confidence > 1.0
        vec![SignalReference::metric("test", "test")],
    );

    let result = signal.validate();
    assert!(matches!(result, Err(SignalValidationError::InvalidConfidence(_))));
}

#[test]
fn test_signal_reference_types() {
    let metric_ref = SignalReference::metric("latency_p99", "P99 latency metric");
    assert_eq!(metric_ref.ref_type, "metric");

    let event_ref = SignalReference::event("startup", "Service startup event");
    assert_eq!(event_ref.ref_type, "event");

    let log_ref = SignalReference::log("error", "Error log entry");
    assert_eq!(log_ref.ref_type, "log");

    let external_ref = SignalReference::external("doc", "External documentation", "https://docs.example.com");
    assert_eq!(external_ref.ref_type, "external");
    assert!(external_ref.url.is_some());
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_phase3_full_workflow() {
    // 1. Create execution role guard
    let guard = ExecutionRoleGuard::new("failover_agent");

    // 2. Validate coordination action
    let action = ExecutionRoleGuard::routing_action("route to backup provider");
    assert!(guard.validate_action(&action).is_ok());

    // 3. Create and validate signal
    let signal = Phase3Signal::execution_strategy(
        "failover_agent",
        "0.1.0",
        "failover_to_backup",
        "anthropic_provider",
        "Primary provider circuit breaker open",
        0.90,
        vec![
            SignalReference::event("circuit_breaker", "Circuit breaker tripped"),
            SignalReference::metric("error_rate", "Error rate exceeded 50%"),
        ],
    );

    assert!(signal.validate().is_ok());

    // 4. Track budget
    let enforcer = BudgetEnforcer::phase3();
    assert!(enforcer.record_call().is_ok());
    assert!(enforcer.record_tokens(500).is_ok());

    let usage = enforcer.usage();
    assert_eq!(usage.calls_made, 1);
    assert_eq!(usage.tokens_used, 500);
}
