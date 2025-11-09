//! Authentication middleware using API keys

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use crate::error::ProxyError;
use crate::Config;

const API_KEY_HEADER: &str = "x-api-key";
const BEARER_PREFIX: &str = "Bearer ";

/// Authentication middleware
///
/// Validates API keys from either:
/// - x-api-key header
/// - Authorization: Bearer <key> header
///
/// Public endpoints (health, metrics) are always allowed.
pub async fn auth_middleware(
    State(config): State<Config>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    // Skip auth if disabled
    if !config.auth.enabled {
        debug!("Authentication disabled, allowing request");
        return Ok(next.run(request).await);
    }

    // Get the request path
    let path = request.uri().path();

    // Allow health and metrics endpoints without auth
    if !config.auth.require_auth_for_health && (path.starts_with("/health") || path == "/metrics") {
        debug!(path = %path, "Public endpoint, skipping auth");
        return Ok(next.run(request).await);
    }

    // Extract API key from headers
    let api_key = extract_api_key(&headers)?;

    // Validate API key
    if !validate_api_key(&api_key, &config.auth.api_keys) {
        warn!(
            path = %path,
            "Invalid API key attempted"
        );
        return Err(ProxyError::Authentication("Invalid API key".to_string()));
    }

    debug!(path = %path, "Authentication successful");
    Ok(next.run(request).await)
}

/// Extract API key from request headers
fn extract_api_key(headers: &HeaderMap) -> Result<String, crate::error::ProxyError> {
    // Try x-api-key header first
    if let Some(key) = headers.get(API_KEY_HEADER) {
        let key_str = key
            .to_str()
            .map_err(|_| ProxyError::Authentication("Invalid API key format".to_string()))?;
        return Ok(key_str.to_string());
    }

    // Try Authorization: Bearer header
    if let Some(auth) = headers.get("authorization") {
        let auth_str = auth
            .to_str()
            .map_err(|_| ProxyError::Authentication("Invalid authorization header".to_string()))?;

        if let Some(key) = auth_str.strip_prefix(BEARER_PREFIX) {
            return Ok(key.to_string());
        }
    }

    Err(ProxyError::Authentication(
        "Missing API key. Provide either 'x-api-key' header or 'Authorization: Bearer <key>' header".to_string(),
    ))
}

/// Validate API key against configured keys
///
/// Supports both plain-text and SHA-256 hashed keys
fn validate_api_key(provided_key: &str, valid_keys: &[String]) -> bool {
    if valid_keys.is_empty() {
        // If no keys configured, allow all (dev mode)
        return true;
    }

    // Check direct match first (for plain-text keys)
    if valid_keys.iter().any(|k| k == provided_key) {
        return true;
    }

    // Check SHA-256 hash match (for hashed keys)
    let provided_hash = hash_api_key(provided_key);
    valid_keys.iter().any(|k| k == &provided_hash)
}

/// Hash API key using SHA-256
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_key() {
        let key = "test-key-123";
        let hash = hash_api_key(key);
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_validate_api_key_plain() {
        let valid_keys = vec!["key1".to_string(), "key2".to_string()];
        assert!(validate_api_key("key1", &valid_keys));
        assert!(validate_api_key("key2", &valid_keys));
        assert!(!validate_api_key("key3", &valid_keys));
    }

    #[test]
    fn test_validate_api_key_empty() {
        let valid_keys = vec![];
        // Empty keys allows all (dev mode)
        assert!(validate_api_key("any-key", &valid_keys));
    }

    #[test]
    fn test_validate_api_key_hashed() {
        let key = "secret-key";
        let hashed = hash_api_key(key);
        let valid_keys = vec![hashed];

        assert!(validate_api_key(key, &valid_keys));
        assert!(!validate_api_key("wrong-key", &valid_keys));
    }
}
