//! Validation logic for Caching Strategy Agent
//!
//! This module provides deterministic validation for cache eligibility.
//! Validation is based on:
//! - Request type eligibility
//! - Policy constraints
//! - Parameter constraints (temperature, etc.)
//! - Provider/model support
//! - Response characteristics

use crate::error::{AgentError, AgentResult};

use super::types::{
    CacheIneligibilityReason, CachePolicyConfig, CacheRelevantParams, CacheStrategyConfig,
    CacheStrategyInput, CacheValidationError, CacheValidationResult, CacheValidationWarning,
    ResponseMetadata,
};

/// Validate a cache strategy request
///
/// This function is deterministic: given the same inputs, it will always
/// produce the same output.
pub fn validate_cache_strategy(
    input: &CacheStrategyInput,
    config: &CacheStrategyConfig,
) -> AgentResult<CacheValidationResult> {
    let mut result = CacheValidationResult::default();

    // Validate request type
    validate_request_type(input, &config.default_policy, &mut result);

    // Validate policy
    validate_policy(&input.policy, &mut result);

    // Validate parameters
    validate_params(&input.cache_relevant_params, &input.policy, &mut result);

    // Validate provider/model
    validate_provider_model(input, config, &mut result);

    // Validate response metadata if write operation
    if input.is_write_operation {
        if let Some(ref metadata) = input.response_metadata {
            validate_response_metadata(metadata, &input.policy, &mut result);
        }
    }

    // Set overall validity
    result.valid = result.request_type_valid
        && result.policy_valid
        && result.params_valid
        && result.errors.is_empty();

    Ok(result)
}

/// Validate request type eligibility
fn validate_request_type(
    input: &CacheStrategyInput,
    policy: &CachePolicyConfig,
    result: &mut CacheValidationResult,
) {
    // Check if request type is in the non-cacheable list
    if policy
        .non_cacheable_request_types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&input.request_type))
    {
        result.request_type_valid = false;
        result.errors.push(CacheValidationError {
            code: "NON_CACHEABLE_REQUEST_TYPE".to_string(),
            message: format!(
                "Request type '{}' is in the non-cacheable list",
                input.request_type
            ),
            field: Some("request_type".to_string()),
            severity: 0.6,
        });
        return;
    }

    // Check if request type is in the cacheable list (if non-empty)
    if !policy.cacheable_request_types.is_empty()
        && !policy
            .cacheable_request_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&input.request_type))
    {
        result.request_type_valid = false;
        result.errors.push(CacheValidationError {
            code: "REQUEST_TYPE_NOT_ALLOWED".to_string(),
            message: format!(
                "Request type '{}' is not in the allowed cacheable list",
                input.request_type
            ),
            field: Some("request_type".to_string()),
            severity: 0.5,
        });
    }
}

/// Validate policy configuration
fn validate_policy(policy: &CachePolicyConfig, result: &mut CacheValidationResult) {
    // Check if caching is enabled
    if !policy.enabled {
        result.policy_valid = false;
        result.errors.push(CacheValidationError {
            code: "CACHING_DISABLED".to_string(),
            message: "Caching is disabled by policy".to_string(),
            field: Some("policy.enabled".to_string()),
            severity: 0.4,
        });
        return;
    }

    // Validate TTL
    if policy.default_ttl_seconds == 0 {
        result.policy_valid = false;
        result.errors.push(CacheValidationError {
            code: "INVALID_TTL".to_string(),
            message: "TTL cannot be zero".to_string(),
            field: Some("policy.default_ttl_seconds".to_string()),
            severity: 0.3,
        });
    }

    // Validate cache layers
    if policy.cache_layers.is_empty() {
        result.warnings.push(CacheValidationWarning {
            code: "NO_CACHE_LAYERS".to_string(),
            message: "No cache layers configured".to_string(),
            recommendation: Some("Configure at least one cache layer (L1 or L2)".to_string()),
        });
    }
}

/// Validate cache-relevant parameters
fn validate_params(
    params: &CacheRelevantParams,
    policy: &CachePolicyConfig,
    result: &mut CacheValidationResult,
) {
    // Check temperature constraint
    if let Some(temp) = params.temperature {
        if temp > policy.max_temperature_for_cache {
            result.params_valid = false;
            result.errors.push(CacheValidationError {
                code: "TEMPERATURE_TOO_HIGH".to_string(),
                message: format!(
                    "Temperature {} exceeds max cacheable temperature {}",
                    temp, policy.max_temperature_for_cache
                ),
                field: Some("cache_relevant_params.temperature".to_string()),
                severity: 0.5,
            });
        }
    }

    // Check streaming
    if params.stream {
        result.params_valid = false;
        result.errors.push(CacheValidationError {
            code: "STREAMING_NOT_CACHEABLE".to_string(),
            message: "Streaming responses cannot be cached".to_string(),
            field: Some("cache_relevant_params.stream".to_string()),
            severity: 0.6,
        });
    }

    // Add warning for non-deterministic parameters
    if params.temperature.is_none() && params.seed.is_none() {
        result.warnings.push(CacheValidationWarning {
            code: "NO_DETERMINISM_PARAMS".to_string(),
            message: "No temperature or seed specified".to_string(),
            recommendation: Some(
                "Consider setting temperature=0 or seed for deterministic caching".to_string(),
            ),
        });
    }

    // Warning for high top_p
    if let Some(top_p) = params.top_p {
        if top_p > 0.9 {
            result.warnings.push(CacheValidationWarning {
                code: "HIGH_TOP_P".to_string(),
                message: format!("High top_p ({}) may result in variable outputs", top_p),
                recommendation: Some("Consider lowering top_p for more consistent caching".to_string()),
            });
        }
    }
}

/// Validate provider and model support
fn validate_provider_model(
    input: &CacheStrategyInput,
    config: &CacheStrategyConfig,
    result: &mut CacheValidationResult,
) {
    // Check provider
    if let Some(ref provider_id) = input.provider_id {
        if config
            .non_cacheable_providers
            .iter()
            .any(|p| p.eq_ignore_ascii_case(provider_id))
        {
            result.errors.push(CacheValidationError {
                code: "PROVIDER_NOT_CACHEABLE".to_string(),
                message: format!("Provider '{}' does not support caching", provider_id),
                field: Some("provider_id".to_string()),
                severity: 0.5,
            });
        }
    }

    // Check model
    if let Some(ref model_id) = input.model_id {
        if config
            .non_cacheable_models
            .iter()
            .any(|m| m.eq_ignore_ascii_case(model_id))
        {
            result.errors.push(CacheValidationError {
                code: "MODEL_NOT_CACHEABLE".to_string(),
                message: format!("Model '{}' does not support caching", model_id),
                field: Some("model_id".to_string()),
                severity: 0.5,
            });
        }
    }
}

/// Validate response metadata for write operations
fn validate_response_metadata(
    metadata: &ResponseMetadata,
    policy: &CachePolicyConfig,
    result: &mut CacheValidationResult,
) {
    // Check response size
    if metadata.response_size_bytes > policy.max_response_size_bytes {
        result.errors.push(CacheValidationError {
            code: "RESPONSE_TOO_LARGE".to_string(),
            message: format!(
                "Response size {} bytes exceeds max cacheable size {} bytes",
                metadata.response_size_bytes, policy.max_response_size_bytes
            ),
            field: Some("response_metadata.response_size_bytes".to_string()),
            severity: 0.4,
        });
    }

    // Check for sensitive data
    if metadata.contains_sensitive_data {
        result.errors.push(CacheValidationError {
            code: "SENSITIVE_DATA_DETECTED".to_string(),
            message: "Response contains sensitive data and cannot be cached".to_string(),
            field: Some("response_metadata.contains_sensitive_data".to_string()),
            severity: 0.8,
        });
    }

    // Check success
    if !metadata.success {
        result.errors.push(CacheValidationError {
            code: "UNSUCCESSFUL_RESPONSE".to_string(),
            message: "Only successful responses can be cached".to_string(),
            field: Some("response_metadata.success".to_string()),
            severity: 0.3,
        });
    }

    // Warning for truncated responses
    if let Some(ref finish_reason) = metadata.finish_reason {
        if finish_reason == "length" {
            result.warnings.push(CacheValidationWarning {
                code: "TRUNCATED_RESPONSE".to_string(),
                message: "Response was truncated due to length limit".to_string(),
                recommendation: Some(
                    "Consider if caching truncated responses is appropriate".to_string(),
                ),
            });
        }
    }
}

/// Get ineligibility reasons from validation result
pub fn get_ineligibility_reasons(
    result: &CacheValidationResult,
) -> Vec<CacheIneligibilityReason> {
    let mut reasons = Vec::new();

    for error in &result.errors {
        let reason = match error.code.as_str() {
            "NON_CACHEABLE_REQUEST_TYPE" | "REQUEST_TYPE_NOT_ALLOWED" => {
                CacheIneligibilityReason::NonCacheableRequestType
            }
            "CACHING_DISABLED" => CacheIneligibilityReason::PolicyDisabled,
            "INVALID_TTL" => CacheIneligibilityReason::InvalidTtl,
            "TEMPERATURE_TOO_HIGH" => CacheIneligibilityReason::NonDeterministic,
            "STREAMING_NOT_CACHEABLE" => CacheIneligibilityReason::StreamingResponse,
            "PROVIDER_NOT_CACHEABLE" | "MODEL_NOT_CACHEABLE" => {
                CacheIneligibilityReason::ProviderUnsupported
            }
            "RESPONSE_TOO_LARGE" => CacheIneligibilityReason::ResponseTooLarge,
            "SENSITIVE_DATA_DETECTED" => CacheIneligibilityReason::SensitiveData,
            _ => continue,
        };
        reasons.push(reason);
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_input() -> CacheStrategyInput {
        CacheStrategyInput {
            execution_ref: "exec-123".to_string(),
            request_type: "chat_completion".to_string(),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-4".to_string()),
            content_hash: "hash123".to_string(),
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
        }
    }

    #[test]
    fn test_valid_input() {
        let input = default_input();
        let config = CacheStrategyConfig::default();
        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(result.valid);
        assert!(result.request_type_valid);
        assert!(result.policy_valid);
        assert!(result.params_valid);
    }

    #[test]
    fn test_high_temperature() {
        let mut input = default_input();
        input.cache_relevant_params.temperature = Some(1.0);
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.params_valid);
        assert!(result.errors.iter().any(|e| e.code == "TEMPERATURE_TOO_HIGH"));
    }

    #[test]
    fn test_streaming_not_cacheable() {
        let mut input = default_input();
        input.cache_relevant_params.stream = true;
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "STREAMING_NOT_CACHEABLE"));
    }

    #[test]
    fn test_policy_disabled() {
        let mut input = default_input();
        input.policy.enabled = false;
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(!result.valid);
        assert!(!result.policy_valid);
    }

    #[test]
    fn test_non_cacheable_model() {
        let mut input = default_input();
        input.model_id = Some("gpt-4-turbo-preview".to_string());
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(result.errors.iter().any(|e| e.code == "MODEL_NOT_CACHEABLE"));
    }

    #[test]
    fn test_large_response() {
        let mut input = default_input();
        input.is_write_operation = true;
        input.response_metadata = Some(ResponseMetadata {
            response_size_bytes: 100 * 1024 * 1024, // 100MB
            latency_ms: 1000,
            token_count: Some(1000),
            success: true,
            contains_sensitive_data: false,
            finish_reason: Some("stop".to_string()),
        });
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(result.errors.iter().any(|e| e.code == "RESPONSE_TOO_LARGE"));
    }

    #[test]
    fn test_sensitive_data() {
        let mut input = default_input();
        input.is_write_operation = true;
        input.response_metadata = Some(ResponseMetadata {
            response_size_bytes: 1000,
            latency_ms: 100,
            token_count: Some(100),
            success: true,
            contains_sensitive_data: true,
            finish_reason: Some("stop".to_string()),
        });
        let config = CacheStrategyConfig::default();

        let result = validate_cache_strategy(&input, &config).unwrap();

        assert!(result.errors.iter().any(|e| e.code == "SENSITIVE_DATA_DETECTED"));
    }

    #[test]
    fn test_get_ineligibility_reasons() {
        let mut result = CacheValidationResult::default();
        result.valid = false;
        result.errors.push(CacheValidationError {
            code: "TEMPERATURE_TOO_HIGH".to_string(),
            message: "Test".to_string(),
            field: None,
            severity: 0.5,
        });
        result.errors.push(CacheValidationError {
            code: "STREAMING_NOT_CACHEABLE".to_string(),
            message: "Test".to_string(),
            field: None,
            severity: 0.5,
        });

        let reasons = get_ineligibility_reasons(&result);

        assert_eq!(reasons.len(), 2);
        assert!(reasons.contains(&CacheIneligibilityReason::NonDeterministic));
        assert!(reasons.contains(&CacheIneligibilityReason::StreamingResponse));
    }
}
