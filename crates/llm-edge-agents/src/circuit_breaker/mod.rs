//! Circuit Breaker Agent
//!
//! # Purpose
//! Prevent cascading failures by blocking execution paths when thresholds are exceeded.
//!
//! # Classification
//! - **Type**: PROTECTION / GUARD
//! - **Decision Type**: `circuit_breaker_decision`
//!
//! # Scope
//! - Track failure signals (input-provided only)
//! - Apply circuit open / close decisions
//! - Emit block or allow outcomes
//!
//! # Non-Responsibilities
//!
//! This agent MUST NEVER:
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory / Latency-Lens)
//! - Persist state locally (all persistence via ruvector-service)
//! - Execute SQL directly
//!
//! # Invocation
//!
//! This agent is invoked by LLM-Orchestrator during execution flow.
//! LLM-Shield MAY be invoked by this agent for security enforcement.

pub mod types;
pub mod agent;
pub mod handler;
pub mod decision;

pub use types::*;
pub use agent::CircuitBreakerAgent;
pub use handler::CircuitBreakerHandler;
pub use decision::CircuitBreakerDecisionEngine;
