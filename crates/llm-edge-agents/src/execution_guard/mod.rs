//! Execution Guard Agent
//!
//! # Classification: PROTECTION / GUARD
//!
//! This agent enforces execution safety constraints at runtime,
//! protecting against resource exhaustion and budget overruns.
//!
//! # Purpose Statement
//!
//! The Execution Guard Agent is a PROTECTION / GUARD agent that enforces
//! execution safety constraints at runtime. It validates execution envelopes
//! against configurable resource limits and emits allow/block decisions.
//!
//! # Scope
//!
//! - Enforce token limits (input, output, total)
//! - Enforce time limits (max execution duration)
//! - Enforce memory limits
//! - Enforce cost limits (per-request budget)
//! - Enforce rate limits (requests per window)
//! - Enforce concurrency limits (max parallel executions)
//! - Block unsafe execution envelopes
//! - Emit guard decisions to ruvector-service
//!
//! # Decision Types
//!
//! - `allow`: Execution envelope passes all safety constraints
//! - `block`: Execution envelope violates one or more safety constraints
//!
//! decision_type: "execution_guard_decision"
//!
//! # Confidence Semantics
//!
//! Guard decisions are deterministic (threshold-based):
//! - 1.0 = Deterministic decision based on limit thresholds
//! - 0.95+ = All decisions are deterministic for guards
//!
//! # Constraints Applied
//!
//! The agent evaluates and persists these constraint types:
//! - Resource (token, memory, cost)
//! - RateLimit (requests per window)
//! - Guard (model restrictions)
//!
//! # Non-Responsibilities (MUST NEVER DO)
//!
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Persist state locally (use ruvector-service only)
//! - Execute SQL directly
//!
//! # DecisionEvent Mapping
//!
//! Data persisted to ruvector-service:
//! - agent_id: "execution-guard"
//! - agent_version: CARGO_PKG_VERSION
//! - decision_type: ExecutionGuardDecision
//! - inputs_hash: SHA-256 of request_id + model + tokens (no raw data)
//! - outputs: { allowed, reason, model, request_id, validation summary }
//! - confidence: 0.95-1.0 (deterministic)
//! - constraints_applied: All evaluated constraints
//! - execution_ref: Trace ID from context
//!
//! Data NOT persisted:
//! - Raw request payloads
//! - Sensitive metadata (API keys, credentials)
//! - PII from context
//!
//! # CLI Invocation
//!
//! ```bash
//! # Test mode - validate without full processing
//! llm-edge-agent agent execution-guard test --input <json>
//!
//! # Simulate mode - simulate decision with mock data
//! llm-edge-agent agent execution-guard simulate --model gpt-4 --tokens 5000
//!
//! # Inspect mode - show agent configuration and contract
//! llm-edge-agent agent execution-guard inspect
//! ```
//!
//! # Upstream Invocations
//!
//! This agent is invoked by:
//! - LLM-Orchestrator: Before executing LLM calls
//! - LLM-Shield: For pre-execution resource validation
//! - CostOps: For budget enforcement checks
//! - Direct API: For testing and validation
//!
//! # Failure Modes
//!
//! | Failure | HTTP Status | Description |
//! |---------|-------------|-------------|
//! | TOKEN_LIMIT_EXCEEDED | 403 | Input/output/total tokens exceed limits |
//! | COST_LIMIT_EXCEEDED | 403 | Estimated cost exceeds budget |
//! | RATE_LIMIT_EXCEEDED | 429 | Too many requests in window |
//! | CONCURRENCY_EXCEEDED | 429 | Too many parallel executions |
//! | MODEL_BLOCKED | 403 | Model is blocked by policy |
//! | VALIDATION_ERROR | 400 | Request schema validation failed |

mod decision;
mod events;
mod handler;
pub mod types;
mod validator;

pub use decision::{make_guard_decision, GuardDecisionContext};
pub use events::{emit_decision_event, RuvectorClient, RuvectorConfig};
pub use handler::{
    execution_guard_handler, execution_guard_handler_with_config,
    execution_guard_handler_with_ruvector, ExecutionGuardHandler,
};
pub use types::{
    ExecutionEnvelope, ExecutionGuardConfig, ExecutionGuardDecision, ExecutionGuardRequest,
    ExecutionGuardResponse, ExecutionHints, GuardViolation, ResourceLimits, ValidationResult,
};
pub use validator::validate_execution;

use crate::contracts::{AgentMetadata, AgentType};
use crate::AGENT_VERSION;

/// Agent identifier for DecisionEvent
pub const AGENT_ID: &str = "execution-guard";

/// Get agent metadata for DecisionEvent emission
pub fn agent_metadata() -> AgentMetadata {
    AgentMetadata {
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        agent_type: AgentType::Protection,
    }
}

/// Execution Guard Agent
///
/// Stateless agent for execution safety validation.
/// Designed for deployment as a Google Cloud Edge Function.
///
/// # Architecture
///
/// - Stateless execution at runtime
/// - No local persistence (all persistence via ruvector-service)
/// - Deterministic, machine-readable output
/// - DecisionEvent emission per invocation
/// - CLI-invokable endpoints (test/simulate/inspect)
#[derive(Debug, Clone)]
pub struct ExecutionGuardAgent {
    config: ExecutionGuardConfig,
    ruvector_client: Option<RuvectorClient>,
}

impl ExecutionGuardAgent {
    /// Create a new Execution Guard Agent
    pub fn new(config: ExecutionGuardConfig) -> Self {
        Self {
            config,
            ruvector_client: None,
        }
    }

    /// Create with ruvector-service client for DecisionEvent persistence
    pub fn with_ruvector_client(mut self, client: RuvectorClient) -> Self {
        self.ruvector_client = Some(client);
        self
    }

    /// Process an execution guard request
    ///
    /// This is the main entry point for the agent. It:
    /// 1. Validates the execution envelope against resource limits
    /// 2. Evaluates all safety constraints
    /// 3. Makes an allow/block decision
    /// 4. Emits a DecisionEvent to ruvector-service
    /// 5. Returns deterministic, machine-readable output
    ///
    /// # Non-Responsibilities
    ///
    /// This function does NOT:
    /// - Perform orchestration
    /// - Trigger retries
    /// - Emit alerts
    /// - Modify policies
    /// - Persist state locally
    pub async fn process(
        &self,
        request: ExecutionGuardRequest,
    ) -> crate::AgentResult<ExecutionGuardResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // Step 1: Validate the execution envelope
        let validation_result = validate_execution(&request, &self.config)?;

        // Step 2: Create decision context
        let context = GuardDecisionContext {
            request: &request,
            config: &self.config,
            validation: &validation_result,
        };

        // Step 3: Make the guard decision
        let decision = make_guard_decision(context)?;

        // Step 4: Build response
        let response = ExecutionGuardResponse {
            request_id: request.envelope.request_id.clone(),
            execution_ref: request
                .context
                .as_ref()
                .map(|c| c.execution_ref.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            decision: decision.clone(),
            validation: validation_result.clone(),
            processing_time_us: start.elapsed().as_micros() as u64,
        };

        // Step 5: Emit DecisionEvent (async, non-blocking)
        if let Some(ref client) = self.ruvector_client {
            let event = emit_decision_event(
                &request,
                &decision,
                &validation_result,
                response.processing_time_us,
            );

            // Non-blocking write - fire and forget
            let client_clone = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client_clone.emit(event).await {
                    tracing::warn!("Failed to emit DecisionEvent: {}", e);
                }
            });
        }

        Ok(response)
    }

    /// Get agent configuration (for inspect CLI command)
    pub fn config(&self) -> &ExecutionGuardConfig {
        &self.config
    }

    /// Get agent metadata
    pub fn metadata(&self) -> AgentMetadata {
        agent_metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ExecutionContext;
    use std::collections::HashMap;

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
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);

        assert_eq!(agent.metadata().agent_id, AGENT_ID);
        assert_eq!(agent.metadata().agent_type, AgentType::Protection);
    }

    #[tokio::test]
    async fn test_allow_decision() {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);

        let request = ExecutionGuardRequest {
            envelope: create_test_envelope(),
            context: Some(ExecutionContext::new()),
            limits: None,
            current_rate: Some(10),
        };

        let response = agent.process(request).await.unwrap();

        assert!(response.decision.allowed);
        assert!(response.validation.valid);
        assert!(response.decision.execution_hints.is_some());
    }

    #[tokio::test]
    async fn test_block_decision_token_limit() {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);

        let mut envelope = create_test_envelope();
        envelope.input_tokens = 200_000; // Exceeds 100K limit

        let request = ExecutionGuardRequest {
            envelope,
            context: Some(ExecutionContext::new()),
            limits: None,
            current_rate: Some(10),
        };

        let response = agent.process(request).await.unwrap();

        assert!(!response.decision.allowed);
        assert!(!response.validation.tokens_valid);
        assert!(!response.decision.violations.is_empty());
    }

    #[tokio::test]
    async fn test_block_decision_concurrency() {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);

        let mut envelope = create_test_envelope();
        envelope.current_concurrency = 15; // Exceeds default 10

        let request = ExecutionGuardRequest {
            envelope,
            context: Some(ExecutionContext::new()),
            limits: None,
            current_rate: Some(10),
        };

        let response = agent.process(request).await.unwrap();

        assert!(!response.decision.allowed);
        assert!(!response.validation.concurrency_valid);
    }

    #[tokio::test]
    async fn test_custom_limits() {
        let config = ExecutionGuardConfig::default();
        let agent = ExecutionGuardAgent::new(config);

        let mut envelope = create_test_envelope();
        envelope.input_tokens = 5000;

        let custom_limits = ResourceLimits {
            max_input_tokens: Some(1000), // Very low limit
            ..Default::default()
        };

        let request = ExecutionGuardRequest {
            envelope,
            context: Some(ExecutionContext::new()),
            limits: Some(custom_limits),
            current_rate: Some(10),
        };

        let response = agent.process(request).await.unwrap();

        assert!(!response.decision.allowed);
        assert!(!response.validation.tokens_valid);
    }

    #[test]
    fn test_agent_metadata() {
        let metadata = agent_metadata();
        assert_eq!(metadata.agent_id, "execution-guard");
        assert_eq!(metadata.agent_type, AgentType::Protection);
    }
}
