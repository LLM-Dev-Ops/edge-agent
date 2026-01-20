//! CLI for Tool Invocation Agent
//!
//! This CLI provides commands for testing, simulating, and inspecting
//! the Tool Invocation Agent. It is designed for both development
//! and production use.
//!
//! # Commands
//!
//! ```bash
//! # Test mode - validate without execution
//! tool-invocation-cli test --input <json>
//! tool-invocation-cli test --file <path>
//!
//! # Simulate mode - simulate decision with mock data
//! tool-invocation-cli simulate --tool <name> --params <json>
//!
//! # Inspect mode - show agent configuration
//! tool-invocation-cli inspect
//! tool-invocation-cli inspect --json
//!
//! # Serve mode - start HTTP server (Edge Function)
//! tool-invocation-cli serve --port 8080
//! ```

use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use llm_edge_agents::contracts::ExecutionContext;
use llm_edge_agents::tool_invocation::{
    agent_metadata, tool_invocation_handler, validate_invocation, RuvectorConfig,
    ToolDefinition, ToolInvocationAgent, ToolInvocationConfig, ToolInvocationRequest,
};

/// CLI argument parser (minimal, no external dependencies)
#[derive(Debug)]
struct Args {
    command: Command,
}

#[derive(Debug)]
enum Command {
    Test {
        input: Option<String>,
        file: Option<String>,
    },
    Simulate {
        tool: String,
        params: String,
    },
    Inspect {
        json: bool,
    },
    Serve {
        host: String,
        port: u16,
        ruvector_url: Option<String>,
    },
    Help,
    Version,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        return Ok(Args {
            command: Command::Help,
        });
    }

    let command = match args[1].as_str() {
        "test" => {
            let mut input = None;
            let mut file = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--input" | "-i" => {
                        i += 1;
                        if i < args.len() {
                            input = Some(args[i].clone());
                        }
                    }
                    "--file" | "-f" => {
                        i += 1;
                        if i < args.len() {
                            file = Some(args[i].clone());
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            Command::Test { input, file }
        }
        "simulate" => {
            let mut tool = String::new();
            let mut params = "{}".to_string();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--tool" | "-t" => {
                        i += 1;
                        if i < args.len() {
                            tool = args[i].clone();
                        }
                    }
                    "--params" | "-p" => {
                        i += 1;
                        if i < args.len() {
                            params = args[i].clone();
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if tool.is_empty() {
                return Err("--tool is required for simulate command".to_string());
            }
            Command::Simulate { tool, params }
        }
        "inspect" => {
            let json = args.iter().any(|a| a == "--json" || a == "-j");
            Command::Inspect { json }
        }
        "serve" => {
            let mut host = "0.0.0.0".to_string();
            let mut port = 8080u16;
            let mut ruvector_url = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--host" | "-h" => {
                        i += 1;
                        if i < args.len() {
                            host = args[i].clone();
                        }
                    }
                    "--port" | "-p" => {
                        i += 1;
                        if i < args.len() {
                            port = args[i].parse().unwrap_or(8080);
                        }
                    }
                    "--ruvector-url" | "-r" => {
                        i += 1;
                        if i < args.len() {
                            ruvector_url = Some(args[i].clone());
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            Command::Serve {
                host,
                port,
                ruvector_url,
            }
        }
        "--help" | "-h" | "help" => Command::Help,
        "--version" | "-v" | "version" => Command::Version,
        cmd => return Err(format!("Unknown command: {}", cmd)),
    };

    Ok(Args { command })
}

fn print_help() {
    println!(
        r#"Tool Invocation Agent CLI

USAGE:
    tool-invocation-cli <COMMAND> [OPTIONS]

COMMANDS:
    test        Validate a tool invocation request without execution
    simulate    Simulate a decision with mock data
    inspect     Show agent configuration and metadata
    serve       Start HTTP server (Edge Function mode)
    help        Show this help message
    version     Show version information

OPTIONS:
    -h, --help       Show help
    -v, --version    Show version

EXAMPLES:
    # Test a request from JSON string
    tool-invocation-cli test --input '{"tool": {"name": "calc"}, "arguments": {}}'

    # Test a request from file
    tool-invocation-cli test --file request.json

    # Simulate with a tool name and params
    tool-invocation-cli simulate --tool calculator --params '{"x": 1, "y": 2}'

    # Inspect agent configuration
    tool-invocation-cli inspect --json

    # Start HTTP server
    tool-invocation-cli serve --port 8080 --ruvector-url http://localhost:8081
"#
    );
}

fn print_version() {
    let metadata = agent_metadata();
    println!("Tool Invocation Agent CLI");
    println!("Agent ID: {}", metadata.agent_id);
    println!("Version: {}", metadata.agent_version);
    println!("Type: {:?}", metadata.agent_type);
}

async fn run_test(input: Option<String>, file: Option<String>) -> Result<(), String> {
    // Get input JSON
    let json_str = if let Some(input) = input {
        input
    } else if let Some(file) = file {
        fs::read_to_string(&file).map_err(|e| format!("Failed to read file: {}", e))?
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;
        buffer
    };

    // Parse request
    let request: ToolInvocationRequest =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Validate
    let config = ToolInvocationConfig::default();
    let result =
        validate_invocation(&request, &config).map_err(|e| format!("Validation error: {}", e))?;

    // Output result
    let output = serde_json::json!({
        "valid": result.valid,
        "schema_valid": result.schema_valid,
        "security_valid": result.security_valid,
        "constraints_valid": result.constraints_valid,
        "errors": result.errors,
        "warnings": result.warnings,
        "would_allow": result.valid,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());

    if result.valid {
        Ok(())
    } else {
        Err("Validation failed".to_string())
    }
}

async fn run_simulate(tool: String, params: String) -> Result<(), String> {
    // Parse params
    let arguments: serde_json::Value =
        serde_json::from_str(&params).map_err(|e| format!("Failed to parse params: {}", e))?;

    // Build request
    let request = ToolInvocationRequest {
        tool: ToolDefinition {
            name: tool.clone(),
            description: Some(format!("Simulated tool: {}", tool)),
            parameters: vec![],
            capabilities: vec![],
        },
        arguments,
        context: Some(ExecutionContext::new()),
        constraints: None,
    };

    // Process
    let config = ToolInvocationConfig::default();
    let agent = ToolInvocationAgent::new(config);
    let response = agent
        .process(request)
        .await
        .map_err(|e| format!("Processing error: {}", e))?;

    // Output result
    println!("{}", serde_json::to_string_pretty(&response).unwrap());

    if response.decision.allowed {
        Ok(())
    } else {
        Err(format!("Tool blocked: {}", response.decision.reason))
    }
}

fn run_inspect(json: bool) {
    let metadata = agent_metadata();
    let config = ToolInvocationConfig::default();

    if json {
        let output = serde_json::json!({
            "agent_id": metadata.agent_id,
            "agent_version": metadata.agent_version,
            "agent_type": format!("{:?}", metadata.agent_type),
            "classification": "EXECUTION CONTROL",
            "decision_type": "tool_invocation_decision",
            "config": config,
            "endpoints": [
                {"method": "POST", "path": "/invoke", "description": "Process tool invocation"},
                {"method": "POST", "path": "/test", "description": "Test validation"},
                {"method": "POST", "path": "/simulate", "description": "Simulate decision"},
                {"method": "GET", "path": "/inspect", "description": "Get agent info"},
                {"method": "GET", "path": "/health", "description": "Health check"},
            ],
            "non_responsibilities": [
                "Perform orchestration (that is LLM-Orchestrator)",
                "Modify policies dynamically",
                "Trigger retries directly",
                "Emit alerts (that is Sentinel)",
                "Perform analytics (that is Observatory)",
                "Persist state locally",
                "Execute SQL directly",
            ],
            "upstream_invocations": [
                "LLM-Orchestrator",
                "LLM-Shield",
                "Direct API",
            ],
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║           TOOL INVOCATION AGENT                          ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Agent ID:       {}                      ║", metadata.agent_id);
        println!("║ Version:        {}                                   ║", metadata.agent_version);
        println!("║ Type:           {:?}                      ║", metadata.agent_type);
        println!("║ Classification: EXECUTION CONTROL                        ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ PURPOSE                                                  ║");
        println!("║ Intercept and validate tool calls before execution.      ║");
        println!("║ Enforce invocation constraints.                          ║");
        println!("║ Allow or block execution.                                ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ DECISION TYPE                                            ║");
        println!("║ tool_invocation_decision                                 ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ ENDPOINTS                                                ║");
        println!("║ POST /invoke   - Process tool invocation                 ║");
        println!("║ POST /test     - Test validation                         ║");
        println!("║ POST /simulate - Simulate decision                       ║");
        println!("║ GET  /inspect  - Get agent info                          ║");
        println!("║ GET  /health   - Health check                            ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ NON-RESPONSIBILITIES (MUST NEVER DO)                     ║");
        println!("║ • Perform orchestration                                  ║");
        println!("║ • Modify policies dynamically                            ║");
        println!("║ • Trigger retries directly                               ║");
        println!("║ • Emit alerts                                            ║");
        println!("║ • Perform analytics                                      ║");
        println!("║ • Persist state locally                                  ║");
        println!("║ • Execute SQL directly                                   ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        println!("Global Blocked Tools:");
        for tool in &config.global_blocked_tools {
            println!("  - {}", tool);
        }
        println!();
        println!("Dangerous Patterns:");
        for pattern in &config.dangerous_patterns {
            println!("  - {}", pattern);
        }
    }
}

async fn run_serve(host: String, port: u16, ruvector_url: Option<String>) -> Result<(), String> {
    println!("Starting Tool Invocation Agent server...");
    println!("Host: {}:{}", host, port);

    let app = if let Some(url) = ruvector_url {
        println!("ruvector-service: {}", url);
        let config = ToolInvocationConfig::default();
        let ruvector_config = RuvectorConfig {
            base_url: url,
            ..Default::default()
        };
        llm_edge_agents::tool_invocation::tool_invocation_handler_with_ruvector(
            config,
            ruvector_config,
        )
        .map_err(|e| format!("Failed to create handler: {}", e))?
    } else {
        println!("ruvector-service: not configured (events will not be persisted)");
        tool_invocation_handler()
    };

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    println!();
    println!("Tool Invocation Agent is ready!");
    println!("Endpoints:");
    println!("  POST http://{}:{}/invoke", host, port);
    println!("  POST http://{}:{}/test", host, port);
    println!("  POST http://{}:{}/simulate", host, port);
    println!("  GET  http://{}:{}/inspect", host, port);
    println!("  GET  http://{}:{}/health", host, port);
    println!();

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Use --help for usage information");
            return ExitCode::FAILURE;
        }
    };

    let result = match args.command {
        Command::Test { input, file } => run_test(input, file).await,
        Command::Simulate { tool, params } => run_simulate(tool, params).await,
        Command::Inspect { json } => {
            run_inspect(json);
            Ok(())
        }
        Command::Serve {
            host,
            port,
            ruvector_url,
        } => run_serve(host, port, ruvector_url).await,
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            print_version();
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
