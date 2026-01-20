//! Failover Agent core implementation
//!
//! Google Cloud Edge Function handler for failover routing decisions.
//! Stateless execution with deterministic behavior.

use crate::{
    AgentError, AgentMetadata, AgentResult, AgentType, Constraint, ConstraintType, DecisionEvent,
    DecisionType,
};
use sha2::{Digest, Sha256};
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

use super::{
    CircuitState, EvaluatedTarget, FailoverDecision, FailoverOutcome, FailoverPolicy,
    FailoverRequest, FailoverResponse, ProviderHealth, RuVectorClient, TargetProvider,
    AGENT_ID, AGENT_VERSION,
};

/// Failover Agent configuration
#[derive(Debug, Clone)]
pub struct FailoverAgentConfig {
    /// ruvector-service endpoint URL
    pub ruvector_url: String,

    /// Enable event emission to ruvector-service
    pub emit_events: bool,

    /// Default policy when none provided
    pub default_policy: FailoverPolicy,
}

impl Default for FailoverAgentConfig {
    fn default() -> Self {
        Self {
            ruvector_url: "http://localhost:8080".to_string(),
            emit_events: true,
            default_policy: FailoverPolicy::default(),
        }
    }
}

/// Failover Agent - ROUTING classification
///
/// Routes execution to alternate backends when primary targets are unavailable.
/// Deployed as Google Cloud Edge Function.
pub struct FailoverAgent {
    config: FailoverAgentConfig,
    ruvector_client: RuVectorClient,
}

impl FailoverAgent {
    /// Create a new Failover Agent
    pub fn new(config: FailoverAgentConfig) -> Self {
        let ruvector_client = RuVectorClient::new(&config.ruvector_url);
        Self {
            config,
            ruvector_client,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(FailoverAgentConfig::default())
    }

    /// Get reference to the ruvector client
    pub fn ruvector_client(&self) -> &RuVectorClient {
        &self.ruvector_client
    }

    /// Process a failover routing decision
    ///
    /// This is the main entry point for the agent.
    /// Returns a deterministic routing decision based on input state.
    #[instrument(
        skip(self, request),
        fields(
            agent_id = AGENT_ID,
            execution_ref = %request.execution_context.execution_ref,
            primary = %request.primary_target.provider_id
        )
    )]
    pub async fn process(&self, request: FailoverRequest) -> AgentResult<FailoverResponse> {
        let start = Instant::now();
        info!("Processing failover request");

        // Validate input
        self.validate_request(&request)?;

        // Execute failover logic
        let (decision, constraints) = self.evaluate_failover(&request)?;

        let duration_us = start.elapsed().as_micros() as u64;

        // Create response
        let response = FailoverResponse::success(decision.clone(), constraints.clone(), duration_us);

        // Emit DecisionEvent to ruvector-service
        if self.config.emit_events {
            self.emit_decision_event(&request, &decision, &constraints, duration_us)
                .await
                .map_err(|e| {
                    warn!("Failed to emit DecisionEvent: {}", e);
                    // Non-blocking - continue with response
                })
                .ok();
        }

        info!(
            outcome = ?decision.outcome,
            confidence = decision.confidence,
            duration_us = duration_us,
            "Failover decision complete"
        );

        Ok(response)
    }

    /// Validate the failover request against schema
    fn validate_request(&self, request: &FailoverRequest) -> AgentResult<()> {
        use validator::Validate;

        request.validate().map_err(|e| {
            AgentError::SchemaValidation {
                field: "request".to_string(),
                message: e.to_string(),
            }
        })?;

        // Additional semantic validation
        if request.primary_target.provider_id.is_empty() {
            return Err(AgentError::Validation(
                "Primary target provider_id is required".to_string(),
            ));
        }

        Ok(())
    }

    /// Evaluate failover decision based on provider state
    fn evaluate_failover(
        &self,
        request: &FailoverRequest,
    ) -> AgentResult<(FailoverDecision, Vec<Constraint>)> {
        let mut constraints = Vec::new();
        let mut evaluated_targets = Vec::new();

        // Get effective circuit state and health for primary
        let primary_circuit = request
            .circuit_states
            .get(&request.primary_target.provider_id)
            .copied()
            .unwrap_or(request.primary_target.circuit_state);

        let primary_health = request
            .health_status
            .get(&request.primary_target.provider_id)
            .copied()
            .unwrap_or(request.primary_target.health);

        // Constraint: Check if failover is enabled
        let failover_enabled_constraint = Constraint {
            constraint_type: ConstraintType::Policy,
            name: "failover_enabled".to_string(),
            passed: request.policy.auto_failover_enabled,
            details: Some(format!(
                "Auto-failover enabled: {}",
                request.policy.auto_failover_enabled
            )),
            severity: None,
        };
        constraints.push(failover_enabled_constraint);

        // Constraint: Check max failover attempts
        let attempts_constraint = Constraint {
            constraint_type: ConstraintType::Policy,
            name: "max_failover_attempts".to_string(),
            passed: request.current_attempt < request.policy.max_failover_attempts,
            details: Some(format!(
                "Attempt {}/{} (max)",
                request.current_attempt, request.policy.max_failover_attempts
            )),
            severity: if request.current_attempt >= request.policy.max_failover_attempts {
                Some(0.7)
            } else {
                None
            },
        };
        constraints.push(attempts_constraint);

        // Evaluate primary target
        let primary_available = self.is_target_available(
            &request.primary_target,
            primary_circuit,
            primary_health,
            &request.policy,
        );

        evaluated_targets.push(EvaluatedTarget {
            provider_id: request.primary_target.provider_id.clone(),
            selected: false, // Will update if selected
            reason: if primary_available {
                "Primary target available".to_string()
            } else {
                format!(
                    "Primary unavailable: circuit={:?}, health={:?}",
                    primary_circuit, primary_health
                )
            },
            availability_score: self.calculate_availability_score(primary_circuit, primary_health),
        });

        // Constraint: Primary target circuit state
        let primary_circuit_constraint = Constraint {
            constraint_type: ConstraintType::Guard,
            name: "primary_circuit_state".to_string(),
            passed: primary_circuit != CircuitState::Open,
            details: Some(format!("Primary circuit state: {:?}", primary_circuit)),
            severity: if primary_circuit == CircuitState::Open {
                Some(0.8)
            } else {
                None
            },
        };
        constraints.push(primary_circuit_constraint);

        // If primary is available, use it
        if primary_available {
            let mut decision = FailoverDecision {
                outcome: FailoverOutcome::UsePrimary,
                selected_target: Some(request.primary_target.clone()),
                reason: "Primary target is healthy and available".to_string(),
                confidence: 1.0, // Deterministic based on circuit state
                attempt_number: 0,
                evaluated_targets: evaluated_targets.clone(),
            };
            decision.evaluated_targets[0].selected = true;

            debug!(provider_id = %request.primary_target.provider_id, "Using primary target");
            return Ok((decision, constraints));
        }

        // Primary unavailable, evaluate alternates
        if !request.policy.auto_failover_enabled {
            return Ok((
                FailoverDecision {
                    outcome: FailoverOutcome::NoViableTarget,
                    selected_target: None,
                    reason: "Primary unavailable and auto-failover is disabled".to_string(),
                    confidence: 1.0,
                    attempt_number: request.current_attempt,
                    evaluated_targets,
                },
                constraints,
            ));
        }

        // Check if max attempts exceeded
        if request.current_attempt >= request.policy.max_failover_attempts {
            return Ok((
                FailoverDecision {
                    outcome: FailoverOutcome::NoViableTarget,
                    selected_target: None,
                    reason: format!(
                        "Max failover attempts ({}) exceeded",
                        request.policy.max_failover_attempts
                    ),
                    confidence: 1.0,
                    attempt_number: request.current_attempt,
                    evaluated_targets,
                },
                constraints,
            ));
        }

        // Evaluate alternate targets by priority
        let mut sorted_alternates = request.alternate_targets.clone();
        sorted_alternates.sort_by_key(|t| t.priority);

        for alternate in &sorted_alternates {
            let alt_circuit = request
                .circuit_states
                .get(&alternate.provider_id)
                .copied()
                .unwrap_or(alternate.circuit_state);

            let alt_health = request
                .health_status
                .get(&alternate.provider_id)
                .copied()
                .unwrap_or(alternate.health);

            let alt_available =
                self.is_target_available(alternate, alt_circuit, alt_health, &request.policy);

            // Check if provider is allowed by policy
            let provider_allowed = self.is_provider_allowed(&alternate.provider_name, &request.policy);

            let eval = EvaluatedTarget {
                provider_id: alternate.provider_id.clone(),
                selected: false,
                reason: if !provider_allowed {
                    "Provider blocked by policy".to_string()
                } else if alt_available {
                    "Alternate available".to_string()
                } else {
                    format!(
                        "Alternate unavailable: circuit={:?}, health={:?}",
                        alt_circuit, alt_health
                    )
                },
                availability_score: self.calculate_availability_score(alt_circuit, alt_health),
            };
            evaluated_targets.push(eval);

            if alt_available && provider_allowed {
                // Add routing constraint for selected alternate
                constraints.push(Constraint {
                    constraint_type: ConstraintType::Routing,
                    name: "alternate_selection".to_string(),
                    passed: true,
                    details: Some(format!(
                        "Selected alternate: {} (priority {})",
                        alternate.provider_id, alternate.priority
                    )),
                    severity: None,
                });

                let mut decision = FailoverDecision {
                    outcome: FailoverOutcome::UseAlternate,
                    selected_target: Some(alternate.clone()),
                    reason: format!(
                        "Failover to {} - primary unavailable",
                        alternate.provider_name
                    ),
                    confidence: 0.9, // High confidence but not 1.0 (failover scenario)
                    attempt_number: request.current_attempt + 1,
                    evaluated_targets: evaluated_targets.clone(),
                };

                // Mark selected target
                if let Some(last) = decision.evaluated_targets.last_mut() {
                    last.selected = true;
                }

                info!(
                    provider_id = %alternate.provider_id,
                    attempt = request.current_attempt + 1,
                    "Failover to alternate"
                );
                return Ok((decision, constraints));
            }
        }

        // No viable targets
        warn!("No viable targets available for failover");
        Ok((
            FailoverDecision {
                outcome: FailoverOutcome::NoViableTarget,
                selected_target: None,
                reason: "All targets unavailable (circuits open or unhealthy)".to_string(),
                confidence: 1.0, // Deterministic - we evaluated all targets
                attempt_number: request.current_attempt,
                evaluated_targets,
            },
            constraints,
        ))
    }

    /// Check if a target is available based on circuit state and health
    fn is_target_available(
        &self,
        target: &TargetProvider,
        circuit_state: CircuitState,
        health: ProviderHealth,
        policy: &FailoverPolicy,
    ) -> bool {
        // Circuit must not be open
        if circuit_state == CircuitState::Open {
            return false;
        }

        // Health check
        match health {
            ProviderHealth::Healthy => true,
            ProviderHealth::Degraded => !policy.failover_on_degraded,
            ProviderHealth::Unhealthy => false,
            ProviderHealth::Unknown => {
                // Unknown health with closed circuit - cautiously allow
                circuit_state == CircuitState::Closed
            }
        }
    }

    /// Check if provider is allowed by policy
    fn is_provider_allowed(&self, provider_name: &str, policy: &FailoverPolicy) -> bool {
        // Check blocked list first
        if policy.blocked_providers.iter().any(|p| p == provider_name) {
            return false;
        }

        // If allowed list is empty, all providers are allowed
        // Otherwise, provider must be in allowed list
        policy.allowed_providers.is_empty()
            || policy.allowed_providers.iter().any(|p| p == provider_name)
    }

    /// Calculate availability score for a target
    fn calculate_availability_score(&self, circuit_state: CircuitState, health: ProviderHealth) -> f64 {
        let circuit_score = match circuit_state {
            CircuitState::Closed => 1.0,
            CircuitState::HalfOpen => 0.5,
            CircuitState::Open => 0.0,
        };

        let health_score = match health {
            ProviderHealth::Healthy => 1.0,
            ProviderHealth::Degraded => 0.6,
            ProviderHealth::Unhealthy => 0.0,
            ProviderHealth::Unknown => 0.5,
        };

        (circuit_score + health_score) / 2.0
    }

    /// Compute SHA-256 hash of input for deduplication
    fn compute_inputs_hash(&self, request: &FailoverRequest) -> String {
        let input_json = serde_json::json!({
            "primary_target": request.primary_target.provider_id,
            "alternate_targets": request.alternate_targets.iter()
                .map(|t| &t.provider_id)
                .collect::<Vec<_>>(),
            "circuit_states": &request.circuit_states,
            "health_status": &request.health_status,
            "current_attempt": request.current_attempt,
            "policy_id": &request.policy.policy_id,
        });

        let mut hasher = Sha256::new();
        hasher.update(input_json.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Emit DecisionEvent to ruvector-service
    async fn emit_decision_event(
        &self,
        request: &FailoverRequest,
        decision: &FailoverDecision,
        constraints: &[Constraint],
        duration_us: u64,
    ) -> AgentResult<()> {
        let agent_metadata = AgentMetadata {
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            agent_type: AgentType::Routing,
        };

        let inputs_hash = self.compute_inputs_hash(request);

        let outputs = serde_json::json!({
            "outcome": decision.outcome,
            "selected_target": decision.selected_target.as_ref().map(|t| &t.provider_id),
            "reason": &decision.reason,
            "attempt_number": decision.attempt_number,
            "evaluated_targets_count": decision.evaluated_targets.len(),
        });

        let event = DecisionEvent::new(
            agent_metadata,
            DecisionType::Failover,
            inputs_hash,
            outputs,
            decision.confidence,
            constraints.to_vec(),
            request.execution_context.execution_ref.clone(),
            duration_us,
        );

        self.ruvector_client.emit_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionContext;

    fn create_test_request() -> FailoverRequest {
        let ctx = ExecutionContext::new();
        let primary = TargetProvider::new("openai-1", "openai", "gpt-4");
        let alternates = vec![
            TargetProvider::new("anthropic-1", "anthropic", "claude-3"),
            TargetProvider::new("google-1", "google", "gemini-pro"),
        ];

        FailoverRequest::new(ctx, primary, alternates)
    }

    #[tokio::test]
    async fn test_use_primary_when_healthy() {
        let agent = FailoverAgent::new(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        });

        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Closed, ProviderHealth::Healthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::UsePrimary);
        assert_eq!(response.decision.confidence, 1.0);
    }

    #[tokio::test]
    async fn test_failover_when_primary_circuit_open() {
        let agent = FailoverAgent::new(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        });

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

    #[tokio::test]
    async fn test_no_viable_target_when_all_unavailable() {
        let agent = FailoverAgent::new(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        });

        let request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("anthropic-1", CircuitState::Open, ProviderHealth::Unhealthy)
            .with_provider_status("google-1", CircuitState::Open, ProviderHealth::Unhealthy);

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::NoViableTarget);
    }

    #[tokio::test]
    async fn test_failover_disabled_by_policy() {
        let agent = FailoverAgent::new(FailoverAgentConfig {
            emit_events: false,
            ..Default::default()
        });

        let mut request = create_test_request()
            .with_provider_status("openai-1", CircuitState::Open, ProviderHealth::Healthy);
        request.policy.auto_failover_enabled = false;

        let response = agent.process(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.decision.outcome, FailoverOutcome::NoViableTarget);
        assert!(response.decision.reason.contains("auto-failover is disabled"));
    }

    #[test]
    fn test_provider_allowed_by_policy() {
        let agent = FailoverAgent::new(FailoverAgentConfig::default());

        // Empty allowed list - all providers allowed
        let policy = FailoverPolicy::default();
        assert!(agent.is_provider_allowed("openai", &policy));
        assert!(agent.is_provider_allowed("anthropic", &policy));

        // Explicit allowed list
        let policy = FailoverPolicy {
            allowed_providers: vec!["openai".to_string()],
            ..Default::default()
        };
        assert!(agent.is_provider_allowed("openai", &policy));
        assert!(!agent.is_provider_allowed("anthropic", &policy));

        // Blocked list takes precedence
        let policy = FailoverPolicy {
            blocked_providers: vec!["openai".to_string()],
            ..Default::default()
        };
        assert!(!agent.is_provider_allowed("openai", &policy));
        assert!(agent.is_provider_allowed("anthropic", &policy));
    }
}
