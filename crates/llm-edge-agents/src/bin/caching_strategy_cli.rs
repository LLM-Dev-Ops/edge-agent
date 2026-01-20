//! CLI for Caching Strategy Agent
//!
//! Provides test, simulate, and inspect commands for the Caching Strategy Agent.
//!
//! # Usage
//!
//! ```bash
//! # Test mode - validate cache eligibility
//! caching-strategy-cli test --input '{"execution_ref":"exec-123",...}'
//!
//! # Simulate mode - simulate cache decision with mock data
//! caching-strategy-cli simulate --request-type chat_completion --temperature 0.0
//!
//! # Inspect mode - show agent configuration
//! caching-strategy-cli inspect
//! ```

use clap::{Parser, Subcommand};
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use llm_edge_agents::caching_strategy::{
    self, CacheStrategyConfig, CacheStrategyRequest, CachingStrategyAgent,
};
use llm_edge_agents::caching_strategy::types::{
    CacheDecision, CacheLayer, CachePolicyConfig, CacheRelevantParams, CacheStrategyInput,
    ResponseMetadata,
};
use llm_edge_agents::contracts::ExecutionContext;

#[derive(Parser)]
#[command(name = "caching-strategy-cli")]
#[command(about = "CLI for Caching Strategy Agent - EXECUTION CONTROL")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", global = true)]
    log_level: String,

    /// Output format (json, text)
    #[arg(long, default_value = "json", global = true)]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Test cache eligibility with provided input
    Test {
        /// JSON input for cache strategy request
        #[arg(long)]
        input: String,

        /// Validate only (don't make decision)
        #[arg(long)]
        validate_only: bool,
    },

    /// Simulate cache decision with mock data
    Simulate {
        /// Request type (e.g., chat_completion, embedding)
        #[arg(long, default_value = "chat_completion")]
        request_type: String,

        /// Provider ID
        #[arg(long)]
        provider_id: Option<String>,

        /// Model ID
        #[arg(long)]
        model_id: Option<String>,

        /// Temperature setting
        #[arg(long)]
        temperature: Option<f32>,

        /// Simulate a cache hit
        #[arg(long)]
        cache_hit: bool,

        /// Simulate a write operation
        #[arg(long)]
        write: bool,

        /// Response size in bytes (for write simulation)
        #[arg(long)]
        response_size: Option<usize>,
    },

    /// Inspect agent configuration
    Inspect,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&cli.log_level)
        }))
        .init();

    match run(cli).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let config = CacheStrategyConfig::default();
    let agent = CachingStrategyAgent::new(config.clone());

    match cli.command {
        Commands::Test { input, validate_only } => {
            let input: CacheStrategyInput = serde_json::from_str(&input)?;

            if validate_only {
                let validation = caching_strategy::validate_cache_strategy(&input, &config)?;
                println!("{}", serde_json::to_string_pretty(&validation)?);
            } else {
                let request = CacheStrategyRequest {
                    input,
                    context: Some(ExecutionContext::new()),
                    config_override: None,
                };

                let response = agent.process(request).await?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        }

        Commands::Simulate {
            request_type,
            provider_id,
            model_id,
            temperature,
            cache_hit,
            write,
            response_size,
        } => {
            let execution_ref = uuid::Uuid::new_v4().to_string();
            let content_hash = format!("sim_{}", &uuid::Uuid::new_v4().to_string()[..8]);

            let existing_entry = if cache_hit {
                Some(caching_strategy::types::CacheEntryMetadata {
                    cache_key: format!("cache:sim:{}", content_hash),
                    layer: CacheLayer::L1,
                    created_at: chrono::Utc::now() - chrono::Duration::minutes(30),
                    expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
                    last_accessed_at: Some(chrono::Utc::now()),
                    access_count: 5,
                    response_size_bytes: 1000,
                    is_valid: true,
                    is_stale: false,
                    content_hash: content_hash.clone(),
                    semantic_hash: None,
                })
            } else {
                None
            };

            let response_metadata = if write {
                Some(ResponseMetadata {
                    response_size_bytes: response_size.unwrap_or(1000),
                    latency_ms: 250,
                    token_count: Some(150),
                    success: true,
                    contains_sensitive_data: false,
                    finish_reason: Some("stop".to_string()),
                })
            } else {
                None
            };

            let input = CacheStrategyInput {
                execution_ref,
                request_type,
                provider_id,
                model_id,
                content_hash,
                prompt_hash: format!("prompt_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                semantic_hash: None,
                cache_relevant_params: CacheRelevantParams {
                    temperature,
                    top_p: None,
                    max_tokens: None,
                    seed: Some(42),
                    response_format: None,
                    stream: false,
                    system_prompt_hash: None,
                },
                policy: CachePolicyConfig::default(),
                existing_entry,
                check_cache: true,
                is_write_operation: write,
                response_metadata,
                tenant_id: None,
                user_id_hash: None,
            };

            let request = CacheStrategyRequest {
                input,
                context: Some(ExecutionContext::new()),
                config_override: None,
            };

            let response = agent.process(request).await?;

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!("Decision: {:?}", response.output.decision);
                println!("Cacheable: {}", response.output.is_cacheable);
                println!("Cache Key: {:?}", response.output.cache_key);
                println!("Confidence: {}", response.output.confidence);
                println!("Reason: {}", response.output.reason);
                if !response.output.ineligibility_reasons.is_empty() {
                    println!("Ineligibility Reasons: {:?}", response.output.ineligibility_reasons);
                }
                println!("Processing Time: {}us", response.processing_time_us);
            }
        }

        Commands::Inspect => {
            let metadata = agent.metadata();
            let inspection = serde_json::json!({
                "agent_id": metadata.agent_id,
                "agent_version": metadata.agent_version,
                "agent_type": metadata.agent_type,
                "classification": "EXECUTION CONTROL",
                "decision_type": "cache_strategy_decision",
                "config": config,
                "capabilities": [
                    "Evaluate cache eligibility",
                    "Apply cache read/write/bypass decisions",
                    "Emit cache directives",
                    "Semantic caching support",
                    "Multi-layer cache recommendations",
                    "Stale-while-revalidate support"
                ],
                "non_responsibilities": [
                    "Perform orchestration (that is LLM-Orchestrator)",
                    "Modify policies dynamically",
                    "Trigger retries directly (retry logic lives in Orchestrator)",
                    "Emit alerts (that is Sentinel)",
                    "Perform analytics (that is Observatory/Latency-Lens)",
                    "Persist state locally (use ruvector-service only)",
                    "Execute SQL directly",
                    "Store actual response payloads (only metadata)"
                ],
                "upstream_invokers": [
                    "LLM-Orchestrator: During request processing for cache check",
                    "LLM-Orchestrator: After response for cache write decision",
                    "Direct API: For testing and validation"
                ],
                "cli_commands": {
                    "test": "caching-strategy-cli test --input <json>",
                    "simulate": "caching-strategy-cli simulate --request-type chat_completion --temperature 0.0",
                    "inspect": "caching-strategy-cli inspect"
                }
            });

            println!("{}", serde_json::to_string_pretty(&inspection)?);
        }
    }

    Ok(())
}
