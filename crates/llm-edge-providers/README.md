# llm-edge-providers

[![Crates.io](https://img.shields.io/crates/v/llm-edge-providers.svg)](https://crates.io/crates/llm-edge-providers)
[![Documentation](https://docs.rs/llm-edge-providers/badge.svg)](https://docs.rs/llm-edge-providers)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A unified Rust library for interfacing with multiple Large Language Model (LLM) providers through a consistent API. This crate provides production-ready adapters with built-in retry logic, error handling, and observability.

## Features

- **Unified API**: Single interface for all LLM providers
- **Multiple Providers**: Support for 5 major LLM providers:
  - **OpenAI** (GPT-4, GPT-3.5, o1)
  - **Anthropic** (Claude 3.5 Sonnet, Claude 3 Opus/Haiku)
  - **Google** (Gemini Pro, Gemini Ultra)
  - **AWS Bedrock** (Multi-model support)
  - **Azure OpenAI** (Enterprise-ready)
- **Async/Await**: Built on Tokio for efficient async operations
- **Retry Logic**: Automatic retry with exponential backoff
- **Security**: Secure API key handling with `secrecy`
- **Observability**: OpenTelemetry integration for tracing
- **Type Safety**: Strongly typed requests and responses
- **Cost Tracking**: Built-in pricing information and usage tracking

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-edge-providers = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Usage

### Basic Example

```rust
use llm_edge_providers::{
    LLMProvider, UnifiedRequest, Message,
    openai::OpenAIAdapter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create provider adapter
    let provider = OpenAIAdapter::new(
        std::env::var("OPENAI_API_KEY")?
    );

    // Create request
    let request = UnifiedRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "What is Rust?".to_string(),
            }
        ],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        stream: false,
        metadata: Default::default(),
    };

    // Send request
    let response = provider.send(request).await?;

    println!("Response: {}", response.choices[0].message.content);
    println!("Tokens used: {}", response.usage.total_tokens);

    Ok(())
}
```

### OpenAI Provider

```rust
use llm_edge_providers::openai::OpenAIAdapter;

let provider = OpenAIAdapter::new(api_key);

// Get pricing information
if let Some(pricing) = provider.get_pricing("gpt-4") {
    println!("Input: ${}/1K tokens", pricing.input_cost_per_1k);
    println!("Output: ${}/1K tokens", pricing.output_cost_per_1k);
}

// Check provider health
let health = provider.health().await;
```

### Anthropic Provider

```rust
use llm_edge_providers::anthropic::AnthropicAdapter;

let provider = AnthropicAdapter::new(api_key);

let request = UnifiedRequest {
    model: "claude-3-5-sonnet-20240229".to_string(),
    messages: vec![
        Message {
            role: "user".to_string(),
            content: "Explain async/await in Rust".to_string(),
        }
    ],
    temperature: Some(0.7),
    max_tokens: Some(2048),
    stream: false,
    metadata: Default::default(),
};

let response = provider.send(request).await?;
```

### Multi-Provider Usage

```rust
use llm_edge_providers::{LLMProvider, UnifiedRequest};

async fn send_to_provider(
    provider: &dyn LLMProvider,
    request: UnifiedRequest
) -> Result<String, Box<dyn std::error::Error>> {
    let response = provider.send(request).await?;
    Ok(response.choices[0].message.content.clone())
}

// Use with any provider
let openai = OpenAIAdapter::new(openai_key);
let anthropic = AnthropicAdapter::new(anthropic_key);

let result1 = send_to_provider(&openai, request.clone()).await?;
let result2 = send_to_provider(&anthropic, request.clone()).await?;
```

### Error Handling

```rust
use llm_edge_providers::ProviderError;

match provider.send(request).await {
    Ok(response) => {
        println!("Success: {}", response.choices[0].message.content);
    }
    Err(ProviderError::RateLimitExceeded) => {
        eprintln!("Rate limit hit, please retry later");
    }
    Err(ProviderError::ApiError { status, message }) => {
        eprintln!("API error {}: {}", status, message);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Provider Configuration

### OpenAI

Set your API key:
```bash
export OPENAI_API_KEY="sk-..."
```

Supported models: `gpt-4`, `gpt-4-turbo`, `gpt-3.5-turbo`, `o1-preview`, `o1-mini`

### Anthropic

Set your API key:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Supported models: `claude-3-5-sonnet-20240229`, `claude-3-opus-20240229`, `claude-3-haiku-20240307`

### Google (Gemini)

Set your API key:
```bash
export GOOGLE_API_KEY="..."
```

Supported models: `gemini-pro`, `gemini-ultra`

### AWS Bedrock

Configure AWS credentials:
```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
```

### Azure OpenAI

Configure Azure credentials:
```bash
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com/"
```

## API Documentation

### Core Types

- **`UnifiedRequest`**: Standard request format across all providers
  - `model`: Model identifier
  - `messages`: Conversation messages
  - `temperature`: Sampling temperature (0.0-2.0)
  - `max_tokens`: Maximum tokens to generate
  - `stream`: Enable streaming responses
  - `metadata`: Custom metadata

- **`UnifiedResponse`**: Standard response format
  - `id`: Unique response identifier
  - `model`: Model used
  - `choices`: Response choices
  - `usage`: Token usage statistics
  - `metadata`: Response metadata (provider, latency, cost)

- **`LLMProvider`**: Core trait implemented by all adapters
  - `name()`: Provider name
  - `send()`: Send request to provider
  - `get_pricing()`: Get model pricing
  - `health()`: Check provider health

### Error Types

All errors are represented by `ProviderError`:
- `Http`: HTTP client errors
- `Serialization`: JSON serialization errors
- `ApiError`: Provider API errors with status code
- `Timeout`: Request timeout
- `RateLimitExceeded`: Rate limit hit
- `Configuration`: Invalid configuration
- `Internal`: Internal errors

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please see the [repository](https://github.com/globalbusinessadvisors/llm-edge-agent) for contribution guidelines.

## Related Crates

- [`llm-edge-core`](../llm-edge-core) - Core abstractions and types
- [`llm-edge-router`](../llm-edge-router) - Request routing and load balancing
- [`llm-edge-cache`](../llm-edge-cache) - Response caching
- [`llm-edge-backend`](../llm-edge-backend) - Complete backend service
