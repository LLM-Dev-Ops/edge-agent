//! Observability and monitoring for LLM Edge Agent
//!
//! Provides:
//! - Prometheus metrics
//! - OpenTelemetry tracing
//! - Request/response logging
//! - Cost tracking

pub mod metrics;
pub mod tracing;
pub mod error;

pub use error::{MonitoringError, MonitoringResult};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert_eq!(2 + 2, 4);
    }
}
