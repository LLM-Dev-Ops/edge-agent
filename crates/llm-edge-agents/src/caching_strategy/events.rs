//! DecisionEvent emission for Caching Strategy Agent
//!
//! This module handles the emission of DecisionEvents to ruvector-service.
//! All persistence occurs via ruvector-service - the agent NEVER connects
//! directly to the database.
//!
//! # Persistence Rules
//!
//! Data that IS persisted:
//! - Agent ID and version
//! - Decision type (cache_strategy_decision)
//! - Cache decision (hit/miss/bypass/write/invalidate/refresh)
//! - Inputs hash (SHA-256 for deduplication)
//! - Cache key used
//! - Constraint results
//! - Execution reference (trace ID)
//! - Timestamp and duration
//! - Cache metrics (hit rate, layer, etc.)
//!
//! Data that is NOT persisted:
//! - Raw prompt content (only hash)
//! - Actual cached responses
//! - Full request payload
//! - Sensitive context data
//! - PII or credentials
//! - Semantic embeddings (only hash)

use sha2::{Digest, Sha256};

use crate::contracts::{AgentMetadata, AgentType, Constraint, DecisionEvent, DecisionType};
use crate::error::{AgentError, AgentResult};
use crate::AGENT_VERSION;

use super::types::{
    CacheDecision, CacheStrategyInput, CacheStrategyOutput, CacheStrategyRequest,
    CacheValidationResult,
};
use super::AGENT_ID;

/// Configuration for ruvector-service client
#[derive(Debug, Clone)]
pub struct RuvectorConfig {
    /// Base URL for ruvector-service
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Request timeout in milliseconds
    pub timeout_ms: u64,

    /// Enable async (non-blocking) writes
    pub async_writes: bool,
}

impl Default for RuvectorConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("RUVECTOR_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").ok(),
            timeout_ms: 5000,
            async_writes: true,
        }
    }
}

/// Client for ruvector-service
#[derive(Debug, Clone)]
pub struct RuvectorClient {
    config: RuvectorConfig,
    http_client: reqwest::Client,
}

impl RuvectorClient {
    /// Create a new ruvector-service client
    pub fn new(config: RuvectorConfig) -> AgentResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AgentError::HttpClient(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create with default configuration
    pub fn from_env() -> AgentResult<Self> {
        Self::new(RuvectorConfig::default())
    }

    /// Emit a DecisionEvent to ruvector-service
    ///
    /// This is an async, non-blocking write. The agent does NOT wait for
    /// confirmation - fire and forget semantics.
    pub async fn emit(&self, event: DecisionEvent) -> AgentResult<()> {
        let url = format!("{}/api/v1/events", self.config.base_url);

        let mut request = self.http_client.post(&url).json(&event);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        // Add tracing headers
        request = request
            .header("X-Execution-Ref", &event.execution_ref)
            .header("X-Agent-ID", AGENT_ID)
            .header("X-Agent-Version", AGENT_VERSION);

        let response = request
            .send()
            .await
            .map_err(|e| AgentError::HttpClient(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(AgentError::EventEmission(format!(
                "ruvector-service returned {}: {}",
                status, body
            )));
        }

        tracing::debug!(
            execution_ref = %event.execution_ref,
            event_id = %event.event_id,
            "DecisionEvent emitted successfully"
        );

        Ok(())
    }
}

/// Create a DecisionEvent from a cache strategy decision
///
/// This function creates the canonical event format for persistence.
pub fn emit_decision_event(
    request: &CacheStrategyRequest,
    output: &CacheStrategyOutput,
    validation: &CacheValidationResult,
    constraints: Vec<Constraint>,
    duration_us: u64,
) -> DecisionEvent {
    // Calculate inputs hash (SHA-256)
    let inputs_hash = hash_inputs(&request.input);

    // Get execution reference
    let execution_ref = request
        .context
        .as_ref()
        .map(|c| c.execution_ref.clone())
        .unwrap_or_else(|| request.input.execution_ref.clone());

    // Build outputs (metadata only - never actual response content)
    let outputs = serde_json::json!({
        "decision": format!("{:?}", output.decision),
        "is_cacheable": output.is_cacheable,
        "cache_key": output.cache_key,
        "confidence": output.confidence,
        "recommended_layers": output.recommended_layers,
        "ttl_seconds": output.ttl_seconds,
        "used_semantic_matching": output.used_semantic_matching,
        "semantic_similarity": output.semantic_similarity,
        "ineligibility_reasons": output.ineligibility_reasons,
        "directive": {
            "read": output.directive.read,
            "write": output.directive.write,
            "invalidate": output.directive.invalidate,
            "refresh": output.directive.refresh,
            "bypass": output.directive.bypass,
        },
        "validation": {
            "valid": validation.valid,
            "request_type_valid": validation.request_type_valid,
            "policy_valid": validation.policy_valid,
            "params_valid": validation.params_valid,
            "error_count": validation.errors.len(),
            "warning_count": validation.warnings.len(),
        },
        "request_metadata": {
            "request_type": request.input.request_type,
            "provider_id": request.input.provider_id,
            "model_id": request.input.model_id,
            "check_cache": request.input.check_cache,
            "is_write_operation": request.input.is_write_operation,
        }
    });

    DecisionEvent::new(
        AgentMetadata {
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            agent_type: AgentType::Cache,
        },
        DecisionType::CacheStrategyDecision,
        inputs_hash,
        outputs,
        output.confidence,
        constraints,
        execution_ref,
        duration_us,
    )
}

/// Calculate SHA-256 hash of request inputs
///
/// Only hashes the structural parts needed for deduplication:
/// - Content hash
/// - Prompt hash
/// - Request type
/// - Provider/model
/// - Cache-relevant parameters
fn hash_inputs(input: &CacheStrategyInput) -> String {
    let mut hasher = Sha256::new();

    // Hash content and prompt hashes
    hasher.update(input.content_hash.as_bytes());
    hasher.update(input.prompt_hash.as_bytes());

    // Hash request type
    hasher.update(input.request_type.as_bytes());

    // Hash provider/model if present
    if let Some(ref provider) = input.provider_id {
        hasher.update(provider.as_bytes());
    }
    if let Some(ref model) = input.model_id {
        hasher.update(model.as_bytes());
    }

    // Hash cache-relevant parameters
    if let Some(temp) = input.cache_relevant_params.temperature {
        hasher.update(format!("t:{}", temp).as_bytes());
    }
    if let Some(seed) = input.cache_relevant_params.seed {
        hasher.update(format!("s:{}", seed).as_bytes());
    }
    if input.cache_relevant_params.stream {
        hasher.update(b"stream:true");
    }

    // Hash tenant/user for isolation
    if let Some(ref tenant) = input.tenant_id {
        hasher.update(format!("tenant:{}", tenant).as_bytes());
    }
    if let Some(ref user) = input.user_id_hash {
        hasher.update(format!("user:{}", user).as_bytes());
    }

    hex::encode(hasher.finalize())
}

/// Map CacheDecision to decision type for metrics
pub fn decision_to_metric_type(decision: &CacheDecision) -> &'static str {
    match decision {
        CacheDecision::CacheHit => "cache_hit",
        CacheDecision::CacheMiss => "cache_miss",
        CacheDecision::CacheWrite => "cache_write",
        CacheDecision::CacheBypass => "cache_bypass",
        CacheDecision::CacheInvalidate => "cache_invalidate",
        CacheDecision::CacheRefresh => "cache_refresh",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching_strategy::types::{
        CacheDirective, CacheLayer, CachePolicyConfig, CacheRelevantParams, CacheStrategyInput,
        CacheStrategyOutput,
    };
    use crate::contracts::ExecutionContext;

    fn create_test_request() -> CacheStrategyRequest {
        CacheStrategyRequest {
            input: CacheStrategyInput {
                execution_ref: "exec-123".to_string(),
                request_type: "chat_completion".to_string(),
                provider_id: Some("openai".to_string()),
                model_id: Some("gpt-4".to_string()),
                content_hash: "content123".to_string(),
                prompt_hash: "prompt456".to_string(),
                semantic_hash: None,
                cache_relevant_params: CacheRelevantParams {
                    temperature: Some(0.0),
                    top_p: None,
                    max_tokens: None,
                    seed: Some(42),
                    response_format: None,
                    stream: false,
                    system_prompt_hash: None,
                },
                policy: CachePolicyConfig::default(),
                existing_entry: None,
                check_cache: true,
                is_write_operation: false,
                response_metadata: None,
                tenant_id: None,
                user_id_hash: None,
            },
            context: Some(ExecutionContext::new()),
            config_override: None,
        }
    }

    fn create_test_output() -> CacheStrategyOutput {
        CacheStrategyOutput {
            decision: CacheDecision::CacheMiss,
            confidence: 1.0,
            reason: "Cache miss".to_string(),
            is_cacheable: true,
            ineligibility_reasons: vec![],
            directive: CacheDirective::cache_miss(),
            cache_key: Some("cache:test:key".to_string()),
            recommended_layers: vec![CacheLayer::L1, CacheLayer::L2],
            ttl_seconds: Some(3600),
            used_semantic_matching: false,
            semantic_similarity: None,
            processing_hints: None,
        }
    }

    #[test]
    fn test_hash_inputs() {
        let request = create_test_request();
        let hash = hash_inputs(&request.input);

        // Hash should be consistent
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
        assert_eq!(hash, hash_inputs(&request.input));
    }

    #[test]
    fn test_hash_inputs_different() {
        let request1 = create_test_request();
        let mut request2 = create_test_request();
        request2.input.content_hash = "different_hash".to_string();

        let hash1 = hash_inputs(&request1.input);
        let hash2 = hash_inputs(&request2.input);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_emit_decision_event() {
        let request = create_test_request();
        let output = create_test_output();
        let validation = CacheValidationResult::default();

        let event = emit_decision_event(&request, &output, &validation, vec![], 100);

        assert_eq!(event.agent.agent_id, AGENT_ID);
        assert_eq!(event.decision_type, DecisionType::CacheStrategyDecision);
        assert!(!event.inputs_hash.is_empty());
        assert!(event.outputs.get("is_cacheable").unwrap().as_bool().unwrap());
        assert_eq!(
            event.outputs.get("decision").unwrap().as_str().unwrap(),
            "CacheMiss"
        );
    }

    #[test]
    fn test_decision_to_metric_type() {
        assert_eq!(
            decision_to_metric_type(&CacheDecision::CacheHit),
            "cache_hit"
        );
        assert_eq!(
            decision_to_metric_type(&CacheDecision::CacheMiss),
            "cache_miss"
        );
        assert_eq!(
            decision_to_metric_type(&CacheDecision::CacheBypass),
            "cache_bypass"
        );
    }

    #[test]
    fn test_ruvector_config_defaults() {
        let config = RuvectorConfig::default();
        assert!(config.base_url.contains("localhost") || !config.base_url.is_empty());
        assert!(config.timeout_ms > 0);
    }

    #[test]
    fn test_event_outputs_structure() {
        let request = create_test_request();
        let output = create_test_output();
        let validation = CacheValidationResult::default();

        let event = emit_decision_event(&request, &output, &validation, vec![], 100);

        // Verify output structure
        let outputs = &event.outputs;
        assert!(outputs.get("decision").is_some());
        assert!(outputs.get("is_cacheable").is_some());
        assert!(outputs.get("cache_key").is_some());
        assert!(outputs.get("directive").is_some());
        assert!(outputs.get("validation").is_some());
        assert!(outputs.get("request_metadata").is_some());

        // Verify directive structure
        let directive = outputs.get("directive").unwrap();
        assert!(directive.get("read").is_some());
        assert!(directive.get("write").is_some());
        assert!(directive.get("bypass").is_some());

        // Verify validation structure
        let validation_output = outputs.get("validation").unwrap();
        assert!(validation_output.get("valid").is_some());
        assert!(validation_output.get("error_count").is_some());
    }
}
