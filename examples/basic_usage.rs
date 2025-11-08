// Example: Basic usage of the LLM Edge Agent provider layer
//
// Run with: cargo run --example basic_usage
//
// Set environment variables:
// export OPENAI_API_KEY=sk-...
// export ANTHROPIC_API_KEY=sk-ant-...

use llm_edge_agent::{
    ProviderRegistryBuilder, LLMRequest, Message, ModelPricing,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== LLM Edge Agent - Provider Layer Demo ===\n");

    // Get API keys from environment
    let openai_key = env::var("OPENAI_API_KEY").ok();
    let anthropic_key = env::var("ANTHROPIC_API_KEY").ok();

    if openai_key.is_none() && anthropic_key.is_none() {
        eprintln!("Error: No API keys found!");
        eprintln!("Please set OPENAI_API_KEY and/or ANTHROPIC_API_KEY");
        std::process::exit(1);
    }

    // Build provider registry with configured providers
    let mut builder = ProviderRegistryBuilder::new()
        .with_timeout(30000)  // 30 second timeout
        .with_max_retries(3); // 3 retries

    if let Some(key) = openai_key {
        builder = builder.with_openai_key(key);
        println!("✓ OpenAI provider registered");
    }

    if let Some(key) = anthropic_key {
        builder = builder.with_anthropic_key(key);
        println!("✓ Anthropic provider registered");
    }

    let registry = builder.build()?;
    println!("\n=== Provider Registry Built ===");
    println!("Available providers: {:?}\n", registry.list_providers());

    // Example 1: Health checks
    println!("=== Running Health Checks ===");
    let health_status = registry.health_check_all().await;
    for (provider, status) in health_status {
        if status.healthy {
            println!("✓ {}: Healthy ({}ms)",
                provider,
                status.response_time_ms.unwrap_or(0)
            );
        } else {
            println!("✗ {}: Unhealthy - {}",
                provider,
                status.error.unwrap_or_default()
            );
        }
    }

    // Example 2: Pricing information
    println!("\n=== Model Pricing Information ===");
    let models = vec!["gpt-4", "gpt-3.5-turbo", "claude-3-5-sonnet-20241022", "claude-3-haiku"];
    for model in models {
        if let Some(pricing) = ModelPricing::get(model) {
            println!("{}: ${:.4}/1k input, ${:.4}/1k output",
                model,
                pricing.input_cost_per_1k,
                pricing.output_cost_per_1k
            );
        }
    }

    // Example 3: Cost comparison
    println!("\n=== Cost Comparison (1000 input, 500 output tokens) ===");
    let comparison_models = vec!["gpt-4", "gpt-3.5-turbo", "claude-3-opus", "claude-3-haiku"];
    for model in comparison_models {
        if let Some(pricing) = ModelPricing::get(model) {
            let cost = pricing.calculate_cost(1000, 500);
            println!("{}: {} (${:.6})",
                model,
                cost.format_cost(),
                cost.total_cost
            );
        }
    }

    // Example 4: Make a simple request (if providers are available)
    if let Some(provider) = registry.get("openai").or_else(|| registry.get("anthropic")) {
        println!("\n=== Making Sample Request ===");
        println!("Provider: {}", provider.name());

        let model = if provider.name() == "openai" {
            "gpt-3.5-turbo"
        } else {
            "claude-3-haiku-20240307"
        };

        let request = LLMRequest::new(
            model,
            vec![
                Message::system("You are a helpful assistant that responds concisely."),
                Message::user("What is 2+2? Answer in one sentence."),
            ]
        )
        .with_max_tokens(50)
        .with_temperature(0.7);

        match provider.complete(request).await {
            Ok(response) => {
                println!("\nResponse:");
                if let Some(choice) = response.choices.first() {
                    if let llm_edge_agent::MessageContent::Text(text) = &choice.message.content {
                        println!("  {}", text);
                    }
                }
                println!("\nUsage:");
                println!("  Input tokens:  {}", response.usage.prompt_tokens);
                println!("  Output tokens: {}", response.usage.completion_tokens);
                println!("  Total tokens:  {}", response.usage.total_tokens);

                // Calculate cost
                if let Some(cost) = provider.calculate_cost(model, &response.usage) {
                    println!("\nCost: {} (${:.6})", cost.format_cost(), cost.total_cost);
                }
            }
            Err(e) => {
                eprintln!("\nError making request: {}", e);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}
