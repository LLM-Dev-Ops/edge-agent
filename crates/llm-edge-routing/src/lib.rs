//! Intelligent routing engine for LLM Edge Agent
//!
//! Provides:
//! - Cost-based routing
//! - Latency-based routing
//! - Hybrid routing (multi-factor scoring)
//! - Circuit breakers
//! - Fallback chains

pub mod strategy;
pub mod circuit_breaker;
pub mod error;

pub use error::{RoutingError, RoutingResult};
pub use strategy::{RoutingStrategy, RoutingDecision};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert_eq!(2 + 2, 4);
    }
}
