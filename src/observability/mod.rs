//! Observability module for LLM Edge Agent
//!
//! Provides comprehensive observability stack:
//! - Prometheus metrics for monitoring
//! - OpenTelemetry tracing for distributed tracing
//! - Structured logging with PII redaction

pub mod logging;
pub mod metrics;
pub mod tracing;

// Re-export commonly used items
pub use logging::{
    redact_pii, sanitize_log_data, ErrorLog, ProviderRequestLog,
    RequestLog, ResponseLog, TokenUsage,
};
pub use metrics::{
    CacheMetrics, MetricsRegistry, ProviderMetrics, RequestMetrics,
    SystemMetrics, TokenMetrics,
};
pub use tracing::{init_tracing, shutdown_tracing, TracingConfig};
