//! Cache Strategy Agent Schemas
//!
//! This module defines the contracts for the Caching Strategy Agent.
//! The agent is classified as EXECUTION CONTROL and determines whether
//! execution results should be cached or reused.
//!
//! # Agent Classification: EXECUTION CONTROL
//!
//! # Scope
//! - Evaluate cache eligibility
//! - Apply cache read / write / bypass decisions
//! - Emit cache directives
//!
//! # decision_type: "cache_strategy_decision"
//!
//! # Non-Responsibilities (MUST NEVER DO)
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Persist state locally (use ruvector-service only)
//! - Execute SQL directly
//! - Store actual response payloads (only metadata)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Cache decision types
///
/// Defines the possible cache strategy decisions that can be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheDecision {
    /// Cache hit - return cached response (don't execute)
    CacheHit,
    /// Cache miss - execute and cache result
    CacheMiss,
    /// Cache write - store result in cache
    CacheWrite,
    /// Cache bypass - execute without caching (not cacheable)
    CacheBypass,
    /// Cache invalidate - remove from cache
    CacheInvalidate,
    /// Cache refresh - refresh cached entry (TTL expired but still valid)
    CacheRefresh,
}

/// Cache eligibility reasons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheIneligibilityReason {
    /// Request explicitly opts out of caching
    OptOut,
    /// Request type is not cacheable (mutations, etc.)
    NonCacheableRequestType,
    /// Response is too large to cache
    ResponseTooLarge,
    /// Response contains sensitive/PII data
    SensitiveData,
    /// Request has dynamic parameters that change
    DynamicParameters,
    /// Caching disabled by policy
    PolicyDisabled,
    /// TTL is zero or negative
    InvalidTtl,
    /// Model/provider doesn't support caching
    ProviderUnsupported,
    /// High-variability prompt (non-deterministic)
    NonDeterministic,
    /// Streaming response not cacheable
    StreamingResponse,
    /// User-specific response (not shareable)
    UserSpecific,
}

/// Cache layer/tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheLayer {
    /// L1 cache (in-memory, fastest, smallest)
    L1,
    /// L2 cache (Redis/distributed, medium speed)
    L2,
    /// L3 cache (persistent/disk, slowest, largest)
    L3,
}

/// Cache strategy input schema
///
/// Input provided to the Caching Strategy Agent for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CacheStrategyInput {
    /// Unique execution reference (trace ID)
    #[validate(length(min = 1, max = 256))]
    pub execution_ref: String,

    /// Request type (e.g., "chat_completion", "embedding", "completion")
    #[validate(length(min = 1, max = 64))]
    pub request_type: String,

    /// Provider ID (e.g., "openai", "anthropic", "google")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,

    /// Model ID (e.g., "gpt-4", "claude-3-opus")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// Hash of the request content for cache key generation
    #[validate(length(min = 1, max = 128))]
    pub content_hash: String,

    /// Hash of the prompt/messages (normalized)
    #[validate(length(min = 1, max = 128))]
    pub prompt_hash: String,

    /// Semantic hash for similarity matching (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_hash: Option<String>,

    /// Request parameters that affect caching (temperature, etc.)
    pub cache_relevant_params: CacheRelevantParams,

    /// Cache policy configuration
    pub policy: CachePolicyConfig,

    /// Existing cache entry metadata (if checking for hit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_entry: Option<CacheEntryMetadata>,

    /// Whether to check for cache hit
    #[serde(default)]
    pub check_cache: bool,

    /// Whether this is a write operation (storing result)
    #[serde(default)]
    pub is_write_operation: bool,

    /// Response metadata (for write operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_metadata: Option<ResponseMetadata>,

    /// Tenant ID for multi-tenancy cache isolation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// User ID hash for user-specific caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id_hash: Option<String>,
}

/// Cache-relevant request parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheRelevantParams {
    /// Temperature setting (affects randomness)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Maximum tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Random seed (if deterministic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Response format (json, text, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// Whether streaming is enabled
    #[serde(default)]
    pub stream: bool,

    /// System prompt hash (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_hash: Option<String>,
}

/// Cache policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CachePolicyConfig {
    /// Whether caching is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Default TTL in seconds
    #[validate(range(min = 0, max = 2592000))] // Max 30 days
    pub default_ttl_seconds: u32,

    /// Maximum response size to cache (bytes)
    #[serde(default = "default_max_response_size")]
    pub max_response_size_bytes: usize,

    /// Minimum temperature for caching (higher temp = less cacheable)
    #[serde(default = "default_max_temperature")]
    pub max_temperature_for_cache: f32,

    /// Cache layers to use
    #[serde(default = "default_cache_layers")]
    pub cache_layers: Vec<CacheLayer>,

    /// Whether to allow semantic caching
    #[serde(default)]
    pub semantic_caching_enabled: bool,

    /// Semantic similarity threshold (0.0-1.0)
    #[validate(range(min = 0.0, max = 1.0))]
    #[serde(default = "default_semantic_threshold")]
    pub semantic_similarity_threshold: f32,

    /// Request types that are cacheable
    #[serde(default = "default_cacheable_request_types")]
    pub cacheable_request_types: Vec<String>,

    /// Request types that are never cacheable
    #[serde(default)]
    pub non_cacheable_request_types: Vec<String>,

    /// Whether to cache user-specific responses separately
    #[serde(default)]
    pub per_user_caching: bool,

    /// Stale-while-revalidate window (seconds)
    #[serde(default)]
    pub stale_while_revalidate_seconds: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_response_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_temperature() -> f32 {
    0.3 // Temperatures above this are considered too random to cache
}

fn default_cache_layers() -> Vec<CacheLayer> {
    vec![CacheLayer::L1, CacheLayer::L2]
}

fn default_semantic_threshold() -> f32 {
    0.95
}

fn default_cacheable_request_types() -> Vec<String> {
    vec![
        "chat_completion".to_string(),
        "completion".to_string(),
        "embedding".to_string(),
    ]
}

impl Default for CachePolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_seconds: 3600, // 1 hour
            max_response_size_bytes: default_max_response_size(),
            max_temperature_for_cache: default_max_temperature(),
            cache_layers: default_cache_layers(),
            semantic_caching_enabled: false,
            semantic_similarity_threshold: default_semantic_threshold(),
            cacheable_request_types: default_cacheable_request_types(),
            non_cacheable_request_types: Vec::new(),
            per_user_caching: false,
            stale_while_revalidate_seconds: 60,
        }
    }
}

/// Existing cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// Cache key
    pub cache_key: String,

    /// Cache layer where entry exists
    pub layer: CacheLayer,

    /// When the entry was created
    pub created_at: DateTime<Utc>,

    /// When the entry expires
    pub expires_at: DateTime<Utc>,

    /// Last access time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accessed_at: Option<DateTime<Utc>>,

    /// Access count
    pub access_count: u64,

    /// Response size in bytes
    pub response_size_bytes: usize,

    /// Whether entry is currently valid
    pub is_valid: bool,

    /// Whether entry is stale but revalidatable
    #[serde(default)]
    pub is_stale: bool,

    /// Content hash at time of caching
    pub content_hash: String,

    /// Semantic hash at time of caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_hash: Option<String>,
}

/// Response metadata for cache write decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Response size in bytes
    pub response_size_bytes: usize,

    /// Response latency in milliseconds
    pub latency_ms: u64,

    /// Token count (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,

    /// Whether response completed successfully
    pub success: bool,

    /// Whether response contains sensitive data (detected)
    #[serde(default)]
    pub contains_sensitive_data: bool,

    /// Finish reason (stop, length, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Cache strategy output schema
///
/// Output from the Caching Strategy Agent decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStrategyOutput {
    /// The cache decision made
    pub decision: CacheDecision,

    /// Confidence in the decision (0.0-1.0)
    pub confidence: f64,

    /// Reason for the decision
    pub reason: String,

    /// Whether request is cache-eligible
    pub is_cacheable: bool,

    /// Reasons for ineligibility (if not cacheable)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ineligibility_reasons: Vec<CacheIneligibilityReason>,

    /// Cache directive details
    pub directive: CacheDirective,

    /// Cache key to use (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,

    /// Recommended cache layers
    #[serde(default)]
    pub recommended_layers: Vec<CacheLayer>,

    /// TTL to use (in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u32>,

    /// Whether semantic matching was used
    #[serde(default)]
    pub used_semantic_matching: bool,

    /// Semantic similarity score (if semantic matching used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_similarity: Option<f64>,

    /// Processing hints for cache operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_hints: Option<CacheProcessingHints>,
}

/// Cache directive details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDirective {
    /// Read from cache
    pub read: bool,

    /// Write to cache
    pub write: bool,

    /// Invalidate existing cache
    pub invalidate: bool,

    /// Refresh cache (background update)
    pub refresh: bool,

    /// Bypass all caching
    pub bypass: bool,
}

impl CacheDirective {
    /// Create a cache hit directive (read only)
    pub fn cache_hit() -> Self {
        Self {
            read: true,
            write: false,
            invalidate: false,
            refresh: false,
            bypass: false,
        }
    }

    /// Create a cache miss directive (write after execution)
    pub fn cache_miss() -> Self {
        Self {
            read: false,
            write: true,
            invalidate: false,
            refresh: false,
            bypass: false,
        }
    }

    /// Create a cache bypass directive (no caching)
    pub fn cache_bypass() -> Self {
        Self {
            read: false,
            write: false,
            invalidate: false,
            refresh: false,
            bypass: true,
        }
    }

    /// Create a cache refresh directive (stale-while-revalidate)
    pub fn cache_refresh() -> Self {
        Self {
            read: true,
            write: true,
            invalidate: false,
            refresh: true,
            bypass: false,
        }
    }

    /// Create a cache invalidate directive
    pub fn cache_invalidate() -> Self {
        Self {
            read: false,
            write: false,
            invalidate: true,
            refresh: false,
            bypass: false,
        }
    }
}

/// Processing hints for cache operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheProcessingHints {
    /// Priority for cache write (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_priority: Option<u8>,

    /// Whether to compress before caching
    #[serde(default)]
    pub compress: bool,

    /// Whether to encrypt sensitive data
    #[serde(default)]
    pub encrypt: bool,

    /// Background refresh recommended
    #[serde(default)]
    pub background_refresh: bool,

    /// Prefetch related entries
    #[serde(default)]
    pub prefetch: bool,
}

/// Data that MUST be persisted to ruvector-service
///
/// This struct documents what gets persisted in DecisionEvents.
/// Only metadata is persisted - never actual response content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePersistence {
    /// Execution reference
    pub execution_ref: String,

    /// Cache decision made
    pub decision: CacheDecision,

    /// Cache key used
    pub cache_key: Option<String>,

    /// Whether cache was hit
    pub cache_hit: bool,

    /// Cache layer used
    pub cache_layer: Option<CacheLayer>,

    /// Request content hash (NOT the content itself)
    pub content_hash: String,

    /// TTL applied
    pub ttl_seconds: Option<u32>,

    /// Decision confidence
    pub confidence: f64,

    /// Ineligibility reasons (if applicable)
    pub ineligibility_reasons: Vec<CacheIneligibilityReason>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Processing duration in microseconds
    pub duration_us: u64,
}

/// Data that MUST NOT be persisted
///
/// This struct documents what should NEVER be persisted.
/// This is for documentation purposes only - the struct is not used.
#[allow(dead_code)]
pub struct CacheNonPersisted {
    // Raw prompt/message content - NEVER persist
    // API keys or credentials - NEVER persist
    // Full response payloads - NEVER persist
    // PII or user data - NEVER persist
    // Actual cached content - NEVER persist
    // Semantic embeddings - NEVER persist (only hash)
}

/// Constraint applied during cache decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConstraint {
    /// Constraint name
    pub name: String,

    /// Constraint type
    pub constraint_type: CacheConstraintType,

    /// Whether constraint passed
    pub passed: bool,

    /// Details/reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Severity if failed (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<f64>,
}

/// Cache constraint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheConstraintType {
    /// Policy constraint (caching enabled/disabled)
    Policy,
    /// Request type eligibility
    RequestType,
    /// Size constraint
    Size,
    /// Temperature constraint
    Temperature,
    /// TTL constraint
    Ttl,
    /// Sensitivity constraint
    Sensitivity,
    /// Provider support constraint
    Provider,
    /// Semantic similarity constraint
    Semantic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_decision_serialization() {
        let decision = CacheDecision::CacheHit;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"cache_hit\"");

        let parsed: CacheDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, CacheDecision::CacheHit);
    }

    #[test]
    fn test_cache_policy_defaults() {
        let policy = CachePolicyConfig::default();
        assert!(policy.enabled);
        assert_eq!(policy.default_ttl_seconds, 3600);
        assert!(policy.cacheable_request_types.contains(&"chat_completion".to_string()));
    }

    #[test]
    fn test_cache_directive_constructors() {
        let hit = CacheDirective::cache_hit();
        assert!(hit.read);
        assert!(!hit.write);

        let miss = CacheDirective::cache_miss();
        assert!(!miss.read);
        assert!(miss.write);

        let bypass = CacheDirective::cache_bypass();
        assert!(bypass.bypass);
    }

    #[test]
    fn test_cache_strategy_input_validation() {
        let input = CacheStrategyInput {
            execution_ref: "exec-123".to_string(),
            request_type: "chat_completion".to_string(),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-4".to_string()),
            content_hash: "abc123".to_string(),
            prompt_hash: "def456".to_string(),
            semantic_hash: None,
            cache_relevant_params: CacheRelevantParams::default(),
            policy: CachePolicyConfig::default(),
            existing_entry: None,
            check_cache: true,
            is_write_operation: false,
            response_metadata: None,
            tenant_id: None,
            user_id_hash: None,
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_cache_output_serialization() {
        let output = CacheStrategyOutput {
            decision: CacheDecision::CacheHit,
            confidence: 1.0,
            reason: "Cache hit found".to_string(),
            is_cacheable: true,
            ineligibility_reasons: vec![],
            directive: CacheDirective::cache_hit(),
            cache_key: Some("key-123".to_string()),
            recommended_layers: vec![CacheLayer::L1],
            ttl_seconds: Some(3600),
            used_semantic_matching: false,
            semantic_similarity: None,
            processing_hints: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("cache_hit"));
        assert!(json.contains("key-123"));
    }

    #[test]
    fn test_cache_persistence_struct() {
        let persistence = CachePersistence {
            execution_ref: "exec-123".to_string(),
            decision: CacheDecision::CacheMiss,
            cache_key: Some("cache-key".to_string()),
            cache_hit: false,
            cache_layer: Some(CacheLayer::L2),
            content_hash: "content-hash".to_string(),
            ttl_seconds: Some(3600),
            confidence: 0.95,
            ineligibility_reasons: vec![],
            timestamp: Utc::now(),
            duration_us: 100,
        };

        let json = serde_json::to_string(&persistence).unwrap();
        assert!(json.contains("cache_miss"));
        assert!(json.contains("exec-123"));
    }
}
