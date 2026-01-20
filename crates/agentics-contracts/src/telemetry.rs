//! Telemetry event schemas compatible with LLM-Observatory
//!
//! All agents emit telemetry through this schema for observability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Telemetry severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Telemetry event for LLM-Observatory compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Unique event ID
    pub event_id: Uuid,

    /// Source agent ID
    pub agent_id: String,

    /// Agent version
    pub agent_version: String,

    /// Event type/name
    pub event_type: String,

    /// Severity level
    pub level: TelemetryLevel,

    /// Event message
    pub message: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Duration in microseconds (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,

    /// Correlation ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Execution reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_ref: Option<String>,

    /// Additional attributes
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, serde_json::Value>,

    /// OpenTelemetry span context (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_context: Option<SpanContext>,
}

/// OpenTelemetry span context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub trace_flags: u8,
}

impl TelemetryEvent {
    pub fn new(
        agent_id: impl Into<String>,
        agent_version: impl Into<String>,
        event_type: impl Into<String>,
        level: TelemetryLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            event_type: event_type.into(),
            level,
            message: message.into(),
            timestamp: Utc::now(),
            duration_us: None,
            correlation_id: None,
            execution_ref: None,
            attributes: std::collections::HashMap::new(),
            span_context: None,
        }
    }

    pub fn with_duration_us(mut self, duration: u64) -> Self {
        self.duration_us = Some(duration);
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_execution_ref(mut self, exec_ref: impl Into<String>) -> Self {
        self.execution_ref = Some(exec_ref.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }
}

/// Metric types for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricValue {
    Counter { value: u64, unit: Option<String> },
    Gauge { value: f64, unit: Option<String> },
    Histogram { buckets: Vec<(f64, u64)>, sum: f64, count: u64 },
}

/// Circuit breaker specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerMetrics {
    pub provider_id: String,
    pub state: String,
    pub failure_count: u64,
    pub success_count: u64,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub state_transitions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_event_creation() {
        let event = TelemetryEvent::new(
            "circuit_breaker_agent",
            "0.1.0",
            "circuit_opened",
            TelemetryLevel::Warn,
            "Circuit opened for provider openai",
        )
        .with_duration_us(150)
        .with_attribute("provider_id", serde_json::json!("openai"));

        assert_eq!(event.agent_id, "circuit_breaker_agent");
        assert_eq!(event.level, TelemetryLevel::Warn);
        assert!(event.attributes.contains_key("provider_id"));
    }
}
