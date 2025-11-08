//! Structured logging with PII redaction
//!
//! Provides secure logging capabilities:
//! - Structured JSON logs for production
//! - Request/response logging
//! - Automatic PII redaction
//! - Sensitive data masking
//! - Correlation IDs for request tracking

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::{debug, info, warn};

/// PII patterns to redact from logs
static PII_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

/// Initialize PII redaction patterns
fn init_pii_patterns() -> Vec<(Regex, &'static str)> {
    vec![
        // Email addresses
        (
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
            "[EMAIL_REDACTED]",
        ),
        // Credit card numbers (simple pattern)
        (
            Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap(),
            "[CREDIT_CARD_REDACTED]",
        ),
        // Social Security Numbers (US)
        (
            Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            "[SSN_REDACTED]",
        ),
        // Phone numbers (various formats)
        (
            Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap(),
            "[PHONE_REDACTED]",
        ),
        // API keys (common patterns)
        (
            Regex::new(r"(?i)(api[_-]?key|apikey|api[_-]?secret)[\s:=\"']+([a-zA-Z0-9_-]{16,})").unwrap(),
            "$1=[API_KEY_REDACTED]",
        ),
        // Bearer tokens
        (
            Regex::new(r"(?i)bearer\s+([a-zA-Z0-9_-]+)").unwrap(),
            "Bearer [TOKEN_REDACTED]",
        ),
        // IP addresses (optional - may want to log these)
        (
            Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
            "[IP_REDACTED]",
        ),
    ]
}

/// Redact PII from text
pub fn redact_pii(text: &str) -> String {
    let patterns = PII_PATTERNS.get_or_init(init_pii_patterns);
    
    let mut result = text.to_string();
    for (pattern, replacement) in patterns {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    
    result
}

/// Request log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestLog {
    /// Unique request ID
    pub request_id: String,
    
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    
    /// HTTP method
    pub method: String,
    
    /// Request path
    pub path: String,
    
    /// Client IP (redacted if configured)
    pub client_ip: Option<String>,
    
    /// User agent
    pub user_agent: Option<String>,
    
    /// Request size in bytes
    pub request_size: Option<usize>,
    
    /// Provider used
    pub provider: Option<String>,
    
    /// Model used
    pub model: Option<String>,
}

impl RequestLog {
    /// Create a new request log entry
    pub fn new(
        request_id: String,
        method: String,
        path: String,
    ) -> Self {
        Self {
            request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            method,
            path,
            client_ip: None,
            user_agent: None,
            request_size: None,
            provider: None,
            model: None,
        }
    }
    
    /// Log the request
    pub fn log(&self) {
        info!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            provider = ?self.provider,
            model = ?self.model,
            "Incoming request"
        );
    }
}

/// Response log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseLog {
    /// Unique request ID
    pub request_id: String,
    
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    
    /// HTTP status code
    pub status_code: u16,
    
    /// Response size in bytes
    pub response_size: Option<usize>,
    
    /// Duration in milliseconds
    pub duration_ms: u64,
    
    /// Whether response came from cache
    pub cache_hit: bool,
    
    /// Cache tier if applicable
    pub cache_tier: Option<String>,
    
    /// Token usage
    pub tokens_used: Option<TokenUsage>,
    
    /// Cost in cents
    pub cost_cents: Option<f64>,
    
    /// Error message if failed
    pub error: Option<String>,
}

impl ResponseLog {
    /// Create a new response log entry
    pub fn new(
        request_id: String,
        status_code: u16,
        duration_ms: u64,
    ) -> Self {
        Self {
            request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            status_code,
            response_size: None,
            duration_ms,
            cache_hit: false,
            cache_tier: None,
            tokens_used: None,
            cost_cents: None,
            error: None,
        }
    }
    
    /// Log the response
    pub fn log(&self) {
        if self.status_code >= 500 {
            tracing::error!(
                request_id = %self.request_id,
                status_code = self.status_code,
                duration_ms = self.duration_ms,
                error = ?self.error,
                "Request failed"
            );
        } else if self.status_code >= 400 {
            warn!(
                request_id = %self.request_id,
                status_code = self.status_code,
                duration_ms = self.duration_ms,
                "Request error"
            );
        } else {
            info!(
                request_id = %self.request_id,
                status_code = self.status_code,
                duration_ms = self.duration_ms,
                cache_hit = self.cache_hit,
                tokens = ?self.tokens_used,
                cost_cents = ?self.cost_cents,
                "Request completed"
            );
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Error log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorLog {
    /// Unique request ID
    pub request_id: Option<String>,
    
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    
    /// Error type/category
    pub error_type: String,
    
    /// Error message (PII redacted)
    pub message: String,
    
    /// Stack trace if available
    pub stack_trace: Option<String>,
    
    /// Additional context
    pub context: Option<serde_json::Value>,
}

impl ErrorLog {
    /// Create a new error log entry
    pub fn new(
        request_id: Option<String>,
        error_type: String,
        message: String,
    ) -> Self {
        Self {
            request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            error_type,
            message: redact_pii(&message),
            stack_trace: None,
            context: None,
        }
    }
    
    /// Log the error
    pub fn log(&self) {
        tracing::error!(
            request_id = ?self.request_id,
            error_type = %self.error_type,
            message = %self.message,
            "Error occurred"
        );
    }
}

/// Provider request log
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderRequestLog {
    /// Request ID
    pub request_id: String,
    
    /// Provider name
    pub provider: String,
    
    /// Model
    pub model: String,
    
    /// Attempt number
    pub attempt: u32,
    
    /// Timestamp
    pub timestamp: String,
}

impl ProviderRequestLog {
    /// Create a new provider request log
    pub fn new(
        request_id: String,
        provider: String,
        model: String,
        attempt: u32,
    ) -> Self {
        Self {
            request_id,
            provider,
            model,
            attempt,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    /// Log the provider request
    pub fn log(&self) {
        debug!(
            request_id = %self.request_id,
            provider = %self.provider,
            model = %self.model,
            attempt = self.attempt,
            "Sending request to provider"
        );
    }
}

/// Sanitize log data before writing
pub fn sanitize_log_data(data: &str, max_length: usize) -> String {
    let redacted = redact_pii(data);
    
    if redacted.len() > max_length {
        format!("{}... [truncated]", &redacted[..max_length])
    } else {
        redacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_redaction() {
        let text = "Contact us at support@example.com for help";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[EMAIL_REDACTED]"));
        assert!(!redacted.contains("support@example.com"));
    }
    
    #[test]
    fn test_phone_redaction() {
        let text = "Call me at 555-123-4567";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[PHONE_REDACTED]"));
        assert!(!redacted.contains("555-123-4567"));
    }
    
    #[test]
    fn test_api_key_redaction() {
        let text = "API_KEY=sk_test_1234567890abcdef";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[API_KEY_REDACTED]"));
        assert!(!redacted.contains("sk_test_1234567890abcdef"));
    }
    
    #[test]
    fn test_sanitize_long_data() {
        let long_text = "a".repeat(1000);
        let sanitized = sanitize_log_data(&long_text, 100);
        assert!(sanitized.len() <= 120); // 100 + "... [truncated]"
        assert!(sanitized.contains("[truncated]"));
    }
    
    #[test]
    fn test_request_log_creation() {
        let log = RequestLog::new(
            "req-123".to_string(),
            "POST".to_string(),
            "/v1/chat/completions".to_string(),
        );
        assert_eq!(log.request_id, "req-123");
        assert_eq!(log.method, "POST");
    }
}
