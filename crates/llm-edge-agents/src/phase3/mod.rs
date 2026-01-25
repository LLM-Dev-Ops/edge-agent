//! Phase 3 - Automation & Resilience (Layer 1)
//!
//! This module enforces Phase 3 startup hardening, execution role constraints,
//! performance budgets, and failure semantics for the Agentics Dev platform.
//!
//! # Phase 3 Requirements
//!
//! - AGENT_PHASE=phase3, AGENT_LAYER=layer1 enforcement
//! - Agents MUST: Coordinate, Route, Optimize, Escalate
//! - Agents MUST NOT: Make final decisions, Emit executive conclusions
//! - Performance budgets: MAX_TOKENS=1500, MAX_LATENCY_MS=3000, MAX_CALLS_PER_RUN=4
//! - HARD FAIL on: Ruvector unavailable, Execution guard violated
//! - Crashloop on misconfiguration

pub mod startup;
pub mod budgets;
pub mod signals;
pub mod guard;

pub use startup::{Phase3Config, Phase3StartupGuard, StartupError};
pub use budgets::{PerformanceBudget, BudgetEnforcer, BudgetViolation};
pub use signals::{Phase3Signal, SignalType, SignalEmitter, SignalReference};
pub use guard::{ExecutionRoleGuard, RoleViolation, AgentRole};
