use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// Error response structure
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ProxyError {
    fn error_code(&self) -> &str {
        match self {
            ProxyError::Http(_) => "HTTP_ERROR",
            ProxyError::Config(_) => "CONFIG_ERROR",
            ProxyError::Authentication(_) => "AUTH_ERROR",
            ProxyError::RateLimit(_) => "RATE_LIMIT_EXCEEDED",
            ProxyError::Validation(_) => "VALIDATION_ERROR",
            ProxyError::Timeout => "TIMEOUT",
            ProxyError::BadRequest(_) => "BAD_REQUEST",
            ProxyError::InvalidRequest(_) => "INVALID_REQUEST",
            ProxyError::Internal(_) => "INTERNAL_ERROR",
            ProxyError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::Http(_) => StatusCode::BAD_GATEWAY,
            ProxyError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Authentication(_) => StatusCode::UNAUTHORIZED,
            ProxyError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,
            ProxyError::Validation(_) => StatusCode::BAD_REQUEST,
            ProxyError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.to_string(),
                details: None,
            },
        };

        (status, Json(error_response)).into_response()
    }
}

// Implement From conversions for common error types
impl From<std::io::Error> for ProxyError {
    fn from(err: std::io::Error) -> Self {
        ProxyError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for ProxyError {
    fn from(err: serde_json::Error) -> Self {
        ProxyError::BadRequest(err.to_string())
    }
}

pub type ProxyResult<T> = Result<T, ProxyError>;
