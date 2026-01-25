//! Phase 3 Signal Emission
//!
//! Defines the DecisionEvent signals for Phase 3 Layer 1:
//! - execution_strategy_signal
//! - optimization_signal
//! - incident_signal
//!
//! All signals MUST include confidence + references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Signal types for Phase 3 Layer 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Execution strategy recommendation
    ExecutionStrategySignal,
    /// Optimization recommendation
    OptimizationSignal,
    /// Incident notification
    IncidentSignal,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalType::ExecutionStrategySignal => write!(f, "execution_strategy_signal"),
            SignalType::OptimizationSignal => write!(f, "optimization_signal"),
            SignalType::IncidentSignal => write!(f, "incident_signal"),
        }
    }
}

/// Reference to supporting evidence for a signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalReference {
    /// Reference type (metric, log, event, external)
    pub ref_type: String,
    /// Reference identifier
    pub ref_id: String,
    /// Human-readable description
    pub description: String,
    /// Optional URL or path to the reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SignalReference {
    pub fn metric(id: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            ref_type: "metric".to_string(),
            ref_id: id.into(),
            description: desc.into(),
            url: None,
        }
    }

    pub fn event(id: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            ref_type: "event".to_string(),
            ref_id: id.into(),
            description: desc.into(),
            url: None,
        }
    }

    pub fn log(id: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            ref_type: "log".to_string(),
            ref_id: id.into(),
            description: desc.into(),
            url: None,
        }
    }

    pub fn external(id: impl Into<String>, desc: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            ref_type: "external".to_string(),
            ref_id: id.into(),
            description: desc.into(),
            url: Some(url.into()),
        }
    }
}

/// Phase 3 signal - emitted by agents as recommendations/notifications
///
/// IMPORTANT: Signals are NOT decisions. They coordinate, route, optimize, and escalate.
/// They MUST NOT contain final decisions or executive conclusions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase3Signal {
    /// Unique signal identifier
    pub signal_id: Uuid,
    /// Signal type
    pub signal_type: SignalType,
    /// Agent that emitted this signal
    pub agent_id: String,
    /// Agent version
    pub agent_version: String,
    /// Phase and layer
    pub phase: String,
    pub layer: String,
    /// Signal payload (type-specific)
    pub payload: SignalPayload,
    /// Confidence score (0.0 to 1.0) - REQUIRED
    pub confidence: f64,
    /// Supporting references - REQUIRED (at least one)
    pub references: Vec<SignalReference>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Correlation ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

/// Signal payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalPayload {
    /// Execution strategy recommendation
    ExecutionStrategy {
        /// Recommended strategy
        strategy: String,
        /// Target component/agent
        target: String,
        /// Reason for recommendation
        reason: String,
        /// Alternative strategies (if any)
        alternatives: Vec<String>,
    },
    /// Optimization recommendation
    Optimization {
        /// Optimization type (performance, cost, reliability)
        optimization_type: String,
        /// Target metric or component
        target: String,
        /// Current value (if measurable)
        current_value: Option<String>,
        /// Recommended action
        recommendation: String,
        /// Expected improvement
        expected_improvement: Option<String>,
    },
    /// Incident notification
    Incident {
        /// Incident severity (low, medium, high, critical)
        severity: String,
        /// Incident category
        category: String,
        /// Description
        description: String,
        /// Affected components
        affected_components: Vec<String>,
        /// Suggested escalation path
        escalation_path: Option<String>,
    },
}

impl Phase3Signal {
    /// Create a new execution strategy signal
    pub fn execution_strategy(
        agent_id: impl Into<String>,
        agent_version: impl Into<String>,
        strategy: impl Into<String>,
        target: impl Into<String>,
        reason: impl Into<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Self {
        Self {
            signal_id: Uuid::new_v4(),
            signal_type: SignalType::ExecutionStrategySignal,
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            phase: "phase3".to_string(),
            layer: "layer1".to_string(),
            payload: SignalPayload::ExecutionStrategy {
                strategy: strategy.into(),
                target: target.into(),
                reason: reason.into(),
                alternatives: vec![],
            },
            confidence,
            references,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a new optimization signal
    pub fn optimization(
        agent_id: impl Into<String>,
        agent_version: impl Into<String>,
        optimization_type: impl Into<String>,
        target: impl Into<String>,
        recommendation: impl Into<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Self {
        Self {
            signal_id: Uuid::new_v4(),
            signal_type: SignalType::OptimizationSignal,
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            phase: "phase3".to_string(),
            layer: "layer1".to_string(),
            payload: SignalPayload::Optimization {
                optimization_type: optimization_type.into(),
                target: target.into(),
                current_value: None,
                recommendation: recommendation.into(),
                expected_improvement: None,
            },
            confidence,
            references,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a new incident signal
    pub fn incident(
        agent_id: impl Into<String>,
        agent_version: impl Into<String>,
        severity: impl Into<String>,
        category: impl Into<String>,
        description: impl Into<String>,
        affected_components: Vec<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Self {
        Self {
            signal_id: Uuid::new_v4(),
            signal_type: SignalType::IncidentSignal,
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            phase: "phase3".to_string(),
            layer: "layer1".to_string(),
            payload: SignalPayload::Incident {
                severity: severity.into(),
                category: category.into(),
                description: description.into(),
                affected_components,
                escalation_path: None,
            },
            confidence,
            references,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    /// Add correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Validate signal meets Phase 3 requirements
    pub fn validate(&self) -> Result<(), SignalValidationError> {
        // Confidence must be between 0 and 1
        if self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(SignalValidationError::InvalidConfidence(self.confidence));
        }

        // Must have at least one reference
        if self.references.is_empty() {
            return Err(SignalValidationError::MissingReferences);
        }

        // Phase must be "phase3"
        if self.phase != "phase3" {
            return Err(SignalValidationError::InvalidPhase(self.phase.clone()));
        }

        // Layer must be "layer1"
        if self.layer != "layer1" {
            return Err(SignalValidationError::InvalidLayer(self.layer.clone()));
        }

        Ok(())
    }
}

/// Signal validation errors
#[derive(Debug, thiserror::Error)]
pub enum SignalValidationError {
    #[error("Invalid confidence score: {0} (must be 0.0-1.0)")]
    InvalidConfidence(f64),

    #[error("Missing references (at least one required)")]
    MissingReferences,

    #[error("Invalid phase: {0} (must be 'phase3')")]
    InvalidPhase(String),

    #[error("Invalid layer: {0} (must be 'layer1')")]
    InvalidLayer(String),
}

/// Signal emitter for Phase 3 agents
pub struct SignalEmitter {
    agent_id: String,
    agent_version: String,
    ruvector_url: String,
    client: reqwest::Client,
}

impl SignalEmitter {
    pub fn new(agent_id: impl Into<String>, agent_version: impl Into<String>, ruvector_url: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            agent_version: agent_version.into(),
            ruvector_url: ruvector_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Emit a signal to RuVector
    pub async fn emit(&self, signal: &Phase3Signal) -> Result<(), SignalEmitError> {
        // Validate before emit
        signal.validate().map_err(SignalEmitError::ValidationFailed)?;

        let url = format!("{}/api/v1/signals", self.ruvector_url);

        let response = self.client
            .post(&url)
            .json(signal)
            .send()
            .await
            .map_err(|e| SignalEmitError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            tracing::debug!(
                signal_id = %signal.signal_id,
                signal_type = %signal.signal_type,
                "Signal emitted successfully"
            );
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(SignalEmitError::RuVectorError(format!("{}: {}", status, body)))
        }
    }

    /// Create an execution strategy signal
    pub fn execution_strategy_signal(
        &self,
        strategy: impl Into<String>,
        target: impl Into<String>,
        reason: impl Into<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Phase3Signal {
        Phase3Signal::execution_strategy(
            &self.agent_id,
            &self.agent_version,
            strategy,
            target,
            reason,
            confidence,
            references,
        )
    }

    /// Create an optimization signal
    pub fn optimization_signal(
        &self,
        optimization_type: impl Into<String>,
        target: impl Into<String>,
        recommendation: impl Into<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Phase3Signal {
        Phase3Signal::optimization(
            &self.agent_id,
            &self.agent_version,
            optimization_type,
            target,
            recommendation,
            confidence,
            references,
        )
    }

    /// Create an incident signal
    pub fn incident_signal(
        &self,
        severity: impl Into<String>,
        category: impl Into<String>,
        description: impl Into<String>,
        affected_components: Vec<String>,
        confidence: f64,
        references: Vec<SignalReference>,
    ) -> Phase3Signal {
        Phase3Signal::incident(
            &self.agent_id,
            &self.agent_version,
            severity,
            category,
            description,
            affected_components,
            confidence,
            references,
        )
    }
}

/// Signal emission errors
#[derive(Debug, thiserror::Error)]
pub enum SignalEmitError {
    #[error("Signal validation failed: {0}")]
    ValidationFailed(#[from] SignalValidationError),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("RuVector error: {0}")]
    RuVectorError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_strategy_signal() {
        let signal = Phase3Signal::execution_strategy(
            "test_agent",
            "0.1.0",
            "failover_primary",
            "openai_provider",
            "High latency detected",
            0.85,
            vec![SignalReference::metric("latency_p99", "P99 latency exceeded 2000ms")],
        );

        assert_eq!(signal.signal_type, SignalType::ExecutionStrategySignal);
        assert_eq!(signal.confidence, 0.85);
        assert!(signal.validate().is_ok());
    }

    #[test]
    fn test_signal_requires_references() {
        let signal = Phase3Signal::execution_strategy(
            "test_agent",
            "0.1.0",
            "strategy",
            "target",
            "reason",
            0.8,
            vec![], // Empty references
        );

        assert!(matches!(
            signal.validate(),
            Err(SignalValidationError::MissingReferences)
        ));
    }

    #[test]
    fn test_signal_confidence_bounds() {
        let signal = Phase3Signal::execution_strategy(
            "test_agent",
            "0.1.0",
            "strategy",
            "target",
            "reason",
            1.5, // Invalid confidence
            vec![SignalReference::metric("test", "test")],
        );

        assert!(matches!(
            signal.validate(),
            Err(SignalValidationError::InvalidConfidence(_))
        ));
    }
}
