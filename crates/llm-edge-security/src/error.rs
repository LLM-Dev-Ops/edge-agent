use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Invalid JWT token: {0}")]
    InvalidJwt(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type SecurityResult<T> = Result<T, SecurityError>;
