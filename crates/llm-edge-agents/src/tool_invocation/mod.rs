//! Tool Invocation Agent
//!
//! # Classification: EXECUTION CONTROL
//!
//! This agent intercepts and validates tool calls before execution,
//! enforcing invocation constraints and making allow/block decisions.
//!
//! # Purpose
//!
//! The Tool Invocation Agent provides runtime execution control for tool
//! invocations within LLM workflows. It validates:
//! - Tool schema compliance
//! - Security constraints (allowed/blocked tools)
//! - Resource constraints (memory, time)
//! - Capability requirements
//!
//! # Decision Types
//!
//! - `allow`: Tool invocation is permitted
//! - `block`: Tool invocation is blocked
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
//! # CLI Invocation
//!
//! ```bash
//! # Test mode - validate without execution
//! llm-edge-agent agent tool-invocation test --input <json>
//!
//! # Simulate mode - simulate decision with mock data
//! llm-edge-agent agent tool-invocation simulate --tool <name> --params <json>
//!
//! # Inspect mode - show agent configuration
//! llm-edge-agent agent tool-invocation inspect
//! ```
//!
//! # Upstream Invocations
//!
//! This agent is invoked by:
//! - LLM-Orchestrator: During workflow execution when tool calls are detected
//! - LLM-Shield: For pre-execution security validation
//! - Direct API: For testing and validation

mod decision;
mod events;
mod handler;
mod types;
mod validator;

pub use decision::{make_decision, DecisionContext};
pub use events::{emit_decision_event, RuvectorClient, RuvectorConfig};
pub use handler::{tool_invocation_handler, ToolInvocationHandler};
pub use types::{
    ToolDefinition, ToolInvocationConfig, ToolInvocationDecision, ToolInvocationRequest,
    ToolInvocationResponse, ToolParameter, ValidationResult,
};
pub use validator::validate_invocation;

use crate::contracts::{AgentMetadata, AgentType};
use crate::AGENT_VERSION;

/// Agent identifier for DecisionEvent
pub const AGENT_ID: &str = "tool-invocation";

/// Get agent metadata for DecisionEvent emission
pub fn agent_metadata() -> AgentMetadata {
    AgentMetadata {
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        agent_type: AgentType::ExecutionControl,
    }
}

/// Tool Invocation Agent
///
/// Stateless agent for tool invocation validation and execution control.
/// Designed for deployment as a Google Cloud Edge Function.
#[derive(Debug, Clone)]
pub struct ToolInvocationAgent {
    config: ToolInvocationConfig,
    ruvector_client: Option<RuvectorClient>,
}

impl ToolInvocationAgent {
    /// Create a new Tool Invocation Agent
    pub fn new(config: ToolInvocationConfig) -> Self {
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

    /// Process a tool invocation request
    ///
    /// This is the main entry point for the agent. It:
    /// 1. Validates the request against schemas
    /// 2. Evaluates constraints
    /// 3. Makes an allow/block decision
    /// 4. Emits a DecisionEvent to ruvector-service
    /// 5. Returns deterministic, machine-readable output
    pub async fn process(
        &self,
        request: ToolInvocationRequest,
    ) -> crate::AgentResult<ToolInvocationResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // Step 1: Validate the request
        let validation_result = validate_invocation(&request, &self.config)?;

        // Step 2: Create decision context
        let context = DecisionContext {
            request: &request,
            config: &self.config,
            validation: &validation_result,
        };

        // Step 3: Make the decision
        let decision = make_decision(context)?;

        // Step 4: Build response
        let response = ToolInvocationResponse {
            request_id: request.context.as_ref()
                .and_then(|c| c.request_id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            execution_ref: request.context.as_ref()
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
    pub fn config(&self) -> &ToolInvocationConfig {
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

    #[tokio::test]
    async fn test_agent_creation() {
        let config = ToolInvocationConfig::default();
        let agent = ToolInvocationAgent::new(config);

        assert_eq!(agent.metadata().agent_id, AGENT_ID);
        assert_eq!(agent.metadata().agent_type, AgentType::ExecutionControl);
    }

    #[tokio::test]
    async fn test_basic_invocation() {
        let config = ToolInvocationConfig::default();
        let agent = ToolInvocationAgent::new(config);

        let request = ToolInvocationRequest {
            tool: ToolDefinition {
                name: "calculator".to_string(),
                description: Some("A simple calculator".to_string()),
                parameters: vec![],
                capabilities: vec![],
            },
            arguments: serde_json::json!({"a": 1, "b": 2}),
            context: Some(ExecutionContext::new()),
            constraints: None,
        };

        let response = agent.process(request).await.unwrap();
        assert!(response.decision.allowed);
    }
}
