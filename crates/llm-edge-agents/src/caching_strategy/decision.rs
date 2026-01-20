//! Decision logic for Caching Strategy Agent
//!
//! This module provides deterministic decision-making for cache strategy.
//! Decisions are based on validation results, existing cache entries,
//! and policy configuration.
//!
//! # Decision Types
//!
//! - `CacheHit`: Return cached response (don't execute)
//! - `CacheMiss`: Execute and cache result
//! - `CacheBypass`: Execute without caching
//! - `CacheRefresh`: Return cached response but refresh in background
//! - `CacheInvalidate`: Remove from cache
//! - `CacheWrite`: Store result in cache

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::contracts::{Constraint, ConstraintType};
use crate::error::AgentResult;

use super::types::{
    CacheDecision, CacheDirective, CacheKeyComponents, CacheKeyStrategy, CacheLayer,
    CacheProcessingHints, CacheStrategyConfig, CacheStrategyInput, CacheStrategyOutput,
    CacheValidationResult,
};
use super::validator::get_ineligibility_reasons;

/// Context for making a cache decision
pub struct DecisionContext<'a> {
    pub input: &'a CacheStrategyInput,
    pub config: &'a CacheStrategyConfig,
    pub validation: &'a CacheValidationResult,
    pub force_bypass: bool,
    pub force_refresh: bool,
    pub force_invalidate: bool,
}

/// Make a cache strategy decision
///
/// This function is deterministic: given the same inputs, it will always
/// produce the same output. It does NOT:
/// - Perform orchestration
/// - Trigger retries
/// - Emit alerts
/// - Modify policies
/// - Access actual cache storage
pub fn make_decision(context: DecisionContext) -> AgentResult<CacheStrategyOutput> {
    let DecisionContext {
        input,
        config,
        validation,
        force_bypass,
        force_refresh,
        force_invalidate,
    } = context;

    let mut constraints_evaluated = Vec::new();

    // Handle force invalidate first
    if force_invalidate {
        return Ok(build_invalidate_output(input, &constraints_evaluated));
    }

    // Handle force bypass
    if force_bypass {
        return Ok(build_bypass_output(
            input,
            "Cache bypass forced by request",
            &constraints_evaluated,
        ));
    }

    // Evaluate validation constraint
    let validation_passed = validation.valid;
    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::Cache,
        name: "cache_eligibility_validation".to_string(),
        passed: validation_passed,
        details: if validation_passed {
            Some("Cache eligibility validation passed".to_string())
        } else {
            Some(format!(
                "{} validation errors found",
                validation.errors.len()
            ))
        },
        severity: if validation_passed { None } else { Some(0.5) },
    });

    // If validation failed, bypass cache
    if !validation_passed {
        let reasons = get_ineligibility_reasons(validation);
        return Ok(CacheStrategyOutput {
            decision: CacheDecision::CacheBypass,
            confidence: calculate_confidence(&constraints_evaluated, validation),
            reason: format!(
                "Cache ineligible: {}",
                validation.errors.first().map(|e| &e.message).unwrap_or(&"Unknown".to_string())
            ),
            is_cacheable: false,
            ineligibility_reasons: reasons,
            directive: CacheDirective::cache_bypass(),
            cache_key: None,
            recommended_layers: vec![],
            ttl_seconds: None,
            used_semantic_matching: false,
            semantic_similarity: None,
            processing_hints: None,
        });
    }

    // Evaluate policy constraint
    let policy_enabled = input.policy.enabled;
    constraints_evaluated.push(Constraint {
        constraint_type: ConstraintType::Policy,
        name: "cache_policy_enabled".to_string(),
        passed: policy_enabled,
        details: if policy_enabled {
            Some("Caching enabled by policy".to_string())
        } else {
            Some("Caching disabled by policy".to_string())
        },
        severity: if policy_enabled { None } else { Some(0.4) },
    });

    // Generate cache key
    let cache_key = generate_cache_key(input, config);

    // Check for existing cache entry (cache hit check)
    if input.check_cache {
        if let Some(ref existing) = input.existing_entry {
            // Evaluate cache entry validity
            let entry_valid = existing.is_valid;
            let entry_stale = existing.is_stale;

            constraints_evaluated.push(Constraint {
                constraint_type: ConstraintType::Cache,
                name: "cache_entry_validity".to_string(),
                passed: entry_valid,
                details: if entry_valid {
                    Some(format!(
                        "Cache entry valid, accessed {} times",
                        existing.access_count
                    ))
                } else {
                    Some("Cache entry invalid or expired".to_string())
                },
                severity: if entry_valid { None } else { Some(0.2) },
            });

            if entry_valid {
                // Check semantic matching if enabled
                let (used_semantic, semantic_sim) =
                    check_semantic_match(input, existing, &input.policy, config);

                if entry_stale && force_refresh {
                    // Stale-while-revalidate scenario
                    return Ok(CacheStrategyOutput {
                        decision: CacheDecision::CacheRefresh,
                        confidence: calculate_confidence(&constraints_evaluated, validation),
                        reason: "Cache entry stale, returning cached value and refreshing".to_string(),
                        is_cacheable: true,
                        ineligibility_reasons: vec![],
                        directive: CacheDirective::cache_refresh(),
                        cache_key: Some(cache_key),
                        recommended_layers: get_recommended_layers(&input.policy),
                        ttl_seconds: Some(input.policy.default_ttl_seconds),
                        used_semantic_matching: used_semantic,
                        semantic_similarity: semantic_sim,
                        processing_hints: Some(CacheProcessingHints {
                            write_priority: Some(50),
                            compress: false,
                            encrypt: false,
                            background_refresh: true,
                            prefetch: false,
                        }),
                    });
                }

                // Valid cache hit
                return Ok(CacheStrategyOutput {
                    decision: CacheDecision::CacheHit,
                    confidence: calculate_confidence(&constraints_evaluated, validation),
                    reason: format!(
                        "Cache hit for key '{}', {} accesses",
                        existing.cache_key, existing.access_count
                    ),
                    is_cacheable: true,
                    ineligibility_reasons: vec![],
                    directive: CacheDirective::cache_hit(),
                    cache_key: Some(existing.cache_key.clone()),
                    recommended_layers: vec![existing.layer],
                    ttl_seconds: None, // Entry already has TTL
                    used_semantic_matching: used_semantic,
                    semantic_similarity: semantic_sim,
                    processing_hints: None,
                });
            }
        }
    }

    // Handle write operations
    if input.is_write_operation {
        return Ok(CacheStrategyOutput {
            decision: CacheDecision::CacheWrite,
            confidence: calculate_confidence(&constraints_evaluated, validation),
            reason: "Writing response to cache".to_string(),
            is_cacheable: true,
            ineligibility_reasons: vec![],
            directive: CacheDirective {
                read: false,
                write: true,
                invalidate: false,
                refresh: false,
                bypass: false,
            },
            cache_key: Some(cache_key),
            recommended_layers: get_recommended_layers(&input.policy),
            ttl_seconds: Some(input.policy.default_ttl_seconds),
            used_semantic_matching: false,
            semantic_similarity: None,
            processing_hints: Some(build_processing_hints(input, config)),
        });
    }

    // Cache miss - execute and cache
    Ok(CacheStrategyOutput {
        decision: CacheDecision::CacheMiss,
        confidence: calculate_confidence(&constraints_evaluated, validation),
        reason: "Cache miss - will cache after execution".to_string(),
        is_cacheable: true,
        ineligibility_reasons: vec![],
        directive: CacheDirective::cache_miss(),
        cache_key: Some(cache_key),
        recommended_layers: get_recommended_layers(&input.policy),
        ttl_seconds: Some(input.policy.default_ttl_seconds),
        used_semantic_matching: config.enable_semantic_caching && input.semantic_hash.is_some(),
        semantic_similarity: None,
        processing_hints: Some(build_processing_hints(input, config)),
    })
}

/// Generate a cache key based on the configuration strategy
fn generate_cache_key(input: &CacheStrategyInput, config: &CacheStrategyConfig) -> String {
    let components = CacheKeyComponents {
        provider_id: input.provider_id.clone(),
        model_id: input.model_id.clone(),
        request_type: input.request_type.clone(),
        content_hash: input.content_hash.clone(),
        prompt_hash: input.prompt_hash.clone(),
        params_hash: match config.key_strategy {
            CacheKeyStrategy::PromptHashWithParams | CacheKeyStrategy::Hybrid => {
                Some(hash_params(&input.cache_relevant_params))
            }
            _ => None,
        },
        tenant_id: input.tenant_id.clone(),
        user_id_hash: if input.policy.per_user_caching {
            input.user_id_hash.clone()
        } else {
            None
        },
    };

    components.generate_key()
}

/// Hash cache-relevant parameters
fn hash_params(params: &super::types::CacheRelevantParams) -> String {
    let mut hasher = Sha256::new();

    if let Some(temp) = params.temperature {
        hasher.update(format!("t:{}", temp).as_bytes());
    }
    if let Some(top_p) = params.top_p {
        hasher.update(format!("p:{}", top_p).as_bytes());
    }
    if let Some(max_tokens) = params.max_tokens {
        hasher.update(format!("m:{}", max_tokens).as_bytes());
    }
    if let Some(seed) = params.seed {
        hasher.update(format!("s:{}", seed).as_bytes());
    }
    if let Some(ref format) = params.response_format {
        hasher.update(format!("f:{}", format).as_bytes());
    }

    hex::encode(hasher.finalize())[..16].to_string()
}

/// Check for semantic match
fn check_semantic_match(
    input: &CacheStrategyInput,
    existing: &super::types::CacheEntryMetadata,
    policy: &super::types::CachePolicyConfig,
    config: &CacheStrategyConfig,
) -> (bool, Option<f64>) {
    if !config.enable_semantic_caching || !policy.semantic_caching_enabled {
        return (false, None);
    }

    match (&input.semantic_hash, &existing.semantic_hash) {
        (Some(input_hash), Some(existing_hash)) => {
            // Simple hash comparison for now
            // In production, this would use actual semantic similarity
            let similarity = if input_hash == existing_hash {
                1.0
            } else {
                0.0 // Would use vector similarity in production
            };

            (true, Some(similarity))
        }
        _ => (false, None),
    }
}

/// Get recommended cache layers based on policy
fn get_recommended_layers(policy: &super::types::CachePolicyConfig) -> Vec<CacheLayer> {
    if policy.cache_layers.is_empty() {
        vec![CacheLayer::L1, CacheLayer::L2]
    } else {
        policy.cache_layers.clone()
    }
}

/// Build processing hints
fn build_processing_hints(
    input: &CacheStrategyInput,
    config: &CacheStrategyConfig,
) -> CacheProcessingHints {
    let response_size = input
        .response_metadata
        .as_ref()
        .map(|m| m.response_size_bytes)
        .unwrap_or(0);

    CacheProcessingHints {
        write_priority: Some(50),
        compress: config.enable_compression
            && response_size > config.compression_threshold_bytes,
        encrypt: false, // Would be determined by sensitivity analysis
        background_refresh: false,
        prefetch: false,
    }
}

/// Build bypass output
fn build_bypass_output(
    input: &CacheStrategyInput,
    reason: &str,
    constraints: &[Constraint],
) -> CacheStrategyOutput {
    CacheStrategyOutput {
        decision: CacheDecision::CacheBypass,
        confidence: 1.0,
        reason: reason.to_string(),
        is_cacheable: false,
        ineligibility_reasons: vec![],
        directive: CacheDirective::cache_bypass(),
        cache_key: None,
        recommended_layers: vec![],
        ttl_seconds: None,
        used_semantic_matching: false,
        semantic_similarity: None,
        processing_hints: None,
    }
}

/// Build invalidate output
fn build_invalidate_output(
    input: &CacheStrategyInput,
    constraints: &[Constraint],
) -> CacheStrategyOutput {
    CacheStrategyOutput {
        decision: CacheDecision::CacheInvalidate,
        confidence: 1.0,
        reason: "Cache invalidation requested".to_string(),
        is_cacheable: false,
        ineligibility_reasons: vec![],
        directive: CacheDirective::cache_invalidate(),
        cache_key: Some(generate_cache_key(input, &CacheStrategyConfig::default())),
        recommended_layers: vec![CacheLayer::L1, CacheLayer::L2],
        ttl_seconds: None,
        used_semantic_matching: false,
        semantic_similarity: None,
        processing_hints: None,
    }
}

/// Calculate decision confidence
///
/// Confidence is based on:
/// - Constraint pass/fail status
/// - Validation error count
/// - Cache entry characteristics
fn calculate_confidence(
    constraints: &[Constraint],
    validation: &CacheValidationResult,
) -> f64 {
    let mut confidence = 1.0;

    // Reduce confidence for each failed constraint
    for constraint in constraints {
        if !constraint.passed {
            if let Some(severity) = constraint.severity {
                confidence -= severity * 0.2;
            } else {
                confidence -= 0.1;
            }
        }
    }

    // Reduce confidence for validation warnings
    confidence -= (validation.warnings.len() as f64) * 0.05;

    // Ensure confidence is in valid range
    confidence.max(0.0).min(1.0)
}

/// Get constraints evaluated for a decision
pub fn get_constraints_evaluated(context: &DecisionContext) -> Vec<Constraint> {
    let mut constraints = Vec::new();

    // Validation constraint
    constraints.push(Constraint {
        constraint_type: ConstraintType::Cache,
        name: "cache_eligibility_validation".to_string(),
        passed: context.validation.valid,
        details: Some(if context.validation.valid {
            "Validation passed".to_string()
        } else {
            format!("{} errors", context.validation.errors.len())
        }),
        severity: if context.validation.valid { None } else { Some(0.5) },
    });

    // Policy constraint
    constraints.push(Constraint {
        constraint_type: ConstraintType::Policy,
        name: "cache_policy_enabled".to_string(),
        passed: context.input.policy.enabled,
        details: Some(if context.input.policy.enabled {
            "Caching enabled".to_string()
        } else {
            "Caching disabled".to_string()
        }),
        severity: if context.input.policy.enabled { None } else { Some(0.4) },
    });

    constraints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching_strategy::types::{CachePolicyConfig, CacheRelevantParams};

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
    fn test_cache_miss_decision() {
        let input = default_input();
        let config = CacheStrategyConfig::default();
        let validation = CacheValidationResult::default();

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: false,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheMiss);
        assert!(output.is_cacheable);
        assert!(output.cache_key.is_some());
        assert!(!output.recommended_layers.is_empty());
    }

    #[test]
    fn test_cache_hit_decision() {
        let mut input = default_input();
        input.existing_entry = Some(super::super::types::CacheEntryMetadata {
            cache_key: "cache:test:key".to_string(),
            layer: CacheLayer::L1,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            last_accessed_at: Some(Utc::now()),
            access_count: 5,
            response_size_bytes: 1000,
            is_valid: true,
            is_stale: false,
            content_hash: "hash123".to_string(),
            semantic_hash: None,
        });

        let config = CacheStrategyConfig::default();
        let validation = CacheValidationResult::default();

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: false,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheHit);
        assert!(output.directive.read);
        assert!(!output.directive.write);
    }

    #[test]
    fn test_cache_bypass_forced() {
        let input = default_input();
        let config = CacheStrategyConfig::default();
        let validation = CacheValidationResult::default();

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: true,
            force_refresh: false,
            force_invalidate: false,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheBypass);
        assert!(output.directive.bypass);
    }

    #[test]
    fn test_cache_invalidate() {
        let input = default_input();
        let config = CacheStrategyConfig::default();
        let validation = CacheValidationResult::default();

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: true,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheInvalidate);
        assert!(output.directive.invalidate);
    }

    #[test]
    fn test_cache_write_decision() {
        let mut input = default_input();
        input.is_write_operation = true;
        input.response_metadata = Some(super::super::types::ResponseMetadata {
            response_size_bytes: 1000,
            latency_ms: 100,
            token_count: Some(100),
            success: true,
            contains_sensitive_data: false,
            finish_reason: Some("stop".to_string()),
        });

        let config = CacheStrategyConfig::default();
        let validation = CacheValidationResult::default();

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: false,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheWrite);
        assert!(output.directive.write);
        assert!(!output.directive.read);
    }

    #[test]
    fn test_cache_key_generation() {
        let input = default_input();
        let config = CacheStrategyConfig::default();

        let key = generate_cache_key(&input, &config);

        assert!(key.starts_with("cache:"));
        assert!(key.contains("chat_completion"));
        assert!(key.contains("hash123"));
    }

    #[test]
    fn test_validation_failed_bypass() {
        let input = default_input();
        let config = CacheStrategyConfig::default();
        let mut validation = CacheValidationResult::default();
        validation.valid = false;
        validation.errors.push(super::super::types::CacheValidationError {
            code: "TEMPERATURE_TOO_HIGH".to_string(),
            message: "Temperature too high".to_string(),
            field: None,
            severity: 0.5,
        });

        let context = DecisionContext {
            input: &input,
            config: &config,
            validation: &validation,
            force_bypass: false,
            force_refresh: false,
            force_invalidate: false,
        };

        let output = make_decision(context).unwrap();

        assert_eq!(output.decision, CacheDecision::CacheBypass);
        assert!(!output.is_cacheable);
        assert!(!output.ineligibility_reasons.is_empty());
    }

    #[test]
    fn test_confidence_calculation() {
        let constraints = vec![
            Constraint {
                constraint_type: ConstraintType::Cache,
                name: "test".to_string(),
                passed: true,
                details: None,
                severity: None,
            },
            Constraint {
                constraint_type: ConstraintType::Policy,
                name: "policy".to_string(),
                passed: false,
                details: None,
                severity: Some(0.5),
            },
        ];
        let validation = CacheValidationResult::default();

        let confidence = calculate_confidence(&constraints, &validation);

        assert!(confidence < 1.0);
        assert!(confidence > 0.5);
    }
}
