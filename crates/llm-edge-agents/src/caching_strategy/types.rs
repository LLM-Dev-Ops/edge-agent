//! Types for Caching Strategy Agent
//!
//! These types define the request/response contracts for the Caching Strategy
//! Agent. All types are designed to be:
//! - Serializable (for HTTP API)
//! - Validatable (using validator crate)
//! - Deterministic (same inputs = same outputs)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::contracts::{Constraint, ExecutionContext};

// Re-export cache types from agentics-contracts for convenience
pub use agentics_contracts::{
    CacheConstraint, CacheConstraintType, CacheDecision, CacheDirective, CacheEntryMetadata,
    CacheIneligibilityReason, CacheLayer, CachePolicyConfig, CacheProcessingHints,
    CacheRelevantParams, CacheStrategyInput, CacheStrategyOutput, ResponseMetadata,
};

/// Cache strategy request
///
/// Input schema for the Caching Strategy Agent.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CacheStrategyRequest {
    /// Core cache strategy input (from agentics-contracts)
    #[validate(nested)]
    pub input: CacheStrategyInput,

    /// Execution context for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ExecutionContext>,

    /// Override configuration for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_override: Option<CacheStrategyConfigOverride>,
}

/// Configuration overrides for individual requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStrategyConfigOverride {
    /// Override TTL for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,

    /// Force bypass cache
    #[serde(default)]
    pub force_bypass: bool,

    /// Force cache refresh
    #[serde(default)]
    pub force_refresh: bool,

    /// Force cache invalidation
    #[serde(default)]
    pub force_invalidate: bool,

    /// Custom cache key prefix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_key_prefix: Option<String>,

    /// Specific layers to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_layers: Option<Vec<CacheLayer>>,
}

/// Cache strategy response
///
/// Output schema for the Caching Strategy Agent.
/// This is deterministic and machine-readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStrategyResponse {
    /// Request ID for correlation
    pub request_id: String,

    /// Execution reference for distributed tracing
    pub execution_ref: String,

    /// The cache strategy output
    pub output: CacheStrategyOutput,

    /// Validation results
    pub validation: CacheValidationResult,

    /// Constraints that were evaluated
    pub constraints_evaluated: Vec<Constraint>,

    /// Processing time in microseconds
    pub processing_time_us: u64,

    /// Cache metrics (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_metrics: Option<CacheMetrics>,
}

/// Validation result for cache strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValidationResult {
    /// Whether the input is valid
    pub valid: bool,

    /// Whether request type is cacheable
    pub request_type_valid: bool,

    /// Whether policy allows caching
    pub policy_valid: bool,

    /// Whether parameters are cacheable
    pub params_valid: bool,

    /// Validation errors (if any)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CacheValidationError>,

    /// Validation warnings (non-blocking)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<CacheValidationWarning>,
}

impl Default for CacheValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            request_type_valid: true,
            policy_valid: true,
            params_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Cache validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Field that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,

    /// Severity (0.0 = info, 1.0 = critical)
    pub severity: f64,
}

/// Cache validation warning (non-blocking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Recommendation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Cache metrics for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Cache hit rate (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_rate: Option<f64>,

    /// Average latency savings in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_savings_ms: Option<u64>,

    /// Current cache size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_size_bytes: Option<u64>,

    /// Number of entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_count: Option<u64>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStrategyConfig {
    /// Default cache policy
    pub default_policy: CachePolicyConfig,

    /// Enable detailed metrics
    #[serde(default)]
    pub enable_metrics: bool,

    /// Enable semantic caching
    #[serde(default)]
    pub enable_semantic_caching: bool,

    /// Semantic similarity model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_model: Option<String>,

    /// Cache key generation strategy
    #[serde(default = "default_key_strategy")]
    pub key_strategy: CacheKeyStrategy,

    /// Providers that don't support caching
    #[serde(default)]
    pub non_cacheable_providers: Vec<String>,

    /// Models that don't support caching
    #[serde(default)]
    pub non_cacheable_models: Vec<String>,

    /// Enable compression for large responses
    #[serde(default = "default_true")]
    pub enable_compression: bool,

    /// Compression threshold in bytes
    #[serde(default = "default_compression_threshold")]
    pub compression_threshold_bytes: usize,
}

fn default_true() -> bool {
    true
}

fn default_key_strategy() -> CacheKeyStrategy {
    CacheKeyStrategy::ContentHash
}

fn default_compression_threshold() -> usize {
    10 * 1024 // 10KB
}

/// Cache key generation strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheKeyStrategy {
    /// Use content hash only
    ContentHash,
    /// Use prompt hash with parameters
    PromptHashWithParams,
    /// Use semantic hash for similarity
    SemanticHash,
    /// Hybrid approach (content + semantic)
    Hybrid,
}

impl Default for CacheStrategyConfig {
    fn default() -> Self {
        Self {
            default_policy: CachePolicyConfig::default(),
            enable_metrics: false,
            enable_semantic_caching: false,
            semantic_model: None,
            key_strategy: CacheKeyStrategy::ContentHash,
            non_cacheable_providers: vec![],
            non_cacheable_models: vec![
                // Models that are typically used for non-deterministic tasks
                "gpt-4-turbo-preview".to_string(),
            ],
            enable_compression: true,
            compression_threshold_bytes: 10 * 1024,
        }
    }
}

/// Cache key components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheKeyComponents {
    /// Provider ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,

    /// Model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// Request type
    pub request_type: String,

    /// Content hash
    pub content_hash: String,

    /// Prompt hash
    pub prompt_hash: String,

    /// Parameters hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params_hash: Option<String>,

    /// Tenant ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// User ID hash (for per-user caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id_hash: Option<String>,
}

impl CacheKeyComponents {
    /// Generate a cache key from components
    pub fn generate_key(&self) -> String {
        let mut parts = vec![
            self.request_type.clone(),
            self.content_hash.clone(),
        ];

        if let Some(ref provider) = self.provider_id {
            parts.push(provider.clone());
        }

        if let Some(ref model) = self.model_id {
            parts.push(model.clone());
        }

        if let Some(ref params) = self.params_hash {
            parts.push(params.clone());
        }

        if let Some(ref tenant) = self.tenant_id {
            parts.push(format!("t:{}", tenant));
        }

        if let Some(ref user) = self.user_id_hash {
            parts.push(format!("u:{}", user));
        }

        format!("cache:{}", parts.join(":"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = CacheStrategyConfig::default();
        assert!(config.default_policy.enabled);
        assert!(config.enable_compression);
        assert_eq!(config.key_strategy, CacheKeyStrategy::ContentHash);
    }

    #[test]
    fn test_validation_result_default() {
        let result = CacheValidationResult::default();
        assert!(result.valid);
        assert!(result.request_type_valid);
        assert!(result.policy_valid);
        assert!(result.params_valid);
    }

    #[test]
    fn test_cache_key_generation() {
        let components = CacheKeyComponents {
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-4".to_string()),
            request_type: "chat_completion".to_string(),
            content_hash: "abc123".to_string(),
            prompt_hash: "def456".to_string(),
            params_hash: Some("param789".to_string()),
            tenant_id: None,
            user_id_hash: None,
        };

        let key = components.generate_key();
        assert!(key.starts_with("cache:"));
        assert!(key.contains("chat_completion"));
        assert!(key.contains("abc123"));
        assert!(key.contains("openai"));
        assert!(key.contains("gpt-4"));
    }

    #[test]
    fn test_cache_key_with_tenant() {
        let components = CacheKeyComponents {
            provider_id: None,
            model_id: None,
            request_type: "embedding".to_string(),
            content_hash: "hash123".to_string(),
            prompt_hash: "prompt456".to_string(),
            params_hash: None,
            tenant_id: Some("tenant-abc".to_string()),
            user_id_hash: Some("user-xyz".to_string()),
        };

        let key = components.generate_key();
        assert!(key.contains("t:tenant-abc"));
        assert!(key.contains("u:user-xyz"));
    }

    #[test]
    fn test_request_serialization() {
        let request = CacheStrategyRequest {
            input: CacheStrategyInput {
                execution_ref: "exec-123".to_string(),
                request_type: "chat_completion".to_string(),
                provider_id: Some("openai".to_string()),
                model_id: Some("gpt-4".to_string()),
                content_hash: "content123".to_string(),
                prompt_hash: "prompt456".to_string(),
                semantic_hash: None,
                cache_relevant_params: CacheRelevantParams::default(),
                policy: CachePolicyConfig::default(),
                existing_entry: None,
                check_cache: true,
                is_write_operation: false,
                response_metadata: None,
                tenant_id: None,
                user_id_hash: None,
            },
            context: None,
            config_override: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("chat_completion"));
        assert!(json.contains("exec-123"));
    }
}
