//! Cache key generation using SHA-256 hashing
//!
//! This module provides efficient cache key generation from LLM prompts and parameters.
//! Uses SHA-256 for consistent, collision-resistant hashing.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Represents a cacheable LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheableRequest {
    /// The model name (e.g., "gpt-4", "claude-3-sonnet")
    pub model: String,
    /// The prompt or messages
    pub prompt: String,
    /// Temperature parameter
    pub temperature: Option<f32>,
    /// Max tokens to generate
    pub max_tokens: Option<u32>,
    /// Additional parameters that affect the response
    pub parameters: HashMap<String, serde_json::Value>,
}

impl CacheableRequest {
    /// Create a new cacheable request
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            temperature: None,
            max_tokens: None,
            parameters: HashMap::new(),
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Add a custom parameter
    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }
}

/// Generate a cache key from a request using SHA-256
///
/// The key includes:
/// - Model name
/// - Prompt content
/// - Temperature (normalized to 2 decimal places)
/// - Max tokens
/// - All additional parameters (sorted for consistency)
///
/// # Performance
/// - Target: <100μs for typical requests
/// - SHA-256 is hardware-accelerated on most modern CPUs
pub fn generate_cache_key(request: &CacheableRequest) -> String {
    let mut hasher = Sha256::new();

    // Add model name
    hasher.update(request.model.as_bytes());
    hasher.update(b"|");

    // Add prompt
    hasher.update(request.prompt.as_bytes());
    hasher.update(b"|");

    // Add temperature (normalized to 2 decimals to avoid floating point precision issues)
    if let Some(temp) = request.temperature {
        hasher.update(format!("{:.2}", temp).as_bytes());
    }
    hasher.update(b"|");

    // Add max_tokens
    if let Some(max_tokens) = request.max_tokens {
        hasher.update(max_tokens.to_string().as_bytes());
    }
    hasher.update(b"|");

    // Add sorted parameters for deterministic hashing
    let mut param_keys: Vec<_> = request.parameters.keys().collect();
    param_keys.sort();
    for key in param_keys {
        if let Some(value) = request.parameters.get(key) {
            hasher.update(key.as_bytes());
            hasher.update(b"=");
            // Serialize value to JSON for consistent representation
            if let Ok(json_str) = serde_json::to_string(value) {
                hasher.update(json_str.as_bytes());
            }
            hasher.update(b";");
        }
    }

    // Return hex-encoded hash
    let result = hasher.finalize();
    hex::encode(result)
}

/// Generate a short cache key (first 16 characters of the full hash)
/// Useful for logging and debugging
pub fn generate_short_key(request: &CacheableRequest) -> String {
    let full_key = generate_cache_key(request);
    full_key.chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_consistency() {
        let req1 = CacheableRequest::new("gpt-4", "Hello, world!")
            .with_temperature(0.7)
            .with_max_tokens(100);

        let req2 = CacheableRequest::new("gpt-4", "Hello, world!")
            .with_temperature(0.7)
            .with_max_tokens(100);

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_eq!(
            key1, key2,
            "Identical requests should produce identical keys"
        );
    }

    #[test]
    fn test_cache_key_different_prompts() {
        let req1 = CacheableRequest::new("gpt-4", "Hello, world!");
        let req2 = CacheableRequest::new("gpt-4", "Goodbye, world!");

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_ne!(
            key1, key2,
            "Different prompts should produce different keys"
        );
    }

    #[test]
    fn test_cache_key_different_models() {
        let req1 = CacheableRequest::new("gpt-4", "Hello, world!");
        let req2 = CacheableRequest::new("gpt-3.5-turbo", "Hello, world!");

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_ne!(key1, key2, "Different models should produce different keys");
    }

    #[test]
    fn test_cache_key_temperature_normalization() {
        let req1 = CacheableRequest::new("gpt-4", "Hello").with_temperature(0.7);
        let req2 = CacheableRequest::new("gpt-4", "Hello").with_temperature(0.700001);

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_eq!(
            key1, key2,
            "Temperature should be normalized to avoid precision issues"
        );
    }

    #[test]
    fn test_cache_key_parameter_order_independence() {
        let mut req1 = CacheableRequest::new("gpt-4", "Hello");
        req1.parameters
            .insert("param_a".to_string(), serde_json::json!("value1"));
        req1.parameters
            .insert("param_b".to_string(), serde_json::json!("value2"));

        let mut req2 = CacheableRequest::new("gpt-4", "Hello");
        req2.parameters
            .insert("param_b".to_string(), serde_json::json!("value2"));
        req2.parameters
            .insert("param_a".to_string(), serde_json::json!("value1"));

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_eq!(key1, key2, "Parameter order should not affect cache key");
    }

    #[test]
    fn test_short_key_length() {
        let req = CacheableRequest::new("gpt-4", "Test prompt");
        let short_key = generate_short_key(&req);

        assert_eq!(short_key.len(), 16, "Short key should be 16 characters");
    }

    #[test]
    fn test_cache_key_is_hexadecimal() {
        let req = CacheableRequest::new("gpt-4", "Test prompt");
        let key = generate_cache_key(&req);

        assert!(
            key.chars().all(|c| c.is_ascii_hexdigit()),
            "Cache key should be valid hexadecimal"
        );
        assert_eq!(key.len(), 64, "SHA-256 hash should be 64 hex characters");
    }
}
