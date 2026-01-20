//! LLM Edge Agents
//!
//! This crate provides edge agents for LLM runtime interception,
//! routing, and execution control. Agents are deployed as Google
//! Cloud Edge Functions within the LLM-Edge-Agent unified service.
//!
//! # Architecture
//!
//! All agents follow the LLM-Edge-Agent infrastructure constitution:
//! - Stateless execution at runtime
//! - No local persistence (all persistence via ruvector-service)
//! - Deterministic, machine-readable output
//! - DecisionEvent emission per invocation
//! - CLI-invokable endpoints (test/simulate/inspect)
//!
//! # Available Agents
//!
//! - [`failover`]: Failover Agent - ROUTING
//!   - Routes execution to alternate backends when primary targets are unavailable
//!   - Evaluates circuit breaker states and health status
//!   - Selects optimal alternate providers based on policy
//!
//! - [`tool_invocation`]: Tool Invocation Agent - EXECUTION CONTROL
//!   - Intercepts and validates tool calls before execution
//!   - Enforces invocation constraints
//!   - Allows or blocks execution
//!
//! - [`caching_strategy`]: Caching Strategy Agent - EXECUTION CONTROL
//!   - Determines whether execution results should be cached or reused
//!   - Evaluates cache eligibility based on request characteristics
//!   - Applies cache read/write/bypass/invalidate decisions
//!
//! - [`circuit_breaker`]: Circuit Breaker Agent - PROTECTION / GUARD
//!   - Prevents cascading failures by blocking execution paths
//!   - Tracks failure signals (input-provided only)
//!   - Applies circuit open / close decisions
//!
//! - [`execution_guard`]: Execution Guard Agent - PROTECTION / GUARD
//!   - Enforces execution safety constraints at runtime
//!   - Validates token, time, memory, cost, rate, and concurrency limits
//!   - Blocks unsafe execution envelopes
//!
//! # Non-Responsibilities
//!
//! Edge agents MUST NOT:
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Execute SQL directly (use ruvector-service)

pub mod caching_strategy;
pub mod circuit_breaker;
pub mod contracts;
pub mod error;
pub mod execution_guard;
pub mod failover;
pub mod tool_invocation;

// Re-exports
pub use contracts::{
    AgentMetadata, AgentType, Constraint, ConstraintType, DecisionEvent, DecisionType,
    ExecutionContext, InvocationConstraint,
};
pub use error::{AgentError, AgentResult};

// Failover Agent exports
pub use failover::{
    CircuitState, FailoverAgent, FailoverDecision, FailoverOutcome, FailoverPolicy,
    FailoverRequest, FailoverResponse, ProviderHealth, RuVectorClient, TargetProvider,
    create_failover_router, FailoverAgentConfig,
};

// Tool Invocation Agent exports
pub use tool_invocation::{
    ToolInvocationAgent, ToolInvocationConfig, ToolInvocationDecision,
    ToolInvocationRequest, ToolInvocationResponse,
    tool_invocation_handler, RuvectorConfig as ToolInvocationRuvectorConfig,
};

// Re-export ruvector client module
pub mod ruvector_client;
pub use ruvector_client::RuVectorConfig;

// Circuit Breaker Agent exports
pub use circuit_breaker::{
    CircuitBreakerAgent, CircuitBreakerDecisionEngine, CircuitBreakerHandler,
    CircuitBreakerRequest, CircuitBreakerResponse,
    CircuitBreakerDecision as CBDecision,  // Renamed to avoid conflict with failover::CircuitState
    CircuitBreakerConfig,
    CircuitState as CBCircuitState,  // Renamed to avoid conflict with failover::CircuitState
    FailureSignal, SuccessSignal,
};

// Execution Guard Agent exports
pub use execution_guard::{
    ExecutionEnvelope, ExecutionGuardAgent, ExecutionGuardConfig, ExecutionGuardDecision,
    ExecutionGuardRequest, ExecutionGuardResponse, ExecutionHints, GuardViolation,
    ResourceLimits, execution_guard_handler, execution_guard_handler_with_config,
    execution_guard_handler_with_ruvector,
};

// Caching Strategy Agent exports
pub use caching_strategy::{
    CachingStrategyAgent, CacheStrategyConfig, CacheStrategyRequest, CacheStrategyResponse,
    CacheValidationResult, CacheKeyStrategy, cache_strategy_router,
    CacheStrategyHandlerState,
};

/// Agent version for DecisionEvent tracking
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn test_version() {
        assert!(!super::AGENT_VERSION.is_empty());
    }
}
