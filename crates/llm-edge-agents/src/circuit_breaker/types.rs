//! Circuit Breaker Agent types and schemas
//!
//! All types defined here follow agentics-contracts patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Circuit state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Normal operation - requests flow through
    #[default]
    Closed,
    /// Circuit open - requests are blocked (fail fast)
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Open => write!(f, "open"),
            Self::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// Failure signal provided as input to the circuit breaker
///
/// The agent does NOT track failures internally - all failure signals
/// MUST be provided by the caller (LLM-Orchestrator).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FailureSignal {
    /// Unique signal identifier
    pub signal_id: String,

    /// Provider or endpoint that failed
    #[validate(length(min = 1, max = 128))]
    pub provider_id: String,

    /// Type of failure (timeout, error, rate_limit, etc.)
    #[validate(length(min = 1, max = 64))]
    pub failure_type: String,

    /// HTTP status code (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,

    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Timestamp of the failure
    pub timestamp: DateTime<Utc>,

    /// Duration of the failed request in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Request identifier for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl FailureSignal {
    pub fn new(provider_id: impl Into<String>, failure_type: impl Into<String>) -> Self {
        Self {
            signal_id: Uuid::new_v4().to_string(),
            provider_id: provider_id.into(),
            failure_type: failure_type.into(),
            status_code: None,
            error_message: None,
            timestamp: Utc::now(),
            duration_ms: None,
            request_id: None,
        }
    }

    pub fn with_status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }

    pub fn with_error_message(mut self, msg: impl Into<String>) -> Self {
        self.error_message = Some(msg.into());
        self
    }

    pub fn with_duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = Some(duration);
        self
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

/// Success signal for circuit recovery
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SuccessSignal {
    /// Unique signal identifier
    pub signal_id: String,

    /// Provider or endpoint that succeeded
    #[validate(length(min = 1, max = 128))]
    pub provider_id: String,

    /// Timestamp of the success
    pub timestamp: DateTime<Utc>,

    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,

    /// Request identifier for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl SuccessSignal {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            signal_id: Uuid::new_v4().to_string(),
            provider_id: provider_id.into(),
            timestamp: Utc::now(),
            response_time_ms: None,
            request_id: None,
        }
    }
}

/// Circuit breaker configuration (from policy)
///
/// These values are provided by policy, not modified by the agent.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CircuitBreakerConfig {
    /// Provider ID this config applies to
    #[validate(length(min = 1, max = 128))]
    pub provider_id: String,

    /// Number of consecutive failures before opening circuit
    #[validate(range(min = 1, max = 1000))]
    pub failure_threshold: u32,

    /// Number of consecutive successes before closing circuit
    #[validate(range(min = 1, max = 100))]
    pub success_threshold: u32,

    /// Timeout in seconds before attempting half-open
    #[validate(range(min = 1, max = 3600))]
    pub timeout_seconds: u32,

    /// Optional: failure rate threshold (0.0-1.0) as alternative to count
    #[validate(range(min = 0.0, max = 1.0))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_rate_threshold: Option<f64>,

    /// Window size for failure rate calculation (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_window_seconds: Option<u32>,

    /// Minimum requests in window before rate-based triggering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_requests_in_window: Option<u32>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            provider_id: "default".to_string(),
            failure_threshold: 5,
            success_threshold: 3,
            timeout_seconds: 30,
            failure_rate_threshold: None,
            rate_window_seconds: None,
            min_requests_in_window: None,
        }
    }
}

impl CircuitBreakerConfig {
    /// Load config from environment variables
    pub fn from_env(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            failure_threshold: std::env::var("CB_FAILURE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            success_threshold: std::env::var("CB_SUCCESS_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            timeout_seconds: std::env::var("CB_TIMEOUT_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            failure_rate_threshold: std::env::var("CB_FAILURE_RATE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok()),
            rate_window_seconds: std::env::var("CB_RATE_WINDOW_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok()),
            min_requests_in_window: std::env::var("CB_MIN_REQUESTS_IN_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok()),
        }
    }
}

/// Input to the Circuit Breaker Agent
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CircuitBreakerRequest {
    /// Execution reference from orchestrator
    #[validate(length(min = 1, max = 256))]
    pub execution_ref: String,

    /// Provider to check/update
    #[validate(length(min = 1, max = 128))]
    pub provider_id: String,

    /// Current circuit state (provided by orchestrator, loaded from persistence)
    pub current_state: CircuitState,

    /// Current failure count
    pub failure_count: u32,

    /// Current success count (for half-open recovery)
    pub success_count: u32,

    /// Last failure timestamp (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_time: Option<DateTime<Utc>>,

    /// New failure signal to record (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_signal: Option<FailureSignal>,

    /// New success signal to record (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_signal: Option<SuccessSignal>,

    /// Circuit breaker configuration (from policy)
    pub config: CircuitBreakerConfig,

    /// Correlation ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Timestamp of the request
    pub timestamp: DateTime<Utc>,
}

impl CircuitBreakerRequest {
    /// Create a new request for checking circuit state
    pub fn check(provider_id: impl Into<String>, execution_ref: impl Into<String>) -> Self {
        Self {
            execution_ref: execution_ref.into(),
            provider_id: provider_id.into(),
            current_state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            failure_signal: None,
            success_signal: None,
            config: CircuitBreakerConfig::default(),
            correlation_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new request for recording a failure
    pub fn record_failure(
        provider_id: impl Into<String>,
        execution_ref: impl Into<String>,
        signal: FailureSignal,
    ) -> Self {
        let provider = provider_id.into();
        Self {
            execution_ref: execution_ref.into(),
            provider_id: provider.clone(),
            current_state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            failure_signal: Some(signal),
            success_signal: None,
            config: CircuitBreakerConfig::default(),
            correlation_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a new request for recording a success
    pub fn record_success(
        provider_id: impl Into<String>,
        execution_ref: impl Into<String>,
        signal: SuccessSignal,
    ) -> Self {
        let provider = provider_id.into();
        Self {
            execution_ref: execution_ref.into(),
            provider_id: provider.clone(),
            current_state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            failure_signal: None,
            success_signal: Some(signal),
            config: CircuitBreakerConfig::default(),
            correlation_id: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_state(mut self, state: CircuitState, failures: u32, successes: u32) -> Self {
        self.current_state = state;
        self.failure_count = failures;
        self.success_count = successes;
        self
    }

    pub fn with_config(mut self, config: CircuitBreakerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Circuit breaker decision outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerDecision {
    /// Allow the request to proceed
    Allow,
    /// Block the request (circuit is open)
    Block,
    /// Allow with warning (half-open test)
    AllowWithProbe,
}

impl std::fmt::Display for CircuitBreakerDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Block => write!(f, "block"),
            Self::AllowWithProbe => write!(f, "allow_with_probe"),
        }
    }
}

/// State transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: CircuitState,
    pub to: CircuitState,
    pub trigger: String,
    pub timestamp: DateTime<Utc>,
}

/// Output from the Circuit Breaker Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerResponse {
    /// The decision made by the agent
    pub decision: CircuitBreakerDecision,

    /// New circuit state after this evaluation
    pub new_state: CircuitState,

    /// Updated failure count
    pub new_failure_count: u32,

    /// Updated success count
    pub new_success_count: u32,

    /// Updated last failure time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_last_failure_time: Option<DateTime<Utc>>,

    /// Time until circuit may transition (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u32>,

    /// Confidence score (1.0 for threshold-based, deterministic)
    pub confidence: f64,

    /// Reason for the decision
    pub reason: String,

    /// State transition that occurred (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_transition: Option<StateTransition>,

    /// Processing duration in microseconds
    pub duration_us: u64,
}

/// Data to persist to ruvector-service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerPersistence {
    /// Provider ID
    pub provider_id: String,

    /// Current circuit state
    pub state: CircuitState,

    /// Failure count
    pub failure_count: u32,

    /// Success count
    pub success_count: u32,

    /// Last failure timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_time: Option<DateTime<Utc>>,

    /// Last state change timestamp
    pub last_state_change: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl CircuitBreakerPersistence {
    pub fn new(provider_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            provider_id: provider_id.into(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_state_change: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_signal_creation() {
        let signal = FailureSignal::new("openai", "timeout")
            .with_status_code(504)
            .with_duration_ms(30000);

        assert_eq!(signal.provider_id, "openai");
        assert_eq!(signal.failure_type, "timeout");
        assert_eq!(signal.status_code, Some(504));
    }

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_circuit_state_serialization() {
        let state = CircuitState::HalfOpen;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"half_open\"");

        let parsed: CircuitState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_request_creation() {
        let request = CircuitBreakerRequest::check("openai", "exec-123");
        assert_eq!(request.provider_id, "openai");
        assert_eq!(request.current_state, CircuitState::Closed);
    }
}
