//! Telemetry emission for LLM Edge Agents
//!
//! All agents emit telemetry compatible with LLM-Observatory through this module.

use std::sync::Arc;
use agentics_contracts::telemetry::{TelemetryEvent, TelemetryLevel, CircuitBreakerMetrics};
use tracing::{debug, error, info, warn, span, Level};
use crate::error::{AgentError, AgentResult};

/// Telemetry emitter for LLM-Observatory compatibility
pub struct TelemetryEmitter {
    agent_id: String,
    agent_version: String,
    /// Observatory endpoint (if direct emission is enabled)
    observatory_endpoint: Option<String>,
    /// HTTP client for direct emission
    client: Option<reqwest::Client>,
}

impl TelemetryEmitter {
    /// Create a new telemetry emitter
    pub fn new(agent_id: impl Into<String>, agent_version: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            observatory_endpoint: std::env::var("OBSERVATORY_ENDPOINT").ok(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .ok(),
        }
    }

    /// Emit a telemetry event
    ///
    /// Events are logged via tracing (for structured logging) and optionally
    /// sent directly to Observatory if configured.
    pub async fn emit(&self, event: TelemetryEvent) {
        // Always log via tracing for structured logging
        match event.level {
            TelemetryLevel::Trace => debug!(
                agent_id = %self.agent_id,
                event_type = %event.event_type,
                message = %event.message,
                "telemetry"
            ),
            TelemetryLevel::Debug => debug!(
                agent_id = %self.agent_id,
                event_type = %event.event_type,
                message = %event.message,
                "telemetry"
            ),
            TelemetryLevel::Info => info!(
                agent_id = %self.agent_id,
                event_type = %event.event_type,
                message = %event.message,
                "telemetry"
            ),
            TelemetryLevel::Warn => warn!(
                agent_id = %self.agent_id,
                event_type = %event.event_type,
                message = %event.message,
                "telemetry"
            ),
            TelemetryLevel::Error => error!(
                agent_id = %self.agent_id,
                event_type = %event.event_type,
                message = %event.message,
                "telemetry"
            ),
        }

        // Optionally emit to Observatory directly
        if let (Some(endpoint), Some(client)) = (&self.observatory_endpoint, &self.client) {
            let url = format!("{}/api/v1/events", endpoint);
            if let Err(e) = client.post(&url).json(&event).send().await {
                debug!("Failed to send telemetry to Observatory: {}", e);
            }
        }
    }

    /// Emit circuit breaker specific metrics
    pub fn emit_metrics(&self, metrics: &CircuitBreakerMetrics) {
        // Use metrics crate for Prometheus-compatible metrics
        metrics::counter!("circuit_breaker_total_requests", "provider" => metrics.provider_id.clone())
            .increment(metrics.total_requests);
        metrics::counter!("circuit_breaker_blocked_requests", "provider" => metrics.provider_id.clone())
            .increment(metrics.blocked_requests);
        metrics::counter!("circuit_breaker_failures", "provider" => metrics.provider_id.clone())
            .increment(metrics.failure_count);
        metrics::counter!("circuit_breaker_successes", "provider" => metrics.provider_id.clone())
            .increment(metrics.success_count);
        metrics::gauge!("circuit_breaker_state", "provider" => metrics.provider_id.clone(), "state" => metrics.state.clone())
            .set(1.0);
    }

    /// Create a telemetry event for circuit state change
    pub fn circuit_state_changed(
        &self,
        provider_id: &str,
        from_state: &str,
        to_state: &str,
        reason: &str,
    ) -> TelemetryEvent {
        TelemetryEvent::new(
            &self.agent_id,
            &self.agent_version,
            "circuit_state_changed",
            TelemetryLevel::Warn,
            format!("Circuit for {} changed from {} to {}: {}", provider_id, from_state, to_state, reason),
        )
        .with_attribute("provider_id", serde_json::json!(provider_id))
        .with_attribute("from_state", serde_json::json!(from_state))
        .with_attribute("to_state", serde_json::json!(to_state))
        .with_attribute("reason", serde_json::json!(reason))
    }

    /// Create a telemetry event for decision made
    pub fn decision_made(
        &self,
        provider_id: &str,
        decision: &str,
        confidence: f64,
        duration_us: u64,
    ) -> TelemetryEvent {
        TelemetryEvent::new(
            &self.agent_id,
            &self.agent_version,
            "decision_made",
            TelemetryLevel::Info,
            format!("Decision for {}: {} (confidence: {:.2})", provider_id, decision, confidence),
        )
        .with_duration_us(duration_us)
        .with_attribute("provider_id", serde_json::json!(provider_id))
        .with_attribute("decision", serde_json::json!(decision))
        .with_attribute("confidence", serde_json::json!(confidence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_emitter_creation() {
        let emitter = TelemetryEmitter::new("test_agent", "0.1.0");
        assert_eq!(emitter.agent_id, "test_agent");
        assert_eq!(emitter.agent_version, "0.1.0");
    }

    #[test]
    fn test_circuit_state_changed_event() {
        let emitter = TelemetryEmitter::new("circuit_breaker_agent", "0.1.0");
        let event = emitter.circuit_state_changed("openai", "closed", "open", "threshold exceeded");

        assert_eq!(event.event_type, "circuit_state_changed");
        assert_eq!(event.level, TelemetryLevel::Warn);
    }
}
