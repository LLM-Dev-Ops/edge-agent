//! Failover Agent - ROUTING
//!
//! Routes execution to alternate backends when primary targets are unavailable.
//!
//! # Classification
//!
//! **Type:** ROUTING
//!
//! # Purpose
//!
//! The Failover Agent evaluates provider health status, circuit breaker states,
//! and routing policies to determine if execution should be routed to an
//! alternate provider. It intercepts routing requests and makes deterministic
//! failover decisions based on configured policies.
//!
//! # Decision Type Semantics
//!
//! - `decision_type`: `Failover` (always)
//! - Possible outcomes:
//!   - `UsePrimary`: Primary target is healthy, use it
//!   - `UseAlternate`: Primary unavailable, route to alternate
//!   - `NoViableTarget`: All targets unavailable, block execution
//!
//! # Confidence Semantics
//!
//! - `1.0`: Deterministic decision based on circuit breaker state
//! - `0.9`: Policy-based decision with health checks
//! - `0.7-0.8`: Heuristic decision based on metrics thresholds
//! - `<0.7`: Degraded confidence due to incomplete data
//!
//! # Constraints Applied Semantics
//!
//! - `Routing`: Provider selection constraints
//! - `Policy`: Failover policy rules
//! - `Guard`: Health check guards
//! - `RateLimit`: Provider rate limits
//!
//! # Non-Responsibilities (MUST NEVER)
//!
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Execute SQL directly (use ruvector-service)
//! - Persist state locally (all persistence via ruvector-service)
//!
//! # Upstream Invocation
//!
//! This agent is invoked by:
//! - LLM-Orchestrator during execution routing
//! - LLM-Edge-Proxy when provider selection is needed
//!
//! # CLI Invocation
//!
//! ```bash
//! # Test mode - validate failover logic with mock inputs
//! llm-edge-agent failover test --primary openai --alternates anthropic,google
//!
//! # Simulate mode - run failover scenario without persistence
//! llm-edge-agent failover simulate --scenario primary-down
//!
//! # Inspect mode - show current failover configuration
//! llm-edge-agent failover inspect --provider openai
//! ```
//!
//! # Failure Modes
//!
//! - `NoAlternatesConfigured`: No alternate targets available
//! - `AllTargetsUnavailable`: All providers have open circuits
//! - `PolicyViolation`: Failover blocked by policy
//! - `InvalidConfiguration`: Malformed failover policy
//! - `EventEmissionFailure`: Failed to persist DecisionEvent (non-blocking)

mod agent;
mod contracts;
mod handler;
mod ruvector_client;

pub use agent::{FailoverAgent, FailoverAgentConfig};
pub use contracts::{
    CircuitState, EvaluatedTarget, FailoverDecision, FailoverOutcome, FailoverPolicy, FailoverRequest,
    FailoverResponse, ProviderHealth, TargetProvider,
};
pub use handler::{create_failover_router, failover_handler};
pub use ruvector_client::RuVectorClient;

/// Failover Agent ID for DecisionEvent tracking
pub const AGENT_ID: &str = "failover-agent";

/// Failover Agent version
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id() {
        assert_eq!(AGENT_ID, "failover-agent");
    }

    #[test]
    fn test_version_not_empty() {
        assert!(!AGENT_VERSION.is_empty());
    }
}
