use crate::{ProviderResult, UnifiedRequest, UnifiedResponse};
use async_trait::async_trait;

/// Health status of a provider
#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Pricing information for a model
#[derive(Debug, Clone)]
pub struct PricingInfo {
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
}

/// Trait that all LLM provider adapters must implement
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Returns the provider name
    fn name(&self) -> &str;

    /// Sends a request to the provider
    async fn send(&self, request: UnifiedRequest) -> ProviderResult<UnifiedResponse>;

    /// Gets pricing information for a model
    fn get_pricing(&self, model: &str) -> Option<PricingInfo>;

    /// Checks provider health
    async fn health(&self) -> HealthStatus;
}
