//! Execution envelope and context schemas
//!
//! Defines the execution context passed between LLM-Orchestrator and Edge agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Execution reference - unique identifier for an execution context
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExecutionRef {
    /// Unique execution ID
    pub id: Uuid,

    /// Orchestrator instance that created this execution
    #[validate(length(min = 1, max = 128))]
    pub orchestrator_id: String,

    /// Timestamp when execution was initiated
    pub initiated_at: DateTime<Utc>,

    /// Sequence number within the orchestrator session
    pub sequence: u64,
}

impl ExecutionRef {
    pub fn new(orchestrator_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            orchestrator_id: orchestrator_id.into(),
            initiated_at: Utc::now(),
            sequence: 0,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}:{}", self.orchestrator_id, self.id, self.sequence)
    }
}

/// Execution context passed to agents
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExecutionContext {
    /// Execution reference
    pub execution_ref: ExecutionRef,

    /// Request type (e.g., "chat_completion", "embedding")
    #[validate(length(min = 1, max = 64))]
    pub request_type: String,

    /// Target provider (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,

    /// Target model (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// Request priority (0-100, higher = more important)
    #[validate(range(min = 0, max = 100))]
    pub priority: u8,

    /// Retry attempt number (0 = first attempt)
    pub retry_attempt: u32,

    /// Maximum retries allowed by policy
    pub max_retries: u32,

    /// Timeout in milliseconds
    pub timeout_ms: u64,

    /// Tenant/organization ID (for multi-tenant deployments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// User ID (anonymized/hashed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id_hash: Option<String>,

    /// Correlation ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Additional context metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            execution_ref: ExecutionRef::new("default"),
            request_type: "unknown".to_string(),
            provider_id: None,
            model_id: None,
            priority: 50,
            retry_attempt: 0,
            max_retries: 3,
            timeout_ms: 30000,
            tenant_id: None,
            user_id_hash: None,
            correlation_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Execution envelope wrapping a request with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEnvelope<T> {
    /// Execution context
    pub context: ExecutionContext,

    /// The actual payload
    pub payload: T,

    /// Envelope version for compatibility
    pub version: String,
}

impl<T> ExecutionEnvelope<T> {
    pub fn new(context: ExecutionContext, payload: T) -> Self {
        Self {
            context,
            payload,
            version: "1.0.0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_ref_creation() {
        let exec_ref = ExecutionRef::new("orchestrator-1");
        assert_eq!(exec_ref.orchestrator_id, "orchestrator-1");
        assert_eq!(exec_ref.sequence, 0);
    }

    #[test]
    fn test_execution_context_default() {
        let ctx = ExecutionContext::default();
        assert_eq!(ctx.priority, 50);
        assert_eq!(ctx.retry_attempt, 0);
        assert_eq!(ctx.max_retries, 3);
    }
}
