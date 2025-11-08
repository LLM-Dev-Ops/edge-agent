use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("No providers available")]
    NoProvidersAvailable,

    #[error("All providers failed")]
    AllProvidersFailed,

    #[error("Circuit breaker open for provider: {0}")]
    CircuitBreakerOpen(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type RoutingResult<T> = Result<T, RoutingError>;
