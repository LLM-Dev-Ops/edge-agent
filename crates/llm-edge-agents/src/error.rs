//! Error types for LLM Edge Agents

use thiserror::Error;

/// Agent error types
#[derive(Error, Debug)]
pub enum AgentError {
    /// Validation error for input/output
    #[error("Validation error: {0}")]
    Validation(String),

    /// Schema validation failed
    #[error("Schema validation failed: {field} - {message}")]
    SchemaValidation { field: String, message: String },

    /// Tool not allowed
    #[error("Tool not allowed: {0}")]
    ToolNotAllowed(String),

    /// Tool blocked by policy
    #[error("Tool blocked by policy: {tool} - {reason}")]
    ToolBlocked { tool: String, reason: String },

    /// Constraint violation
    #[error("Constraint violation: {constraint} - {details}")]
    ConstraintViolation { constraint: String, details: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Internal agent error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP client error (for ruvector-service)
    #[error("HTTP client error: {0}")]
    HttpClient(String),

    /// Event emission failed
    #[error("Failed to emit DecisionEvent: {0}")]
    EventEmission(String),

    /// Persistence error (ruvector-service)
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// RuVector service error
    #[error("RuVector service error: {0}")]
    RuVectorError(String),

    /// Validation error (alias for circuit breaker compatibility)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Configuration error (alias for circuit breaker compatibility)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

impl AgentError {
    /// Get error code for machine-readable output
    pub fn error_code(&self) -> &'static str {
        match self {
            AgentError::Validation(_) => "VALIDATION_ERROR",
            AgentError::SchemaValidation { .. } => "SCHEMA_VALIDATION_ERROR",
            AgentError::ToolNotAllowed(_) => "TOOL_NOT_ALLOWED",
            AgentError::ToolBlocked { .. } => "TOOL_BLOCKED",
            AgentError::ConstraintViolation { .. } => "CONSTRAINT_VIOLATION",
            AgentError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            AgentError::Internal(_) => "INTERNAL_ERROR",
            AgentError::Configuration(_) => "CONFIGURATION_ERROR",
            AgentError::Serialization(_) => "SERIALIZATION_ERROR",
            AgentError::HttpClient(_) => "HTTP_CLIENT_ERROR",
            AgentError::EventEmission(_) => "EVENT_EMISSION_ERROR",
            AgentError::PersistenceError(_) => "PERSISTENCE_ERROR",
            AgentError::RuVectorError(_) => "RUVECTOR_ERROR",
            AgentError::ValidationError(_) => "VALIDATION_ERROR",
            AgentError::ConfigurationError(_) => "CONFIGURATION_ERROR",
        }
    }

    /// Get severity level (0.0 = info, 1.0 = critical)
    pub fn severity(&self) -> f64 {
        match self {
            AgentError::Validation(_) => 0.3,
            AgentError::SchemaValidation { .. } => 0.4,
            AgentError::ToolNotAllowed(_) => 0.5,
            AgentError::ToolBlocked { .. } => 0.6,
            AgentError::ConstraintViolation { .. } => 0.6,
            AgentError::RateLimitExceeded(_) => 0.4,
            AgentError::Internal(_) => 0.8,
            AgentError::Configuration(_) => 0.7,
            AgentError::Serialization(_) => 0.5,
            AgentError::HttpClient(_) => 0.6,
            AgentError::EventEmission(_) => 0.3,
            AgentError::PersistenceError(_) => 0.6,
            AgentError::RuVectorError(_) => 0.6,
            AgentError::ValidationError(_) => 0.3,
            AgentError::ConfigurationError(_) => 0.7,
        }
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(err: reqwest::Error) -> Self {
        AgentError::HttpClient(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = AgentError::ToolNotAllowed("dangerous_tool".to_string());
        assert_eq!(err.error_code(), "TOOL_NOT_ALLOWED");
        assert!(err.severity() > 0.0);
    }

    #[test]
    fn test_error_display() {
        let err = AgentError::ToolBlocked {
            tool: "eval".to_string(),
            reason: "Security policy".to_string(),
        };
        assert!(err.to_string().contains("eval"));
        assert!(err.to_string().contains("Security policy"));
    }
}
