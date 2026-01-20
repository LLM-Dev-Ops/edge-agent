//! # Agentics Contracts
//!
//! Shared schemas, policies, and execution envelopes for the Agentics Dev platform.
//! All LLM-Edge-Agent agents MUST import schemas exclusively from this crate.
//!
//! ## Modules
//!
//! - `decision`: DecisionEvent schemas for persisting agent decisions to ruvector-service
//! - `circuit_breaker`: Circuit Breaker Agent specific schemas
//! - `cache`: Caching Strategy Agent schemas
//! - `execution`: Execution envelope and context schemas
//! - `policy`: Policy definitions and constraints
//! - `telemetry`: Telemetry event schemas compatible with LLM-Observatory

pub mod cache;
pub mod circuit_breaker;
pub mod decision;
pub mod execution;
pub mod policy;
pub mod telemetry;

pub use cache::{
    CacheConstraint,
    CacheConstraintType,
    CacheDecision,
    CacheDirective,
    CacheEntryMetadata,
    CacheIneligibilityReason,
    CacheLayer,
    CachePersistence,
    CachePolicyConfig,
    CacheProcessingHints,
    CacheRelevantParams,
    CacheStrategyInput,
    CacheStrategyOutput,
    ResponseMetadata,
};
pub use circuit_breaker::{
    CircuitBreakerConfig,
    CircuitBreakerDecision,
    CircuitBreakerInput,
    CircuitBreakerOutput,
    FailureSignal,
};
pub use decision::{DecisionEvent, DecisionOutcome, DecisionType};
pub use execution::{ExecutionContext, ExecutionEnvelope, ExecutionRef};
pub use policy::{PolicyConstraint, PolicyRule, PolicyViolation};
pub use telemetry::{TelemetryEvent, TelemetryLevel};
