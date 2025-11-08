//! Routing engine for LLM Edge Agent
//!
//! This module provides intelligent routing capabilities for LLM requests:
//! - Multiple routing strategies (round-robin, failover, least-latency, cost-optimized)
//! - Circuit breaker pattern for resilience
//! - Provider health monitoring
//! - Automatic failover and retry with exponential backoff

pub mod circuit_breaker;
pub mod strategies;

use crate::routing::circuit_breaker::{CircuitBreakerHealth, LLMCircuitBreaker, LLMCircuitBreakerConfig};
use crate::routing::strategies::{
    Provider, ProviderWithHealth, RoutingStrategy, RoundRobinStrategy,
    FailoverChainStrategy, LeastLatencyStrategy, CostOptimizedStrategy, RetryConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, instrument};
use thiserror::Error;

/// Routing engine errors
#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("No providers available")]
    NoProvidersAvailable,
    
    #[error("All providers failed after retries")]
    AllProvidersFailed,
    
    #[error("Provider {0} circuit breaker is open")]
    CircuitBreakerOpen(String),
    
    #[error("Request timeout")]
    Timeout,
    
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// Health metrics for a provider
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub last_success: Option<Instant>,
    pub last_failure: Option<Instant>,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            last_success: None,
            last_failure: None,
        }
    }
}

impl ProviderHealth {
    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
    
    /// Determine if provider is healthy
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if:
        // - No requests yet, OR
        // - Success rate >= 80%, OR
        // - Last request was successful and within last 5 minutes
        if self.total_requests == 0 {
            return true;
        }
        
        if self.success_rate() >= 0.8 {
            return true;
        }
        
        if let Some(last_success) = self.last_success {
            if last_success.elapsed() < Duration::from_secs(300) {
                return true;
            }
        }
        
        false
    }
}

/// Main routing engine
pub struct RoutingEngine {
    /// Available providers
    providers: Arc<RwLock<Vec<Provider>>>,
    
    /// Circuit breakers per provider
    circuit_breakers: Arc<RwLock<HashMap<String, LLMCircuitBreaker>>>,
    
    /// Health metrics per provider
    health_metrics: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    
    /// Current routing strategy
    strategy: Arc<dyn RoutingStrategy>,
    
    /// Retry configuration
    retry_config: RetryConfig,
}

impl RoutingEngine {
    /// Create a new routing engine with specified strategy
    pub fn new(
        providers: Vec<Provider>,
        strategy: Arc<dyn RoutingStrategy>,
        retry_config: RetryConfig,
    ) -> Self {
        info!(
            provider_count = providers.len(),
            strategy = strategy.name(),
            "Initializing routing engine"
        );
        
        // Initialize circuit breakers for each provider
        let mut circuit_breakers = HashMap::new();
        for provider in &providers {
            let config = LLMCircuitBreakerConfig {
                failure_threshold: 5,
                timeout: Duration::from_secs(30),
                success_threshold: 2,
                provider_name: provider.id.clone(),
            };
            circuit_breakers.insert(
                provider.id.clone(),
                LLMCircuitBreaker::new(config),
            );
        }
        
        Self {
            providers: Arc::new(RwLock::new(providers)),
            circuit_breakers: Arc::new(RwLock::new(circuit_breakers)),
            health_metrics: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            retry_config,
        }
    }
    
    /// Create engine with round-robin strategy
    pub fn with_round_robin(providers: Vec<Provider>) -> Self {
        Self::new(
            providers,
            Arc::new(RoundRobinStrategy::new()),
            RetryConfig::default(),
        )
    }
    
    /// Create engine with failover chain strategy
    pub fn with_failover(providers: Vec<Provider>) -> Self {
        Self::new(
            providers,
            Arc::new(FailoverChainStrategy::new(3)),
            RetryConfig::default(),
        )
    }
    
    /// Create engine with least latency strategy
    pub fn with_least_latency(providers: Vec<Provider>) -> Self {
        Self::new(
            providers,
            Arc::new(LeastLatencyStrategy::new()),
            RetryConfig::default(),
        )
    }
    
    /// Create engine with cost-optimized strategy
    pub fn with_cost_optimized(providers: Vec<Provider>) -> Self {
        Self::new(
            providers,
            Arc::new(CostOptimizedStrategy::new()),
            RetryConfig::default(),
        )
    }
    
    /// Route a request to an appropriate provider
    #[instrument(skip(self, request_fn), fields(strategy = self.strategy.name()))]
    pub async fn route<F, T, E>(
        &self,
        request_fn: F,
    ) -> Result<T, RoutingError>
    where
        F: Fn(Provider) -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
        E: std::error::Error + Send + Sync + 'static,
        T: Send,
    {
        let mut attempt = 0;
        let mut last_error = None;
        
        while attempt < self.retry_config.max_retries {
            // Select provider
            let provider = self.select_provider().await?;
            
            debug!(
                provider = %provider.id,
                attempt = attempt + 1,
                max_retries = self.retry_config.max_retries,
                "Attempting request"
            );
            
            // Execute request through circuit breaker
            let start = Instant::now();
            let result = self.execute_with_circuit_breaker(&provider, &request_fn).await;
            let latency = start.elapsed();
            
            match result {
                Ok(value) => {
                    // Record success
                    self.record_success(&provider.id, latency).await;
                    self.strategy.record_result(&provider.id, latency, true).await;
                    
                    info!(
                        provider = %provider.id,
                        latency_ms = latency.as_millis(),
                        "Request succeeded"
                    );
                    
                    return Ok(value);
                }
                Err(e) => {
                    // Record failure
                    self.record_failure(&provider.id, latency).await;
                    self.strategy.record_result(&provider.id, latency, false).await;
                    
                    warn!(
                        provider = %provider.id,
                        attempt = attempt + 1,
                        error = %e,
                        "Request failed"
                    );
                    
                    last_error = Some(e);
                    attempt += 1;
                    
                    // Exponential backoff before retry
                    if attempt < self.retry_config.max_retries {
                        let backoff = self.retry_config.backoff_duration(attempt - 1);
                        debug!(
                            backoff_ms = backoff.as_millis(),
                            "Backing off before retry"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
        
        error!(
            attempts = attempt,
            "All retry attempts exhausted"
        );
        
        Err(RoutingError::AllProvidersFailed)
    }
    
    /// Select a provider using the current strategy
    async fn select_provider(&self) -> Result<Provider, RoutingError> {
        let providers = self.providers.read().await;
        let health_metrics = self.health_metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;
        
        // Build list of providers with health status
        let providers_with_health: Vec<ProviderWithHealth> = providers
            .iter()
            .map(|p| {
                let health = health_metrics
                    .get(&p.id)
                    .cloned()
                    .unwrap_or_default();
                
                let circuit_healthy = circuit_breakers
                    .get(&p.id)
                    .map(|cb| !cb.is_open())
                    .unwrap_or(true);
                
                ProviderWithHealth {
                    provider: p.clone(),
                    is_healthy: health.is_healthy() && circuit_healthy,
                    avg_latency_ms: health.avg_latency_ms,
                    success_rate: health.success_rate(),
                }
            })
            .collect();
        
        self.strategy
            .select_provider(&providers_with_health)
            .await
            .ok_or(RoutingError::NoProvidersAvailable)
    }
    
    /// Execute request through circuit breaker
    async fn execute_with_circuit_breaker<F, T, E>(
        &self,
        provider: &Provider,
        request_fn: &F,
    ) -> Result<T, RoutingError>
    where
        F: Fn(Provider) -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
        E: std::error::Error + Send + Sync + 'static,
        T: Send,
    {
        let circuit_breakers = self.circuit_breakers.read().await;
        let cb = circuit_breakers
            .get(&provider.id)
            .ok_or_else(|| RoutingError::ProviderError("Circuit breaker not found".to_string()))?;
        
        let provider_clone = provider.clone();
        cb.call(|| {
            let p = provider_clone.clone();
            request_fn(p)
        })
        .await
        .map_err(|e| match e {
            circuit_breaker::CircuitBreakerError::Open(name) => {
                RoutingError::CircuitBreakerOpen(name)
            }
            circuit_breaker::CircuitBreakerError::RequestFailed(msg) => {
                RoutingError::ProviderError(msg)
            }
            circuit_breaker::CircuitBreakerError::Timeout(msg) => {
                RoutingError::ProviderError(msg)
            }
        })
    }
    
    /// Record successful request
    async fn record_success(&self, provider_id: &str, latency: Duration) {
        let mut metrics = self.health_metrics.write().await;
        let health = metrics.entry(provider_id.to_string()).or_default();
        
        health.total_requests += 1;
        health.successful_requests += 1;
        health.last_success = Some(Instant::now());
        
        // Update average latency (exponential moving average)
        let alpha = 0.3; // Smoothing factor
        if health.avg_latency_ms == 0.0 {
            health.avg_latency_ms = latency.as_millis() as f64;
        } else {
            health.avg_latency_ms = alpha * latency.as_millis() as f64
                + (1.0 - alpha) * health.avg_latency_ms;
        }
    }
    
    /// Record failed request
    async fn record_failure(&self, provider_id: &str, latency: Duration) {
        let mut metrics = self.health_metrics.write().await;
        let health = metrics.entry(provider_id.to_string()).or_default();
        
        health.total_requests += 1;
        health.failed_requests += 1;
        health.last_failure = Some(Instant::now());
    }
    
    /// Get health status for all providers
    pub async fn get_health_status(&self) -> Vec<CircuitBreakerHealth> {
        let circuit_breakers = self.circuit_breakers.read().await;
        circuit_breakers
            .values()
            .map(|cb| cb.health())
            .collect()
    }
    
    /// Get metrics for all providers
    pub async fn get_metrics(&self) -> HashMap<String, ProviderHealth> {
        self.health_metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_providers() -> Vec<Provider> {
        vec![
            Provider {
                id: "provider1".to_string(),
                name: "Provider 1".to_string(),
                endpoint: "https://api1.example.com".to_string(),
                priority: 1,
                cost_per_1k_tokens: 0.002,
                max_tokens: 4096,
                enabled: true,
            },
            Provider {
                id: "provider2".to_string(),
                name: "Provider 2".to_string(),
                endpoint: "https://api2.example.com".to_string(),
                priority: 2,
                cost_per_1k_tokens: 0.001,
                max_tokens: 8192,
                enabled: true,
            },
        ]
    }
    
    #[tokio::test]
    async fn test_routing_engine_creation() {
        let providers = create_test_providers();
        let engine = RoutingEngine::with_round_robin(providers.clone());
        
        let health = engine.get_health_status().await;
        assert_eq!(health.len(), providers.len());
    }
    
    #[tokio::test]
    async fn test_successful_routing() {
        let providers = create_test_providers();
        let engine = RoutingEngine::with_round_robin(providers);
        
        let result = engine
            .route(|_provider| {
                Box::pin(async {
                    Ok::<_, std::io::Error>("success")
                })
            })
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
}
