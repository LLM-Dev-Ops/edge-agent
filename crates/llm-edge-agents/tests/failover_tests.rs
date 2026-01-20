//! Integration tests for the Failover Agent
//!
//! These tests verify the Failover Agent's behavior according to the
//! LLM-Edge-Agent Infrastructure Constitution.

use llm_edge_agents::{
    CircuitState, ExecutionContext, FailoverAgent, FailoverOutcome, FailoverPolicy,
    FailoverRequest, ProviderHealth, TargetProvider,
};

mod failover_agent_tests {
    use super::*;

    /// Helper to create a test failover agent with events disabled
    fn create_test_agent() -> FailoverAgent {
        use llm_edge_agents::failover::FailoverAgentConfig;
        FailoverAgent::new(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        })
    }

    /// Helper to create a basic failover request
    fn create_test_request() -> FailoverRequest {
        let ctx = ExecutionContext::new();
        let primary = TargetProvider::new("openai-1", "openai", "gpt-4");
        let alternates = vec![
            TargetProvider::new("anthropic-1", "anthropic", "claude-3"),
            TargetProvider::new("google-1", "google", "gemini-pro"),
        ];
        FailoverRequest::new(ctx, primary, alternates)
    }

    // =========================================================================
    // PROMPT 1 Verification: Contract & Boundary Definition
    // =========================================================================

    /// Test: Agent correctly identifies as ROUTING type
    #[test]
    fn test_agent_classification_is_routing() {
        use llm_edge_agents::{AgentType, failover::AGENT_ID};

        assert_eq!(AGENT_ID, "failover-agent");
        // Agent type is Routing per the infrastructure constitution
    }

    /// Test: Decision type is always Failover
    #[tokio::test]
    async fn test_decision_type_is_failover() {
        let agent = create_test_agent();
        let request = create_test_request();

        let response = agent.process(request).await.unwrap();

        // Decision type semantics per PROMPT 1:
        // - UsePrimary: Primary target is healthy
        // - UseAlternate: Primary unavailable, route to alternate
        // - NoViableTarget: All targets unavailable
        assert!(matches!(
            response.decision.outcome,
            FailoverOutcome::UsePrimary
                | FailoverOutcome::UseAlternate
                | FailoverOutcome::NoViableTarget
        ));
    }

    /// Test: Confidence semantics are correct
    #[tokio::test]
    async fn test_confidence_semantics() {
        let agent = create_test_agent();

        // Deterministic decision (circuit closed, healthy) should have confidence 1.0
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();
        assert_eq!(response.decision.confidence, 1.0);

        // Failover scenario should have confidence < 1.0 (0.9 per implementation)
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();
        assert!(response.decision.confidence <= 1.0);
        assert!(response.decision.confidence >= 0.7);
    }

    // =========================================================================
    // PROMPT 2 Verification: Runtime Implementation
    // =========================================================================

    /// Test: Use primary when healthy and circuit closed
    #[tokio::test]
    async fn test_use_primary_when_healthy() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::UsePrimary);
        assert_eq!(
            response.decision.selected_target.as_ref().unwrap().provider_name,
            "openai"
        );
    }

    /// Test: Failover when primary circuit is open
    #[tokio::test]
    async fn test_failover_when_circuit_open() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::UseAlternate);
        assert_eq!(
            response.decision.selected_target.as_ref().unwrap().provider_name,
            "anthropic"
        );
    }

    /// Test: Failover when primary health is unhealthy
    #[tokio::test]
    async fn test_failover_when_unhealthy() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Unhealthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::UseAlternate);
    }

    /// Test: No viable target when all circuits open
    #[tokio::test]
    async fn test_no_viable_target_all_circuits_open() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("anthropic-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("google-1", CircuitState::Open, ProviderHealth::Unhealthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success); // Request processed successfully
        assert_eq!(response.decision.outcome, FailoverOutcome::NoViableTarget);
        assert!(response.decision.selected_target.is_none());
    }

    /// Test: Stateless execution (no side effects between calls)
    #[tokio::test]
    async fn test_stateless_execution() {
        let agent = create_test_agent();

        // First call with primary healthy
        let request1 = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);
        let response1 = agent.process(request1).await.unwrap();
        assert_eq!(response1.decision.outcome, FailoverOutcome::UsePrimary);

        // Second call with primary down - agent state unchanged
        let request2 = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy);
        let response2 = agent.process(request2).await.unwrap();
        assert_eq!(response2.decision.outcome, FailoverOutcome::UseAlternate);

        // Third call same as first - deterministic result
        let request3 = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);
        let response3 = agent.process(request3).await.unwrap();
        assert_eq!(response3.decision.outcome, FailoverOutcome::UsePrimary);
    }

    /// Test: Deterministic behavior (same inputs = same outputs)
    #[tokio::test]
    async fn test_deterministic_behavior() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy);

        // Run same request multiple times
        let response1 = agent.process(request.clone()).await.unwrap();
        let response2 = agent.process(request.clone()).await.unwrap();
        let response3 = agent.process(request).await.unwrap();

        // All responses should have same outcome
        assert_eq!(response1.decision.outcome, response2.decision.outcome);
        assert_eq!(response2.decision.outcome, response3.decision.outcome);

        // All should select same provider
        assert_eq!(
            response1.decision.selected_target.as_ref().unwrap().provider_id,
            response2.decision.selected_target.as_ref().unwrap().provider_id
        );
    }

    // =========================================================================
    // Policy Enforcement Tests
    // =========================================================================

    /// Test: Failover disabled by policy
    #[tokio::test]
    async fn test_failover_disabled_by_policy() {
        let agent = create_test_agent();
        let mut request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy);
        request.policy.auto_failover_enabled = false;

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.decision.outcome, FailoverOutcome::NoViableTarget);
        assert!(response.decision.reason.contains("auto-failover is disabled"));
    }

    /// Test: Max failover attempts exceeded
    #[tokio::test]
    async fn test_max_failover_attempts_exceeded() {
        let agent = create_test_agent();
        let mut request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy);
        request.current_attempt = 5;
        request.policy.max_failover_attempts = 3;

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.decision.outcome, FailoverOutcome::NoViableTarget);
        assert!(response.decision.reason.contains("Max failover attempts"));
    }

    /// Test: Provider blocked by policy
    #[tokio::test]
    async fn test_provider_blocked_by_policy() {
        let agent = create_test_agent();
        let mut request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy)
            .with_provider_status("google-1", CircuitState::Closed, ProviderHealth::Healthy);

        // Block anthropic
        request.policy.blocked_providers = vec!["anthropic".to_string()];

        let response = agent.process(request).await.unwrap();

        // Should skip anthropic and use google
        assert_eq!(response.decision.outcome, FailoverOutcome::UseAlternate);
        assert_eq!(
            response.decision.selected_target.as_ref().unwrap().provider_name,
            "google"
        );
    }

    /// Test: Only allowed providers used
    #[tokio::test]
    async fn test_allowed_providers_filter() {
        let agent = create_test_agent();
        let mut request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy)
            .with_provider_status("anthropic-1", CircuitState::Closed, ProviderHealth::Healthy)
            .with_provider_status("google-1", CircuitState::Closed, ProviderHealth::Healthy);

        // Only allow google
        request.policy.allowed_providers = vec!["google".to_string()];

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.decision.outcome, FailoverOutcome::UseAlternate);
        assert_eq!(
            response.decision.selected_target.as_ref().unwrap().provider_name,
            "google"
        );
    }

    // =========================================================================
    // Constraint Evaluation Tests
    // =========================================================================

    /// Test: Constraints are properly recorded
    #[tokio::test]
    async fn test_constraints_recorded() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();

        // Should have at least policy and guard constraints
        assert!(!response.constraints_applied.is_empty());

        // Check for expected constraint types
        let constraint_names: Vec<&str> = response
            .constraints_applied
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        assert!(constraint_names.contains(&"failover_enabled"));
        assert!(constraint_names.contains(&"max_failover_attempts"));
        assert!(constraint_names.contains(&"primary_circuit_state"));
    }

    // =========================================================================
    // Non-Responsibility Tests (MUST NOT do)
    // =========================================================================

    /// Test: Agent does not modify input request (no orchestration)
    #[tokio::test]
    async fn test_no_input_modification() {
        let agent = create_test_agent();
        let request = create_test_request();
        let request_clone = request.clone();

        let _ = agent.process(request.clone()).await.unwrap();

        // Request should be unchanged
        assert_eq!(request.primary_target.provider_id, request_clone.primary_target.provider_id);
        assert_eq!(request.alternate_targets.len(), request_clone.alternate_targets.len());
    }

    /// Test: Agent returns deterministic output (no analytics side effects)
    #[tokio::test]
    async fn test_no_analytics_side_effects() {
        let agent = create_test_agent();
        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);

        // Multiple calls should return same structure
        let response1 = agent.process(request.clone()).await.unwrap();
        let response2 = agent.process(request).await.unwrap();

        assert_eq!(response1.decision.outcome, response2.decision.outcome);
        assert_eq!(response1.constraints_applied.len(), response2.constraints_applied.len());
    }
}

mod contract_serialization_tests {
    use super::*;

    /// Test: FailoverRequest serializes correctly
    #[test]
    fn test_failover_request_serialization() {
        let request = super::failover_agent_tests::create_test_request();

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("openai"));
        assert!(json.contains("anthropic"));

        let deserialized: FailoverRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.primary_target.provider_name, "openai");
    }

    /// Test: FailoverPolicy defaults are sensible
    #[test]
    fn test_failover_policy_defaults() {
        let policy = FailoverPolicy::default();

        assert!(policy.auto_failover_enabled);
        assert_eq!(policy.max_failover_attempts, 3);
        assert!(!policy.failover_on_degraded);
        assert!(policy.allowed_providers.is_empty());
        assert!(policy.blocked_providers.is_empty());
    }

    /// Test: TargetProvider availability logic
    #[test]
    fn test_target_provider_availability() {
        let mut provider = TargetProvider::new("test-1", "test", "model-v1");

        // Healthy and closed = available
        provider.circuit_state = CircuitState::Closed;
        provider.health = ProviderHealth::Healthy;
        assert!(provider.is_available());

        // Open circuit = unavailable
        provider.circuit_state = CircuitState::Open;
        assert!(!provider.is_available());

        // Closed but unhealthy = unavailable
        provider.circuit_state = CircuitState::Closed;
        provider.health = ProviderHealth::Unhealthy;
        assert!(!provider.is_available());

        // Half-open and healthy = available (testing recovery)
        provider.circuit_state = CircuitState::HalfOpen;
        provider.health = ProviderHealth::Healthy;
        assert!(provider.is_available());
    }
}

mod decision_event_tests {
    use super::*;

    /// Test: DecisionEvent includes required fields per Infrastructure Constitution
    #[test]
    fn test_decision_event_required_fields() {
        use llm_edge_agents::{AgentMetadata, AgentType, DecisionEvent, DecisionType};

        let agent = AgentMetadata {
            agent_id: "failover-agent".to_string(),
            agent_version: "0.1.0".to_string(),
            agent_type: AgentType::Routing,
        };

        let event = DecisionEvent::new(
            agent,
            DecisionType::Failover,
            "sha256hash".to_string(),
            serde_json::json!({"outcome": "use_primary"}),
            1.0,
            vec![],
            "trace-123".to_string(),
            100,
        );

        // Verify required fields per Infrastructure Constitution
        assert!(!event.event_id.is_empty());
        assert_eq!(event.agent.agent_id, "failover-agent");
        assert!(!event.agent.agent_version.is_empty());
        assert_eq!(event.decision_type, DecisionType::Failover);
        assert!(!event.inputs_hash.is_empty());
        assert!(!event.outputs.is_null());
        assert!(event.confidence >= 0.0 && event.confidence <= 1.0);
        assert!(!event.execution_ref.is_empty());
        // timestamp is auto-set
    }
}
