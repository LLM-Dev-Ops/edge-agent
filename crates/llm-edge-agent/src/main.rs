use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use llm_edge_agent::{
    check_system_health, handle_chat_completions, initialize_app_state, AppConfig,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "llm_edge_agent=info,llm_edge_cache=info,tower_http=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting LLM Edge Agent v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration from environment
    info!("Loading configuration");
    let config = AppConfig::from_env();
    info!(
        "Configuration loaded: host={}, port={}, l2_cache_enabled={}",
        config.host, config.port, config.enable_l2_cache
    );

    // Initialize Prometheus metrics exporter
    if config.enable_metrics {
        info!(
            "Initializing Prometheus metrics exporter on port {}",
            config.metrics_port
        );
        PrometheusBuilder::new()
            .install()
            .expect("Failed to install Prometheus exporter");
    }

    // Initialize application state (cache, providers, etc.)
    info!("Initializing application state");
    let app_state = match initialize_app_state(config.clone()).await {
        Ok(state) => {
            info!("Application state initialized successfully");
            Arc::new(state)
        }
        Err(e) => {
            error!("Failed to initialize application state: {}", e);
            return Err(e);
        }
    };

    // Perform initial health check
    info!("Performing initial health check");
    let health = check_system_health(&app_state).await;
    info!(
        "Health check: status={}, cache_l1={}, cache_l2={}, openai={}, anthropic={}",
        health.status_string(),
        health.cache_l1_healthy,
        health.cache_l2_healthy,
        health.openai_healthy,
        health.anthropic_healthy
    );

    if !health.is_healthy() {
        warn!("System health check failed, but continuing startup");
    }

    // Build the HTTP router
    info!("Building HTTP router");
    let app = Router::new()
        // Health check endpoints
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/health/live", get(liveness_handler))
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        // Main proxy endpoints (OpenAI-compatible)
        .route("/v1/chat/completions", post(handle_chat_completions))
        // Share application state with handlers
        .with_state(app_state.clone());

    // Start the HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("LLM Edge Agent is ready to accept requests!");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check handler
async fn health_handler(
    axum::extract::State(state): axum::extract::State<Arc<llm_edge_agent::AppState>>,
) -> axum::Json<serde_json::Value> {
    let health = check_system_health(&state).await;

    axum::Json(serde_json::json!({
        "status": health.status_string(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "cache": {
            "l1_healthy": health.cache_l1_healthy,
            "l2_healthy": health.cache_l2_healthy,
            "l2_configured": health.cache_l2_configured,
        },
        "providers": {
            "openai": {
                "configured": health.openai_configured,
                "healthy": health.openai_healthy,
            },
            "anthropic": {
                "configured": health.anthropic_configured,
                "healthy": health.anthropic_healthy,
            },
        },
    }))
}

/// Readiness check handler
async fn readiness_handler(
    axum::extract::State(state): axum::extract::State<Arc<llm_edge_agent::AppState>>,
) -> axum::Json<serde_json::Value> {
    let health = check_system_health(&state).await;

    let ready = health.is_healthy();

    axum::Json(serde_json::json!({
        "ready": ready,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Liveness check handler
async fn liveness_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "alive": true,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Prometheus metrics handler
async fn metrics_handler() -> String {
    // Get the metrics handle from the global registry
    // In a real implementation, we'd store this in the app state
    // For now, return a basic response
    "# Prometheus metrics\n# See /health for detailed status\n".to_string()
}
