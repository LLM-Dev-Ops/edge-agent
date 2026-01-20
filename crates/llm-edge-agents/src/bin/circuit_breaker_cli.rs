//! Circuit Breaker Agent CLI
//!
//! CLI-invokable endpoint for testing, simulating, and inspecting the Circuit Breaker Agent.
//!
//! # Usage
//!
//! ```bash
//! # Test the circuit breaker
//! circuit-breaker-cli test --provider-id openai --failures 5 --successes 3
//!
//! # Simulate a scenario
//! circuit-breaker-cli simulate --provider-id openai --state open --failures 5
//!
//! # Inspect current state
//! circuit-breaker-cli inspect --provider-id openai
//!
//! # Start the HTTP server
//! circuit-breaker-cli serve --port 8080
//! ```

use std::net::SocketAddr;
use std::process::ExitCode;

use chrono::Utc;
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use llm_edge_agents::circuit_breaker::{
    CircuitBreakerAgent, CircuitBreakerConfig, CircuitBreakerDecisionEngine,
    CircuitBreakerHandler, CircuitBreakerRequest, CircuitState, FailureSignal,
    SuccessSignal,
};

/// Circuit Breaker Agent CLI
#[derive(Parser)]
#[command(name = "circuit-breaker-cli")]
#[command(author = "LLM Edge Agent Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "CLI for Circuit Breaker Agent - PROTECTION / GUARD")]
#[command(long_about = r#"
Circuit Breaker Agent - Prevents cascading failures

Classification: PROTECTION / GUARD
Decision Type: circuit_breaker_decision

This agent:
- Tracks failure signals (input-provided only)
- Applies circuit open / close decisions
- Emits block or allow outcomes
- Persists state via ruvector-service (when enabled)

This agent MUST NOT:
- Perform orchestration
- Modify policies dynamically
- Trigger retries directly
- Emit alerts
- Perform analytics
- Persist state locally
"#)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Test the circuit breaker with simulated signals
    Test {
        /// Provider ID to test
        #[arg(short, long)]
        provider_id: String,

        /// Number of failures to simulate
        #[arg(short, long, default_value = "0")]
        failures: u32,

        /// Number of successes to simulate
        #[arg(short, long, default_value = "0")]
        successes: u32,

        /// Failure threshold (default: 5)
        #[arg(long, default_value = "5")]
        failure_threshold: u32,

        /// Success threshold (default: 3)
        #[arg(long, default_value = "3")]
        success_threshold: u32,

        /// Timeout seconds (default: 30)
        #[arg(long, default_value = "30")]
        timeout_seconds: u32,

        /// Output format (json, text)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Simulate a specific scenario (dry-run, no persistence)
    Simulate {
        /// Provider ID
        #[arg(short, long)]
        provider_id: String,

        /// Initial circuit state (closed, open, half_open)
        #[arg(short, long, default_value = "closed")]
        state: String,

        /// Initial failure count
        #[arg(long, default_value = "0")]
        failures: u32,

        /// Initial success count
        #[arg(long, default_value = "0")]
        successes: u32,

        /// Add a failure signal
        #[arg(long)]
        add_failure: bool,

        /// Add a success signal
        #[arg(long)]
        add_success: bool,

        /// Output format (json, text)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Inspect agent configuration and state
    Inspect {
        /// Provider ID to inspect
        #[arg(short, long)]
        provider_id: String,

        /// Output format (json, text)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Start the HTTP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    // Run the appropriate command
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    match cli.command {
        Commands::Test {
            provider_id,
            failures,
            successes,
            failure_threshold,
            success_threshold,
            timeout_seconds,
            format,
        } => {
            rt.block_on(async {
                run_test(
                    &provider_id,
                    failures,
                    successes,
                    failure_threshold,
                    success_threshold,
                    timeout_seconds,
                    &format,
                )
                .await
            })
        }

        Commands::Simulate {
            provider_id,
            state,
            failures,
            successes,
            add_failure,
            add_success,
            format,
        } => {
            rt.block_on(async {
                run_simulate(
                    &provider_id,
                    &state,
                    failures,
                    successes,
                    add_failure,
                    add_success,
                    &format,
                )
                .await
            })
        }

        Commands::Inspect {
            provider_id,
            format,
        } => rt.block_on(async { run_inspect(&provider_id, &format).await }),

        Commands::Serve { port, host } => rt.block_on(async { run_serve(&host, port).await }),
    }
}

async fn run_test(
    provider_id: &str,
    failures: u32,
    successes: u32,
    failure_threshold: u32,
    success_threshold: u32,
    timeout_seconds: u32,
    format: &str,
) -> ExitCode {
    info!("Testing circuit breaker for provider: {}", provider_id);

    let config = CircuitBreakerConfig {
        provider_id: provider_id.to_string(),
        failure_threshold,
        success_threshold,
        timeout_seconds,
        ..Default::default()
    };

    let agent = CircuitBreakerAgent::new(None);
    let mut state = CircuitState::Closed;
    let mut failure_count = 0u32;
    let mut success_count = 0u32;
    let mut results = Vec::new();

    // Process failures
    for i in 0..failures {
        let signal = FailureSignal::new(provider_id, "test_failure");
        let request = CircuitBreakerRequest::record_failure(
            provider_id,
            format!("test-{}", i),
            signal,
        )
        .with_state(state, failure_count, success_count)
        .with_config(config.clone());

        match agent.invoke(request).await {
            Ok(response) => {
                results.push(json!({
                    "step": i + 1,
                    "type": "failure",
                    "decision": format!("{:?}", response.decision),
                    "state": format!("{:?}", response.new_state),
                    "failures": response.new_failure_count,
                    "successes": response.new_success_count,
                }));
                state = response.new_state;
                failure_count = response.new_failure_count;
                success_count = response.new_success_count;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Process successes
    for i in 0..successes {
        let signal = SuccessSignal::new(provider_id);
        let request = CircuitBreakerRequest::record_success(
            provider_id,
            format!("test-s-{}", i),
            signal,
        )
        .with_state(state, failure_count, success_count)
        .with_config(config.clone());

        match agent.invoke(request).await {
            Ok(response) => {
                results.push(json!({
                    "step": failures + i + 1,
                    "type": "success",
                    "decision": format!("{:?}", response.decision),
                    "state": format!("{:?}", response.new_state),
                    "failures": response.new_failure_count,
                    "successes": response.new_success_count,
                }));
                state = response.new_state;
                failure_count = response.new_failure_count;
                success_count = response.new_success_count;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Output results
    let output = json!({
        "provider_id": provider_id,
        "config": {
            "failure_threshold": failure_threshold,
            "success_threshold": success_threshold,
            "timeout_seconds": timeout_seconds,
        },
        "initial_state": "closed",
        "final_state": format!("{:?}", state),
        "final_failures": failure_count,
        "final_successes": success_count,
        "steps": results,
    });

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("\n=== Circuit Breaker Test Results ===");
        println!("Provider: {}", provider_id);
        println!("Configuration:");
        println!("  - Failure Threshold: {}", failure_threshold);
        println!("  - Success Threshold: {}", success_threshold);
        println!("  - Timeout: {} seconds", timeout_seconds);
        println!("\nResults:");
        println!("  - Initial State: Closed");
        println!("  - Final State: {:?}", state);
        println!("  - Final Failures: {}", failure_count);
        println!("  - Final Successes: {}", success_count);
        println!("\nSteps:");
        for (i, step) in results.iter().enumerate() {
            println!(
                "  {}. [{}] {} -> State: {}, Failures: {}, Successes: {}",
                i + 1,
                step["type"].as_str().unwrap_or("unknown"),
                step["decision"].as_str().unwrap_or("unknown"),
                step["state"].as_str().unwrap_or("unknown"),
                step["failures"].as_u64().unwrap_or(0),
                step["successes"].as_u64().unwrap_or(0)
            );
        }
    }

    ExitCode::SUCCESS
}

async fn run_simulate(
    provider_id: &str,
    state_str: &str,
    failures: u32,
    successes: u32,
    add_failure: bool,
    add_success: bool,
    format: &str,
) -> ExitCode {
    info!("Simulating circuit breaker scenario for: {}", provider_id);

    let state = match state_str.to_lowercase().as_str() {
        "closed" => CircuitState::Closed,
        "open" => CircuitState::Open,
        "half_open" | "halfopen" => CircuitState::HalfOpen,
        _ => {
            eprintln!("Invalid state: {}. Use: closed, open, half_open", state_str);
            return ExitCode::FAILURE;
        }
    };

    let mut request = CircuitBreakerRequest::check(provider_id, "simulate")
        .with_state(state, failures, successes);

    if add_failure {
        request.failure_signal = Some(FailureSignal::new(provider_id, "simulated"));
    }
    if add_success {
        request.success_signal = Some(SuccessSignal::new(provider_id));
    }

    let response = CircuitBreakerDecisionEngine::evaluate(&request);

    let output = json!({
        "provider_id": provider_id,
        "input": {
            "state": format!("{:?}", state),
            "failures": failures,
            "successes": successes,
            "add_failure": add_failure,
            "add_success": add_success,
        },
        "output": {
            "decision": format!("{:?}", response.decision),
            "new_state": format!("{:?}", response.new_state),
            "new_failures": response.new_failure_count,
            "new_successes": response.new_success_count,
            "confidence": response.confidence,
            "reason": response.reason,
            "state_transition": response.state_transition,
        }
    });

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("\n=== Circuit Breaker Simulation ===");
        println!("Provider: {}", provider_id);
        println!("\nInput:");
        println!("  - State: {:?}", state);
        println!("  - Failures: {}", failures);
        println!("  - Successes: {}", successes);
        println!("  - Add Failure: {}", add_failure);
        println!("  - Add Success: {}", add_success);
        println!("\nOutput:");
        println!("  - Decision: {:?}", response.decision);
        println!("  - New State: {:?}", response.new_state);
        println!("  - New Failures: {}", response.new_failure_count);
        println!("  - New Successes: {}", response.new_success_count);
        println!("  - Confidence: {}", response.confidence);
        println!("  - Reason: {}", response.reason);
        if let Some(ref transition) = response.state_transition {
            println!("  - Transition: {:?} -> {:?}", transition.from, transition.to);
            println!("    Trigger: {}", transition.trigger);
        }
    }

    ExitCode::SUCCESS
}

async fn run_inspect(provider_id: &str, format: &str) -> ExitCode {
    info!("Inspecting circuit breaker for: {}", provider_id);

    let config = CircuitBreakerConfig::from_env(provider_id);
    let agent = CircuitBreakerAgent::new(None);
    let metadata = agent.metadata();

    let output = json!({
        "agent": {
            "id": metadata.agent_id,
            "version": metadata.agent_version,
            "type": format!("{:?}", metadata.agent_type),
        },
        "provider_id": provider_id,
        "config": {
            "failure_threshold": config.failure_threshold,
            "success_threshold": config.success_threshold,
            "timeout_seconds": config.timeout_seconds,
            "failure_rate_threshold": config.failure_rate_threshold,
        },
        "classification": {
            "type": "PROTECTION / GUARD",
            "decision_type": "circuit_breaker_decision",
        },
        "non_responsibilities": [
            "Perform orchestration",
            "Modify policies dynamically",
            "Trigger retries directly",
            "Emit alerts",
            "Perform analytics",
            "Persist state locally"
        ]
    });

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("\n=== Circuit Breaker Agent Inspection ===");
        println!("\nAgent:");
        println!("  - ID: {}", metadata.agent_id);
        println!("  - Version: {}", metadata.agent_version);
        println!("  - Type: {:?}", metadata.agent_type);
        println!("\nProvider: {}", provider_id);
        println!("\nConfiguration (from environment):");
        println!("  - Failure Threshold: {}", config.failure_threshold);
        println!("  - Success Threshold: {}", config.success_threshold);
        println!("  - Timeout: {} seconds", config.timeout_seconds);
        if let Some(rate) = config.failure_rate_threshold {
            println!("  - Failure Rate Threshold: {:.2}", rate);
        }
        println!("\nClassification:");
        println!("  - Type: PROTECTION / GUARD");
        println!("  - Decision Type: circuit_breaker_decision");
        println!("\nNon-Responsibilities:");
        println!("  - Perform orchestration (LLM-Orchestrator)");
        println!("  - Modify policies dynamically");
        println!("  - Trigger retries directly");
        println!("  - Emit alerts (LLM-Sentinel)");
        println!("  - Perform analytics (LLM-Observatory)");
        println!("  - Persist state locally");
    }

    ExitCode::SUCCESS
}

async fn run_serve(host: &str, port: u16) -> ExitCode {
    info!("Starting Circuit Breaker Agent HTTP server on {}:{}", host, port);

    let handler = match CircuitBreakerHandler::from_env() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to create handler: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let app = handler.router();

    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    println!("\n=== Circuit Breaker Agent Server ===");
    println!("Listening on: http://{}", addr);
    println!("\nEndpoints:");
    println!("  POST /invoke    - Invoke the agent");
    println!("  POST /test      - Run test scenario");
    println!("  POST /simulate  - Dry-run simulation");
    println!("  GET  /inspect/:provider_id - Inspect state");
    println!("  GET  /health    - Health check");
    println!("  GET  /metrics   - Prometheus metrics");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    ExitCode::SUCCESS
}
