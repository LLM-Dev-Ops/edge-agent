// Provider abstraction layer for LLM providers
// This module provides a unified interface for interacting with different LLM providers

pub mod types;
pub mod pricing;
pub mod openai;
pub mod anthropic;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

pub use types::*;
pub use pricing::{ModelPricing, CostCalculation};

/// Errors that can occur when interacting with providers
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid API key for provider {provider}")]
    InvalidApiKey { provider: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { message: String },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Provider error: {message}")]
    ProviderError { message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Timeout: request exceeded {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for provider operations
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Main trait for LLM provider implementations
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider name (e.g., "openai", "anthropic")
    fn name(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// Send a completion request
    async fn complete(&self, request: LLMRequest) -> ProviderResult<LLMResponse>;

    /// Check provider health
    async fn health_check(&self) -> ProviderResult<HealthStatus>;

    /// List available models
    fn list_models(&self) -> Vec<String>;

    /// Validate a model name
    fn validate_model(&self, model: &str) -> bool {
        self.list_models().contains(&model.to_string())
    }

    /// Get model pricing information
    fn get_pricing(&self, model: &str) -> Option<&'static ModelPricing> {
        ModelPricing::get(model)
    }

    /// Calculate cost for a request/response
    fn calculate_cost(&self, model: &str, usage: &Usage) -> Option<CostCalculation> {
        self.get_pricing(model).map(|pricing| {
            pricing.calculate_cost(usage.prompt_tokens, usage.completion_tokens)
        })
    }
}

/// Provider registry for managing multiple providers
pub struct ProviderRegistry {
    providers: std::collections::HashMap<String, Arc<dyn LLMProvider>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Arc<dyn LLMProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(name).cloned()
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Get provider for a specific model
    pub fn get_for_model(&self, model: &str) -> Option<Arc<dyn LLMProvider>> {
        // Try to infer provider from model name
        if model.starts_with("gpt-") || model.starts_with("o1-") {
            self.get("openai")
        } else if model.starts_with("claude-") {
            self.get("anthropic")
        } else {
            // Fallback: check all providers
            self.providers.values().find(|p| p.validate_model(model)).cloned()
        }
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> std::collections::HashMap<String, HealthStatus> {
        let mut results = std::collections::HashMap::new();

        for (name, provider) in &self.providers {
            match provider.health_check().await {
                Ok(status) => {
                    results.insert(name.clone(), status);
                }
                Err(e) => {
                    results.insert(name.clone(), HealthStatus {
                        healthy: false,
                        last_check: chrono::Utc::now().timestamp(),
                        response_time_ms: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        results
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a configured provider registry
pub struct ProviderRegistryBuilder {
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    timeout_ms: u64,
    max_retries: u32,
}

impl ProviderRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            openai_api_key: None,
            anthropic_api_key: None,
            timeout_ms: 30000, // 30 seconds default
            max_retries: 3,
        }
    }

    /// Set OpenAI API key
    pub fn with_openai_key(mut self, api_key: impl Into<String>) -> Self {
        self.openai_api_key = Some(api_key.into());
        self
    }

    /// Set Anthropic API key
    pub fn with_anthropic_key(mut self, api_key: impl Into<String>) -> Self {
        self.anthropic_api_key = Some(api_key.into());
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Build the registry
    pub fn build(self) -> ProviderResult<ProviderRegistry> {
        let mut registry = ProviderRegistry::new();

        // Register OpenAI if API key provided
        if let Some(api_key) = self.openai_api_key {
            let provider = openai::OpenAIProvider::new(api_key, self.timeout_ms, self.max_retries)?;
            registry.register(Arc::new(provider));
        }

        // Register Anthropic if API key provided
        if let Some(api_key) = self.anthropic_api_key {
            let provider = anthropic::AnthropicProvider::new(api_key, self.timeout_ms, self.max_retries)?;
            registry.register(Arc::new(provider));
        }

        Ok(registry)
    }
}

impl Default for ProviderRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.list_providers().len(), 0);
    }

    #[test]
    fn test_model_inference() {
        let mut registry = ProviderRegistry::new();

        // This would normally use actual providers
        // For now, just test the structure
        assert_eq!(registry.list_providers().len(), 0);
    }
}
