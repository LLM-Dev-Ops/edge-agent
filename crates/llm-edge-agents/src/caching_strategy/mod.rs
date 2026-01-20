//! Caching Strategy Agent
//!
//! # Classification: EXECUTION CONTROL
//!
//! This agent determines whether execution results should be cached or reused.
//! It evaluates cache eligibility, applies cache read/write/bypass decisions,
//! and emits cache directives.
//!
//! # Purpose
//!
//! The Caching Strategy Agent provides runtime execution control for LLM
//! response caching. It evaluates:
//! - Cache eligibility based on request characteristics
//! - Policy constraints (TTL, size limits, etc.)
//! - Parameter constraints (temperature, streaming, etc.)
//! - Provider/model support for caching
//! - Semantic similarity for approximate cache matching
//!
//! # Decision Types
//!
//! - `cache_hit`: Return cached response (don't execute)
//! - `cache_miss`: Execute and cache result
//! - `cache_write`: Store result in cache
//! - `cache_bypass`: Execute without caching
//! - `cache_invalidate`: Remove from cache
//! - `cache_refresh`: Refresh stale entry (stale-while-revalidate)
//!
//! # decision_type: "cache_strategy_decision"
//!
//! # Non-Responsibilities (MUST NEVER DO)
//!
//! - Perform orchestration (that is LLM-Orchestrator)
//! - Modify policies dynamically
//! - Trigger retries directly (retry logic lives in Orchestrator)
//! - Emit alerts (that is Sentinel)
//! - Perform analytics (that is Observatory/Latency-Lens)
//! - Persist state locally (use ruvector-service only)
//! - Execute SQL directly
//! - Store actual response payloads (only metadata)
//!
//! # CLI Invocation
//!
//! ```bash
//! # Test mode - validate cache eligibility
//! llm-edge-agent agent cache-strategy test --input <json>
//!
//! # Simulate mode - simulate cache decision with mock data
//! llm-edge-agent agent cache-strategy simulate --request-type chat_completion --temperature 0.0
//!
//! # Inspect mode - show agent configuration
//! llm-edge-agent agent cache-strategy inspect
//! ```
//!
//! # Upstream Invocations
//!
//! This agent is invoked by:
//! - LLM-Orchestrator: During request processing for cache check (pre-execution)
//! - LLM-Orchestrator: After response for cache write decision (post-execution)
//! - Direct API: For testing and validation
//!
//! # Versioning
//!
//! - Version format: MAJOR.MINOR.PATCH (semver)
//! - Breaking changes increment MAJOR
//! - New features increment MINOR
//! - Bug fixes increment PATCH
//! - All versions recorded in DecisionEvents for audit trail
//!
//! # Failure Modes
//!
//! | Failure | Behavior | Impact |
//! |---------|----------|--------|
//! | Validation error | Return bypass decision | Execution proceeds without caching |
//! | Policy not found | Use default policy | May cache when shouldn't |
//! | ruvector-service unavailable | Log warning, continue | DecisionEvent not persisted |
//! | Invalid cache entry | Return miss decision | Re-execution occurs |
//! | Semantic model unavailable | Fall back to exact matching | Reduced cache hit rate |

mod decision;
mod events;
mod handler;
pub mod types;
mod validator;

pub use decision::{make_decision, DecisionContext};
pub use events::{emit_decision_event, RuvectorClient, RuvectorConfig};
pub use handler::{cache_strategy_router, CacheStrategyHandlerState};
pub use types::{
    CacheKeyComponents, CacheKeyStrategy, CacheMetrics, CacheStrategyConfig,
    CacheStrategyConfigOverride, CacheStrategyRequest, CacheStrategyResponse,
    CacheValidationError, CacheValidationResult, CacheValidationWarning,
};
pub use validator::{get_ineligibility_reasons, validate_cache_strategy};

use crate::contracts::{AgentMetadata, AgentType};
use crate::error::AgentResult;
use crate::AGENT_VERSION;

/// Agent identifier for DecisionEvent
pub const AGENT_ID: &str = "caching-strategy";

/// Get agent metadata for DecisionEvent emission
pub fn agent_metadata() -> AgentMetadata {
    AgentMetadata {
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        agent_type: AgentType::Cache,
    }
}

/// Caching Strategy Agent
///
/// Stateless agent for cache strategy decisions and execution control.
/// Designed for deployment as a Google Cloud Edge Function.
///
/// # Architecture
///
/// The agent follows a deterministic pipeline:
/// 1. Validate input against schemas and policies
/// 2. Check for existing cache entry (if cache check requested)
/// 3. Evaluate constraints (policy, temperature, size, etc.)
/// 4. Make cache decision (hit/miss/bypass/write/invalidate/refresh)
/// 5. Emit DecisionEvent to ruvector-service (async, non-blocking)
/// 6. Return deterministic, machine-readable output
///
/// # Example
///
/// ```rust,ignore
/// use llm_edge_agents::caching_strategy::{
///     CachingStrategyAgent, CacheStrategyConfig, CacheStrategyRequest,
/// };
///
/// let config = CacheStrategyConfig::default();
/// let agent = CachingStrategyAgent::new(config);
///
/// let request = CacheStrategyRequest { ... };
/// let response = agent.process(request).await?;
///
/// match response.output.decision {
///     CacheDecision::CacheHit => println!("Return cached response"),
///     CacheDecision::CacheMiss => println!("Execute and cache"),
///     CacheDecision::CacheBypass => println!("Execute without caching"),
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CachingStrategyAgent {
    config: CacheStrategyConfig,
    ruvector_client: Option<RuvectorClient>,
}

impl CachingStrategyAgent {
    /// Create a new Caching Strategy Agent
    pub fn new(config: CacheStrategyConfig) -> Self {
        Self {
            config,
            ruvector_client: None,
        }
    }

    /// Create with ruvector-service client for DecisionEvent persistence
    pub fn with_ruvector_client(mut self, client: RuvectorClient) -> Self {
        self.ruvector_client = Some(client);
        self
    }

    /// Process a cache strategy request
    ///
    /// This is the main entry point for the agent. It:
    /// 1. Validates the request against schemas
    /// 2. Evaluates constraints and cache eligibility
    /// 3. Makes a cache decision
    /// 4. Emits a DecisionEvent to ruvector-service
    /// 5. Returns deterministic, machine-readable output
    pub async fn process(
        &self,
        request: CacheStrategyRequest,
    ) -> AgentResult<CacheStrategyResponse> {
        use std::time::Instant;
        let start = Instant::now();

        // Step 1: Validate the request
        let validation_result = validate_cache_strategy(&request.input, &self.config)?;

        // Step 2: Determine force flags from config override
        let (force_bypass, force_refresh, force_invalidate) = request
            .config_override
            .as_ref()
            .map(|o| (o.force_bypass, o.force_refresh, o.force_invalidate))
            .unwrap_or((false, false, false));

        // Step 3: Create decision context
        let context = DecisionContext {
            input: &request.input,
            config: &self.config,
            validation: &validation_result,
            force_bypass,
            force_refresh,
            force_invalidate,
        };

        // Step 4: Get constraints evaluated
        let constraints_evaluated = decision::get_constraints_evaluated(&context);

        // Step 5: Make the decision
        let output = make_decision(context)?;

        // Step 6: Build response
        let processing_time_us = start.elapsed().as_micros() as u64;

        let response = CacheStrategyResponse {
            request_id: request
                .context
                .as_ref()
                .and_then(|c| c.request_id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            execution_ref: request
                .context
                .as_ref()
                .map(|c| c.execution_ref.clone())
                .unwrap_or_else(|| request.input.execution_ref.clone()),
            output: output.clone(),
            validation: validation_result.clone(),
            constraints_evaluated: constraints_evaluated.clone(),
            processing_time_us,
            cache_metrics: None, // Would be populated from cache layer in production
        };

        // Step 7: Emit DecisionEvent (async, non-blocking)
        if let Some(ref client) = self.ruvector_client {
            let event = emit_decision_event(
                &request,
                &output,
                &validation_result,
                constraints_evaluated,
                processing_time_us,
            );

            // Non-blocking write - fire and forget
            let client_clone = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client_clone.emit(event).await {
                    tracing::warn!("Failed to emit DecisionEvent: {}", e);
                }
            });
        }

        Ok(response)
    }

    /// Get agent configuration (for inspect CLI command)
    pub fn config(&self) -> &CacheStrategyConfig {
        &self.config
    }

    /// Get agent metadata
    pub fn metadata(&self) -> AgentMetadata {
        agent_metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching_strategy::types::{
        CacheDecision, CacheLayer, CachePolicyConfig, CacheRelevantParams, CacheStrategyInput,
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

    #[tokio::test]
    async fn test_agent_creation() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        assert_eq!(agent.metadata().agent_id, AGENT_ID);
        assert_eq!(agent.metadata().agent_type, AgentType::Cache);
    }

    #[tokio::test]
    async fn test_basic_cache_miss() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let request = create_test_request();
        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheMiss);
        assert!(response.output.is_cacheable);
        assert!(response.output.cache_key.is_some());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.input.existing_entry = Some(types::CacheEntryMetadata {
            cache_key: "cache:test:key".to_string(),
            layer: CacheLayer::L1,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            last_accessed_at: Some(chrono::Utc::now()),
            access_count: 5,
            response_size_bytes: 1000,
            is_valid: true,
            is_stale: false,
            content_hash: "content123".to_string(),
            semantic_hash: None,
        });

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheHit);
        assert!(response.output.directive.read);
    }

    #[tokio::test]
    async fn test_cache_bypass_streaming() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.input.cache_relevant_params.stream = true;

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheBypass);
        assert!(!response.output.is_cacheable);
    }

    #[tokio::test]
    async fn test_cache_bypass_high_temperature() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.input.cache_relevant_params.temperature = Some(1.0);

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheBypass);
        assert!(!response.output.is_cacheable);
    }

    #[tokio::test]
    async fn test_force_bypass() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.config_override = Some(CacheStrategyConfigOverride {
            ttl_seconds: None,
            force_bypass: true,
            force_refresh: false,
            force_invalidate: false,
            cache_key_prefix: None,
            target_layers: None,
        });

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheBypass);
    }

    #[tokio::test]
    async fn test_force_invalidate() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.config_override = Some(CacheStrategyConfigOverride {
            ttl_seconds: None,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: true,
            cache_key_prefix: None,
            target_layers: None,
        });

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheInvalidate);
    }

    #[tokio::test]
    async fn test_cache_write_operation() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.input.is_write_operation = true;
        request.input.response_metadata = Some(types::ResponseMetadata {
            response_size_bytes: 1000,
            latency_ms: 100,
            token_count: Some(100),
            success: true,
            contains_sensitive_data: false,
            finish_reason: Some("stop".to_string()),
        });

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheWrite);
        assert!(response.output.directive.write);
    }

    #[tokio::test]
    async fn test_policy_disabled() {
        let config = CacheStrategyConfig::default();
        let agent = CachingStrategyAgent::new(config);

        let mut request = create_test_request();
        request.input.policy.enabled = false;

        let response = agent.process(request).await.unwrap();

        assert_eq!(response.output.decision, CacheDecision::CacheBypass);
        assert!(!response.output.is_cacheable);
    }

    #[test]
    fn test_agent_metadata() {
        let metadata = agent_metadata();
        assert_eq!(metadata.agent_id, "caching-strategy");
        assert_eq!(metadata.agent_type, AgentType::Cache);
    }
}
