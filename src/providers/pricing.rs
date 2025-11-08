// Pricing database for LLM providers
// Used for cost tracking and optimization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Pricing information for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model identifier
    pub model: String,

    /// Provider name
    pub provider: String,

    /// Cost per 1K input tokens (USD)
    pub input_cost_per_1k: f64,

    /// Cost per 1K output tokens (USD)
    pub output_cost_per_1k: f64,

    /// Whether this model is available
    pub available: bool,

    /// Last updated timestamp
    pub updated_at: &'static str,
}

/// Calculate cost for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCalculation {
    /// Input tokens used
    pub input_tokens: u32,

    /// Output tokens generated
    pub output_tokens: u32,

    /// Total tokens
    pub total_tokens: u32,

    /// Input cost in USD
    pub input_cost: f64,

    /// Output cost in USD
    pub output_cost: f64,

    /// Total cost in USD
    pub total_cost: f64,

    /// Model used
    pub model: String,

    /// Provider name
    pub provider: String,
}

/// Pricing database - static data for now, can be externalized later
pub static PRICING_DB: Lazy<HashMap<&'static str, ModelPricing>> = Lazy::new(|| {
    let mut db = HashMap::new();

    // OpenAI GPT-4 models
    db.insert("gpt-4", ModelPricing {
        model: "gpt-4".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.03,
        output_cost_per_1k: 0.06,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("gpt-4-turbo", ModelPricing {
        model: "gpt-4-turbo".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.01,
        output_cost_per_1k: 0.03,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("gpt-4-turbo-preview", ModelPricing {
        model: "gpt-4-turbo-preview".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.01,
        output_cost_per_1k: 0.03,
        available: true,
        updated_at: "2025-01-01",
    });

    // OpenAI GPT-3.5 models
    db.insert("gpt-3.5-turbo", ModelPricing {
        model: "gpt-3.5-turbo".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.0005,
        output_cost_per_1k: 0.0015,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("gpt-3.5-turbo-16k", ModelPricing {
        model: "gpt-3.5-turbo-16k".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.003,
        output_cost_per_1k: 0.004,
        available: true,
        updated_at: "2025-01-01",
    });

    // OpenAI O1 models (reasoning models)
    db.insert("o1-preview", ModelPricing {
        model: "o1-preview".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.015,
        output_cost_per_1k: 0.06,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("o1-mini", ModelPricing {
        model: "o1-mini".to_string(),
        provider: "openai".to_string(),
        input_cost_per_1k: 0.003,
        output_cost_per_1k: 0.012,
        available: true,
        updated_at: "2025-01-01",
    });

    // Anthropic Claude models
    db.insert("claude-3-5-sonnet-20241022", ModelPricing {
        model: "claude-3-5-sonnet-20241022".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.003,
        output_cost_per_1k: 0.015,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("claude-3-opus-20240229", ModelPricing {
        model: "claude-3-opus-20240229".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.015,
        output_cost_per_1k: 0.075,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("claude-3-sonnet-20240229", ModelPricing {
        model: "claude-3-sonnet-20240229".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.003,
        output_cost_per_1k: 0.015,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("claude-3-haiku-20240307", ModelPricing {
        model: "claude-3-haiku-20240307".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.00025,
        output_cost_per_1k: 0.00125,
        available: true,
        updated_at: "2025-01-01",
    });

    // Aliases for convenience
    db.insert("claude-3.5-sonnet", ModelPricing {
        model: "claude-3-5-sonnet-20241022".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.003,
        output_cost_per_1k: 0.015,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("claude-3-opus", ModelPricing {
        model: "claude-3-opus-20240229".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.015,
        output_cost_per_1k: 0.075,
        available: true,
        updated_at: "2025-01-01",
    });

    db.insert("claude-3-haiku", ModelPricing {
        model: "claude-3-haiku-20240307".to_string(),
        provider: "anthropic".to_string(),
        input_cost_per_1k: 0.00025,
        output_cost_per_1k: 0.00125,
        available: true,
        updated_at: "2025-01-01",
    });

    db
});

impl ModelPricing {
    /// Get pricing for a model
    pub fn get(model: &str) -> Option<&'static ModelPricing> {
        PRICING_DB.get(model)
    }

    /// Calculate cost for a request
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> CostCalculation {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        let total_cost = input_cost + output_cost;

        CostCalculation {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            input_cost,
            output_cost,
            total_cost,
            model: self.model.clone(),
            provider: self.provider.clone(),
        }
    }

    /// List all available models
    pub fn list_models() -> Vec<&'static ModelPricing> {
        PRICING_DB.values().collect()
    }

    /// List models by provider
    pub fn list_by_provider(provider: &str) -> Vec<&'static ModelPricing> {
        PRICING_DB
            .values()
            .filter(|p| p.provider == provider)
            .collect()
    }

    /// Get cheapest model for a provider
    pub fn cheapest_for_provider(provider: &str) -> Option<&'static ModelPricing> {
        Self::list_by_provider(provider)
            .into_iter()
            .min_by(|a, b| {
                let a_avg = (a.input_cost_per_1k + a.output_cost_per_1k) / 2.0;
                let b_avg = (b.input_cost_per_1k + b.output_cost_per_1k) / 2.0;
                a_avg.partial_cmp(&b_avg).unwrap()
            })
    }
}

impl CostCalculation {
    /// Format cost as a human-readable string
    pub fn format_cost(&self) -> String {
        format!("${:.6}", self.total_cost)
    }

    /// Get cost in cents
    pub fn cost_in_cents(&self) -> f64 {
        self.total_cost * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_lookup() {
        let pricing = ModelPricing::get("gpt-4");
        assert!(pricing.is_some());
        assert_eq!(pricing.unwrap().provider, "openai");
    }

    #[test]
    fn test_cost_calculation() {
        let pricing = ModelPricing::get("gpt-4").unwrap();
        let cost = pricing.calculate_cost(1000, 500);

        // 1000 input tokens * $0.03/1K = $0.03
        // 500 output tokens * $0.06/1K = $0.03
        // Total = $0.06
        assert_eq!(cost.input_tokens, 1000);
        assert_eq!(cost.output_tokens, 500);
        assert!((cost.total_cost - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_list_by_provider() {
        let openai_models = ModelPricing::list_by_provider("openai");
        assert!(!openai_models.is_empty());

        let anthropic_models = ModelPricing::list_by_provider("anthropic");
        assert!(!anthropic_models.is_empty());
    }

    #[test]
    fn test_cheapest_model() {
        let cheapest = ModelPricing::cheapest_for_provider("anthropic");
        assert!(cheapest.is_some());
        // Claude Haiku should be the cheapest
        assert!(cheapest.unwrap().model.contains("haiku"));
    }

    #[test]
    fn test_claude_aliases() {
        let sonnet_full = ModelPricing::get("claude-3-5-sonnet-20241022");
        let sonnet_alias = ModelPricing::get("claude-3.5-sonnet");

        assert!(sonnet_full.is_some());
        assert!(sonnet_alias.is_some());
        assert_eq!(sonnet_full.unwrap().input_cost_per_1k, sonnet_alias.unwrap().input_cost_per_1k);
    }
}
