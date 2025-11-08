//! Security layer for LLM Edge Agent
//!
//! Provides:
//! - API key authentication
//! - JWT token validation
//! - OAuth2/OIDC (future)
//! - PII detection and redaction
//! - Input validation

pub mod auth;
pub mod pii;
pub mod validation;
pub mod error;

pub use error::{SecurityError, SecurityResult};
pub use auth::{ApiKeyAuth, JwtAuth};
pub use pii::PIIRedactor;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert_eq!(2 + 2, 4);
    }
}
