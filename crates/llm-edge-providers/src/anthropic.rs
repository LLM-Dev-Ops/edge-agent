//! Anthropic provider adapter

use crate::{
    adapter::{HealthStatus, LLMProvider, PricingInfo},
    ProviderResult, UnifiedRequest, UnifiedResponse,
};
use async_trait::async_trait;
use secrecy::Secret;

pub struct AnthropicAdapter {
    #[allow(dead_code)]
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: Secret<String>,
    #[allow(dead_code)]
    base_url: String,
}

impl AnthropicAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Secret::new(api_key),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicAdapter {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn send(&self, _request: UnifiedRequest) -> ProviderResult<UnifiedResponse> {
        // TODO: Implement Anthropic API call
        todo!("Anthropic adapter implementation")
    }

    fn get_pricing(&self, model: &str) -> Option<PricingInfo> {
        // Pricing as of 2024
        match model {
            "claude-3-5-sonnet-20240229" => Some(PricingInfo {
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.015,
            }),
            "claude-3-opus-20240229" => Some(PricingInfo {
                input_cost_per_1k: 0.015,
                output_cost_per_1k: 0.075,
            }),
            "claude-3-haiku-20240307" => Some(PricingInfo {
                input_cost_per_1k: 0.00025,
                output_cost_per_1k: 0.00125,
            }),
            _ => None,
        }
    }

    async fn health(&self) -> HealthStatus {
        // TODO: Implement health check
        HealthStatus::Healthy
    }
}
