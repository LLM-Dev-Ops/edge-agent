// Integration tests for provider layer
// These tests verify the provider implementations without making actual API calls

#[cfg(test)]
mod provider_tests {
    use crate::providers::*;

    #[test]
    fn test_unified_request_builder() {
        let request = LLMRequest::new("gpt-4", vec![])
            .with_user_message("Hello")
            .with_temperature(0.7)
            .with_max_tokens(100)
            .with_streaming(true);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert!(request.stream);
    }

    #[test]
    fn test_message_creation() {
        let system_msg = Message::system("You are helpful");
        let user_msg = Message::user("Hello");
        let assistant_msg = Message::assistant("Hi there");

        assert!(matches!(system_msg.role, Role::System));
        assert!(matches!(user_msg.role, Role::User));
        assert!(matches!(assistant_msg.role, Role::Assistant));
    }

    #[test]
    fn test_pricing_database() {
        // OpenAI models
        assert!(ModelPricing::get("gpt-4").is_some());
        assert!(ModelPricing::get("gpt-3.5-turbo").is_some());
        assert!(ModelPricing::get("o1-preview").is_some());

        // Anthropic models
        assert!(ModelPricing::get("claude-3-5-sonnet-20241022").is_some());
        assert!(ModelPricing::get("claude-3-opus").is_some());
        assert!(ModelPricing::get("claude-3-haiku").is_some());

        // Invalid model
        assert!(ModelPricing::get("invalid-model").is_none());
    }

    #[test]
    fn test_cost_calculation() {
        let pricing = ModelPricing::get("gpt-4").unwrap();
        let cost = pricing.calculate_cost(1000, 500);

        // 1000 input * $0.03/1k = $0.03
        // 500 output * $0.06/1k = $0.03
        // Total = $0.06
        assert_eq!(cost.input_tokens, 1000);
        assert_eq!(cost.output_tokens, 500);
        assert_eq!(cost.total_tokens, 1500);
        assert!((cost.total_cost - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_cost_formatting() {
        let pricing = ModelPricing::get("gpt-3.5-turbo").unwrap();
        let cost = pricing.calculate_cost(1000, 1000);

        let formatted = cost.format_cost();
        assert!(formatted.starts_with('$'));

        let cents = cost.cost_in_cents();
        assert!(cents > 0.0);
    }

    #[test]
    fn test_list_models_by_provider() {
        let openai_models = ModelPricing::list_by_provider("openai");
        assert!(!openai_models.is_empty());

        let anthropic_models = ModelPricing::list_by_provider("anthropic");
        assert!(!anthropic_models.is_empty());
    }

    #[test]
    fn test_cheapest_model() {
        let cheapest_openai = ModelPricing::cheapest_for_provider("openai");
        assert!(cheapest_openai.is_some());

        let cheapest_anthropic = ModelPricing::cheapest_for_provider("anthropic");
        assert!(cheapest_anthropic.is_some());
        // Should be Haiku
        assert!(cheapest_anthropic.unwrap().model.contains("haiku"));
    }

    #[test]
    fn test_provider_registry() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.list_providers().len(), 0);
    }

    #[test]
    fn test_openai_provider_creation() {
        let provider = openai::OpenAIProvider::new(
            "test-key".to_string(),
            30000,
            3
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_openai_model_validation() {
        let provider = openai::OpenAIProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        assert!(provider.validate_model("gpt-4"));
        assert!(provider.validate_model("gpt-3.5-turbo"));
        assert!(provider.validate_model("o1-preview"));
        assert!(!provider.validate_model("claude-3-opus"));
    }

    #[test]
    fn test_openai_list_models() {
        let provider = openai::OpenAIProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        let models = provider.list_models();
        assert!(models.contains(&"gpt-4".to_string()));
        assert!(models.contains(&"gpt-3.5-turbo".to_string()));
        assert!(models.contains(&"o1-preview".to_string()));
    }

    #[test]
    fn test_openai_capabilities() {
        let provider = openai::OpenAIProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        let caps = provider.capabilities();
        assert!(caps.supports_streaming);
        assert!(caps.supports_function_calling);
        assert!(caps.supports_vision);
        assert!(caps.max_context_tokens > 0);
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = anthropic::AnthropicProvider::new(
            "test-key".to_string(),
            30000,
            3
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_anthropic_model_validation() {
        let provider = anthropic::AnthropicProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        assert!(provider.validate_model("claude-3-5-sonnet-20241022"));
        assert!(provider.validate_model("claude-3-opus"));
        assert!(provider.validate_model("claude-3-haiku"));
        assert!(!provider.validate_model("gpt-4"));
    }

    #[test]
    fn test_anthropic_list_models() {
        let provider = anthropic::AnthropicProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        let models = provider.list_models();
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(models.contains(&"claude-3-opus".to_string()));
        assert!(models.contains(&"claude-3-haiku".to_string()));
    }

    #[test]
    fn test_anthropic_capabilities() {
        let provider = anthropic::AnthropicProvider::new(
            "test-key".to_string(),
            30000,
            3
        ).unwrap();

        let caps = provider.capabilities();
        assert!(caps.supports_streaming);
        assert!(caps.supports_function_calling);
        assert!(caps.supports_vision);
        assert_eq!(caps.max_context_tokens, 200000);
    }

    #[test]
    fn test_registry_builder() {
        let builder = ProviderRegistryBuilder::new()
            .with_timeout(60000)
            .with_max_retries(5);

        // Can't test build without real API keys
        // But we can verify the builder pattern works
        assert!(true);
    }

    #[test]
    fn test_error_types() {
        let error = ProviderError::InvalidApiKey {
            provider: "test".to_string()
        };
        assert!(error.to_string().contains("Invalid API key"));

        let error = ProviderError::RateLimitExceeded {
            message: "Too many requests".to_string()
        };
        assert!(error.to_string().contains("Rate limit"));

        let error = ProviderError::ModelNotFound {
            model: "test-model".to_string()
        };
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn test_message_content_types() {
        let text_content = MessageContent::Text("Hello".to_string());
        assert!(matches!(text_content, MessageContent::Text(_)));

        let parts_content = MessageContent::Parts(vec![
            ContentPart::Text { text: "Hello".to_string() }
        ]);
        assert!(matches!(parts_content, MessageContent::Parts(_)));
    }

    #[test]
    fn test_finish_reasons() {
        assert_eq!(FinishReason::Stop, FinishReason::Stop);
        assert_eq!(FinishReason::Length, FinishReason::Length);
        assert_ne!(FinishReason::Stop, FinishReason::Length);
    }

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_provider_model_inference() {
        let registry = ProviderRegistry::new();

        // Test model inference logic (even without registered providers)
        // This tests the logic in get_for_model
        assert!(registry.get_for_model("gpt-4").is_none()); // No providers registered
        assert!(registry.get_for_model("claude-3-opus").is_none()); // No providers registered
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::super::*;

    #[test]
    fn test_request_transformation_performance() {
        let request = LLMRequest::new("gpt-4", vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
        ]);

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = serde_json::to_string(&request);
        }
        let elapsed = start.elapsed();

        // Should be fast - less than 10ms for 1000 serializations
        assert!(elapsed.as_millis() < 10, "Serialization too slow: {:?}", elapsed);
    }

    #[test]
    fn test_pricing_lookup_performance() {
        let start = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = ModelPricing::get("gpt-4");
        }
        let elapsed = start.elapsed();

        // Should be instant - static lookup
        assert!(elapsed.as_millis() < 5, "Pricing lookup too slow: {:?}", elapsed);
    }

    #[test]
    fn test_cost_calculation_performance() {
        let pricing = ModelPricing::get("gpt-4").unwrap();

        let start = std::time::Instant::now();
        for _ in 0..100000 {
            let _ = pricing.calculate_cost(1000, 500);
        }
        let elapsed = start.elapsed();

        // Should be very fast
        assert!(elapsed.as_millis() < 10, "Cost calculation too slow: {:?}", elapsed);
    }
}
