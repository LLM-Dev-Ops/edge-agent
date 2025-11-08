//! Routing strategies for LLM provider selection
//!
//! Implements various strategies for selecting which LLM provider to use:
//! - Round Robin: Distributes requests evenly across providers
//! - Failover Chain: Tries providers in priority order until one succeeds
//! - Least Latency: Routes to the provider with lowest average latency
//! - Cost Optimized: Routes to the cheapest provider that meets requirements

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Provider information for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Unique provider identifier (e.g., "openai", "anthropic")
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Provider endpoint URL
    pub endpoint: String,
    
    /// Priority for failover (lower = higher priority)
    pub priority: u32,
    
    /// Cost per 1K tokens (in cents)
    pub cost_per_1k_tokens: f64,
    
    /// Maximum tokens supported
    pub max_tokens: u32,
    
    /// Whether provider is currently enabled
    pub enabled: bool,
}

/// Provider with runtime health metrics
#[derive(Debug, Clone)]
pub struct ProviderWithHealth {
    pub provider: Provider,
    pub is_healthy: bool,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
}

/// Trait for routing strategies
#[async_trait]
pub trait RoutingStrategy: Send + Sync {
    /// Select a provider based on the strategy
    async fn select_provider(
        &self,
        providers: &[ProviderWithHealth],
    ) -> Option<Provider>;
    
    /// Record the result of a request for learning
    async fn record_result(
        &self,
        provider_id: &str,
        latency: Duration,
        success: bool,
    );
    
    /// Get strategy name for logging
    fn name(&self) -> &str;
}

/// Round-robin routing strategy
/// 
/// Distributes requests evenly across all healthy providers
pub struct RoundRobinStrategy {
    counter: AtomicUsize,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        info!("Initialized Round Robin routing strategy");
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingStrategy for RoundRobinStrategy {
    async fn select_provider(
        &self,
        providers: &[ProviderWithHealth],
    ) -> Option<Provider> {
        // Filter to only healthy providers
        let healthy: Vec<_> = providers
            .iter()
            .filter(|p| p.provider.enabled && p.is_healthy)
            .collect();
        
        if healthy.is_empty() {
            warn!("No healthy providers available for round-robin routing");
            return None;
        }
        
        // Get next provider in round-robin fashion
        let index = self.counter.fetch_add(1, Ordering::Relaxed) % healthy.len();
        let selected = &healthy[index].provider;
        
        debug!(
            provider = %selected.id,
            index = index,
            total_healthy = healthy.len(),
            "Selected provider via round-robin"
        );
        
        Some(selected.clone())
    }
    
    async fn record_result(
        &self,
        _provider_id: &str,
        _latency: Duration,
        _success: bool,
    ) {
        // Round-robin doesn't use result history
    }
    
    fn name(&self) -> &str {
        "round-robin"
    }
}

/// Failover chain routing strategy
///
/// Tries providers in priority order until one succeeds
pub struct FailoverChainStrategy {
    max_retries: usize,
}

impl FailoverChainStrategy {
    pub fn new(max_retries: usize) -> Self {
        info!(
            max_retries = max_retries,
            "Initialized Failover Chain routing strategy"
        );
        Self { max_retries }
    }
}

impl Default for FailoverChainStrategy {
    fn default() -> Self {
        Self::new(3)
    }
}

#[async_trait]
impl RoutingStrategy for FailoverChainStrategy {
    async fn select_provider(
        &self,
        providers: &[ProviderWithHealth],
    ) -> Option<Provider> {
        // Sort by priority (lower number = higher priority)
        let mut sorted: Vec<_> = providers
            .iter()
            .filter(|p| p.provider.enabled && p.is_healthy)
            .collect();
        
        sorted.sort_by_key(|p| p.provider.priority);
        
        if sorted.is_empty() {
            warn!("No healthy providers available for failover chain");
            return None;
        }
        
        let selected = &sorted[0].provider;
        debug!(
            provider = %selected.id,
            priority = selected.priority,
            "Selected highest priority provider"
        );
        
        Some(selected.clone())
    }
    
    async fn record_result(
        &self,
        _provider_id: &str,
        _latency: Duration,
        _success: bool,
    ) {
        // Failover chain doesn't use result history
    }
    
    fn name(&self) -> &str {
        "failover-chain"
    }
}

/// Least latency routing strategy
///
/// Routes to the provider with the lowest average latency
pub struct LeastLatencyStrategy {
    latency_tracker: Arc<LatencyTracker>,
}

impl LeastLatencyStrategy {
    pub fn new() -> Self {
        info!("Initialized Least Latency routing strategy");
        Self {
            latency_tracker: Arc::new(LatencyTracker::new()),
        }
    }
}

impl Default for LeastLatencyStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingStrategy for LeastLatencyStrategy {
    async fn select_provider(
        &self,
        providers: &[ProviderWithHealth],
    ) -> Option<Provider> {
        let healthy: Vec<_> = providers
            .iter()
            .filter(|p| p.provider.enabled && p.is_healthy)
            .collect();
        
        if healthy.is_empty() {
            warn!("No healthy providers available for least latency routing");
            return None;
        }
        
        // Select provider with lowest average latency
        let selected = healthy
            .iter()
            .min_by(|a, b| {
                a.avg_latency_ms
                    .partial_cmp(&b.avg_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| &p.provider);
        
        if let Some(provider) = selected {
            debug!(
                provider = %provider.id,
                avg_latency_ms = ?providers.iter()
                    .find(|p| p.provider.id == provider.id)
                    .map(|p| p.avg_latency_ms),
                "Selected lowest latency provider"
            );
            Some(provider.clone())
        } else {
            None
        }
    }
    
    async fn record_result(
        &self,
        provider_id: &str,
        latency: Duration,
        success: bool,
    ) {
        if success {
            self.latency_tracker.record(provider_id, latency);
        }
    }
    
    fn name(&self) -> &str {
        "least-latency"
    }
}

/// Cost-optimized routing strategy
///
/// Routes to the cheapest provider that meets requirements
pub struct CostOptimizedStrategy;

impl CostOptimizedStrategy {
    pub fn new() -> Self {
        info!("Initialized Cost Optimized routing strategy");
        Self
    }
}

impl Default for CostOptimizedStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingStrategy for CostOptimizedStrategy {
    async fn select_provider(
        &self,
        providers: &[ProviderWithHealth],
    ) -> Option<Provider> {
        let healthy: Vec<_> = providers
            .iter()
            .filter(|p| p.provider.enabled && p.is_healthy)
            .collect();
        
        if healthy.is_empty() {
            warn!("No healthy providers available for cost-optimized routing");
            return None;
        }
        
        // Select cheapest provider
        let selected = healthy
            .iter()
            .min_by(|a, b| {
                a.provider
                    .cost_per_1k_tokens
                    .partial_cmp(&b.provider.cost_per_1k_tokens)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| &p.provider);
        
        if let Some(provider) = selected {
            debug!(
                provider = %provider.id,
                cost_per_1k = provider.cost_per_1k_tokens,
                "Selected lowest cost provider"
            );
            Some(provider.clone())
        } else {
            None
        }
    }
    
    async fn record_result(
        &self,
        _provider_id: &str,
        _latency: Duration,
        _success: bool,
    ) {
        // Cost-optimized doesn't use result history
    }
    
    fn name(&self) -> &str {
        "cost-optimized"
    }
}

/// Tracks latency metrics for providers
struct LatencyTracker {
    // In production, use a more sophisticated data structure
    // For now, simple atomic counters (milliseconds)
}

impl LatencyTracker {
    fn new() -> Self {
        Self {}
    }
    
    fn record(&self, _provider_id: &str, _latency: Duration) {
        // In production, maintain rolling average per provider
        // Using exponential moving average or similar
    }
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Initial backoff duration
    pub initial_backoff: Duration,
    
    /// Maximum backoff duration
    pub max_backoff: Duration,
    
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for a given attempt
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let backoff_ms = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        
        let backoff = Duration::from_millis(backoff_ms as u64);
        std::cmp::min(backoff, self.max_backoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_providers() -> Vec<ProviderWithHealth> {
        vec![
            ProviderWithHealth {
                provider: Provider {
                    id: "provider1".to_string(),
                    name: "Provider 1".to_string(),
                    endpoint: "https://api1.example.com".to_string(),
                    priority: 1,
                    cost_per_1k_tokens: 0.002,
                    max_tokens: 4096,
                    enabled: true,
                },
                is_healthy: true,
                avg_latency_ms: 100.0,
                success_rate: 0.99,
            },
            ProviderWithHealth {
                provider: Provider {
                    id: "provider2".to_string(),
                    name: "Provider 2".to_string(),
                    endpoint: "https://api2.example.com".to_string(),
                    priority: 2,
                    cost_per_1k_tokens: 0.001,
                    max_tokens: 8192,
                    enabled: true,
                },
                is_healthy: true,
                avg_latency_ms: 150.0,
                success_rate: 0.98,
            },
        ]
    }
    
    #[tokio::test]
    async fn test_round_robin_strategy() {
        let strategy = RoundRobinStrategy::new();
        let providers = create_test_providers();
        
        let first = strategy.select_provider(&providers).await.unwrap();
        let second = strategy.select_provider(&providers).await.unwrap();
        
        // Should alternate
        assert_ne!(first.id, second.id);
    }
    
    #[tokio::test]
    async fn test_failover_chain_strategy() {
        let strategy = FailoverChainStrategy::new(3);
        let providers = create_test_providers();
        
        let selected = strategy.select_provider(&providers).await.unwrap();
        
        // Should select provider with priority 1
        assert_eq!(selected.priority, 1);
    }
    
    #[tokio::test]
    async fn test_cost_optimized_strategy() {
        let strategy = CostOptimizedStrategy::new();
        let providers = create_test_providers();
        
        let selected = strategy.select_provider(&providers).await.unwrap();
        
        // Should select cheaper provider (provider2)
        assert_eq!(selected.id, "provider2");
    }
    
    #[test]
    fn test_retry_backoff() {
        let config = RetryConfig::default();
        
        let backoff1 = config.backoff_duration(0);
        let backoff2 = config.backoff_duration(1);
        let backoff3 = config.backoff_duration(2);
        
        // Should increase exponentially
        assert!(backoff2 > backoff1);
        assert!(backoff3 > backoff2);
    }
}
