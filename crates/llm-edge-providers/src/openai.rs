//! OpenAI provider adapter

use crate::{
    adapter::{HealthStatus, LLMProvider, PricingInfo},
    ProviderResult, UnifiedRequest, UnifiedResponse,
};
use async_trait::async_trait;
use secrecy::Secret;

pub struct OpenAIAdapter {
    #[allow(dead_code)]
    client: reqwest::Client,
    #[allow(dead_code)]
    api_key: Secret<String>,
    #[allow(dead_code)]
    base_url: String,
}

impl OpenAIAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Secret::new(api_key),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIAdapter {
    fn name(&self) -> &str {
        "openai"
    }

    async fn send(&self, _request: UnifiedRequest) -> ProviderResult<UnifiedResponse> {
        // TODO: Implement OpenAI API call
        // - Transform UnifiedRequest to OpenAI format
        // - Make HTTP request
        // - Transform response to UnifiedResponse
        todo!("OpenAI adapter implementation")
    }

    fn get_pricing(&self, model: &str) -> Option<PricingInfo> {
        // Pricing as of 2024 (update regularly)
        match model {
            "gpt-4" => Some(PricingInfo {
                input_cost_per_1k: 0.03,
                output_cost_per_1k: 0.06,
            }),
            "gpt-3.5-turbo" => Some(PricingInfo {
                input_cost_per_1k: 0.0005,
                output_cost_per_1k: 0.0015,
            }),
            _ => None,
        }
    }

    async fn health(&self) -> HealthStatus {
        // TODO: Implement health check
        HealthStatus::Healthy
    }
}
