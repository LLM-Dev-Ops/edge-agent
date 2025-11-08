//! LLM Edge Agent - Main Application Entry Point
//!
//! High-performance intercepting proxy for LLM providers.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use llm_edge_agent::{
    init_tracing, shutdown_tracing, MetricsRegistry, Provider, RoutingEngine,
    SystemMetrics, TracingConfig,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

/// Application state
#[derive(Clone)]
struct AppState {
    routing_engine: Arc<RoutingEngine>,
    metrics: Arc<MetricsRegistry>,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    service: String,
}

/// LLM completion request (simplified)
#[derive(Debug, Deserialize)]
struct CompletionRequest {
    model: String,
    prompt: String,
    max_tokens: Option<u32>,
}

/// LLM completion response (simplified)
#[derive(Debug, Serialize)]
struct CompletionResponse {
    id: String,
    model: String,
    text: String,
    provider: String,
}

/// Error response
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

/// Custom error type
enum AppError {
    RoutingError(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::RoutingError(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            error: status.to_string(),
            message: error_message,
        });

        (status, body).into_response()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let tracing_config = TracingConfig::default();
    init_tracing(tracing_config)?;

    info!(
        version = llm_edge_agent::VERSION,
        "Starting LLM Edge Agent"
    );

    // Initialize metrics
    let metrics = Arc::new(MetricsRegistry::new()?);
    info!("Metrics registry initialized");

    // Initialize providers (in production, load from config)
    let providers = vec![
        Provider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            endpoint: "https://api.openai.com/v1".to_string(),
            priority: 1,
            cost_per_1k_tokens: 0.03,
            max_tokens: 4096,
            enabled: true,
        },
        Provider {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            endpoint: "https://api.anthropic.com/v1".to_string(),
            priority: 2,
            cost_per_1k_tokens: 0.025,
            max_tokens: 8192,
            enabled: true,
        },
    ];

    // Initialize routing engine with round-robin strategy
    let routing_engine = Arc::new(RoutingEngine::with_round_robin(providers));
    info!("Routing engine initialized with round-robin strategy");

    // Create application state
    let app_state = AppState {
        routing_engine,
        metrics: metrics.clone(),
    };

    // Build application router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/completions", post(completions_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Cleanup
    shutdown_tracing();
    info!("LLM Edge Agent shutdown complete");

    Ok(())
}

/// Health check endpoint
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: llm_edge_agent::VERSION.to_string(),
        service: llm_edge_agent::NAME.to_string(),
    })
}

/// Metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> String {
    state.metrics.render()
}

/// Completions endpoint (simplified demonstration)
async fn completions_handler(
    State(state): State<AppState>,
    Json(request): Json<CompletionRequest>,
) -> Result<Json<CompletionResponse>, AppError> {
    info!(
        model = %request.model,
        prompt_length = request.prompt.len(),
        "Received completion request"
    );

    // Track active connection
    SystemMetrics::increment_connections();

    // Route the request through the routing engine
    let result = state
        .routing_engine
        .route(|provider| {
            let req = request.clone();
            Box::pin(async move {
                // In production, this would make actual HTTP request to provider
                info!(
                    provider = %provider.id,
                    model = %req.model,
                    "Routing request to provider"
                );
                
                // Simulate provider response
                Ok::<_, std::io::Error>(CompletionResponse {
                    id: uuid::Uuid::new_v4().to_string(),
                    model: req.model.clone(),
                    text: "This is a simulated response".to_string(),
                    provider: provider.id.clone(),
                })
            })
        })
        .await;

    // Track connection closed
    SystemMetrics::decrement_connections();

    match result {
        Ok(response) => {
            info!(
                provider = %response.provider,
                "Request completed successfully"
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!(error = %e, "Request failed");
            Err(AppError::RoutingError(e.to_string()))
        }
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }

    info!("Initiating graceful shutdown");
}
