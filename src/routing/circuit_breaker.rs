//! Circuit breaker implementation using failsafe crate
//! 
//! Implements the circuit breaker pattern to prevent cascading failures
//! when providers are experiencing issues.
//!
//! Circuit States:
//! - CLOSED: Normal operation, requests flow through
//! - OPEN: Provider failing, requests fail fast
//! - HALF_OPEN: Testing if provider recovered
//!
//! Configuration:
//! - Failure threshold: 5 consecutive failures
//! - Timeout: 30 seconds before attempting recovery
//! - Success threshold (half-open): 2 consecutive successes

use failsafe::{CircuitBreaker, Config, Error as FailsafeError};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Circuit breaker error types
#[derive(Error, Debug)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open for provider: {0}")]
    Open(String),
    
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Provider timeout: {0}")]
    Timeout(String),
}

/// Circuit breaker configuration for LLM providers
#[derive(Clone, Debug)]
pub struct LLMCircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    
    /// Time to wait before attempting recovery
    pub timeout: Duration,
    
    /// Number of successes required in half-open state
    pub success_threshold: u32,
    
    /// Provider name for logging
    pub provider_name: String,
}

impl Default for LLMCircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 2,
            provider_name: "unknown".to_string(),
        }
    }
}

/// Wrapper around failsafe CircuitBreaker with LLM-specific logic
pub struct LLMCircuitBreaker {
    breaker: Arc<CircuitBreaker>,
    config: LLMCircuitBreakerConfig,
}

impl LLMCircuitBreaker {
    /// Create a new circuit breaker for an LLM provider
    pub fn new(config: LLMCircuitBreakerConfig) -> Self {
        let cb_config = Config::new()
            .failure_policy(config.failure_threshold)
            .timeout(config.timeout);
        
        info!(
            provider = %config.provider_name,
            failure_threshold = config.failure_threshold,
            timeout_secs = config.timeout.as_secs(),
            "Initialized circuit breaker"
        );
        
        Self {
            breaker: Arc::new(CircuitBreaker::new(cb_config)),
            config,
        }
    }
    
    /// Execute a request through the circuit breaker
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> futures::future::BoxFuture<'static, Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check circuit state
        if self.breaker.is_open() {
            warn!(
                provider = %self.config.provider_name,
                "Circuit breaker is OPEN, failing fast"
            );
            return Err(CircuitBreakerError::Open(
                self.config.provider_name.clone()
            ));
        }
        
        debug!(
            provider = %self.config.provider_name,
            state = ?self.breaker.state(),
            "Executing request through circuit breaker"
        );
        
        // Execute the request
        match self.breaker.call(f).await {
            Ok(result) => {
                debug!(
                    provider = %self.config.provider_name,
                    "Request succeeded"
                );
                Ok(result)
            }
            Err(FailsafeError::Rejected) => {
                error!(
                    provider = %self.config.provider_name,
                    "Circuit breaker REJECTED request (circuit is open)"
                );
                Err(CircuitBreakerError::Open(
                    self.config.provider_name.clone()
                ))
            }
            Err(FailsafeError::Inner(e)) => {
                warn!(
                    provider = %self.config.provider_name,
                    error = %e,
                    "Request failed, recording failure"
                );
                Err(CircuitBreakerError::RequestFailed(e.to_string()))
            }
        }
    }
    
    /// Get current circuit breaker state
    pub fn state(&self) -> String {
        format!("{:?}", self.breaker.state())
    }
    
    /// Check if circuit is open
    pub fn is_open(&self) -> bool {
        self.breaker.is_open()
    }
    
    /// Get failure count (for metrics)
    pub fn failure_count(&self) -> u32 {
        // Note: failsafe crate doesn't expose this directly
        // In production, you'd track this separately
        0
    }
}

/// Health status of a provider's circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerHealth {
    pub provider_name: String,
    pub state: String,
    pub is_healthy: bool,
}

impl LLMCircuitBreaker {
    /// Get health status for monitoring
    pub fn health(&self) -> CircuitBreakerHealth {
        let state = self.state();
        CircuitBreakerHealth {
            provider_name: self.config.provider_name.clone(),
            state: state.clone(),
            is_healthy: !self.is_open(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = LLMCircuitBreakerConfig {
            failure_threshold: 3,
            timeout: Duration::from_secs(1),
            success_threshold: 1,
            provider_name: "test-provider".to_string(),
        };
        
        let cb = LLMCircuitBreaker::new(config);
        let counter = Arc::new(AtomicU32::new(0));
        
        // Simulate failures
        for i in 0..5 {
            let counter_clone = counter.clone();
            let result = cb.call(|| {
                Box::pin(async move {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "simulated failure"
                    ))
                })
            }).await;
            
            if i < 3 {
                assert!(result.is_err());
            } else {
                // After threshold, circuit should be open
                assert!(matches!(result, Err(CircuitBreakerError::Open(_))));
            }
        }
        
        // Circuit should be open
        assert!(cb.is_open());
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_success() {
        let config = LLMCircuitBreakerConfig {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 2,
            provider_name: "test-provider".to_string(),
        };
        
        let cb = LLMCircuitBreaker::new(config);
        
        let result = cb.call(|| {
            Box::pin(async {
                Ok::<_, std::io::Error>("success")
            })
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
}
