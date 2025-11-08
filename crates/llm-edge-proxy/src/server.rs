//! HTTP server implementation
//!
//! Provides the core HTTP/HTTPS server for the proxy with full middleware stack.

pub mod routes;
pub mod tls;
pub mod tracing;

use crate::config::Config;
use crate::error::ProxyError;
use crate::middleware;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};

/// Build the Axum application with all middleware and routes
pub async fn build_app(config: Config) -> Result<Router, ProxyError> {
    // Build the router
    let app = Router::new()
        // Health check endpoints (no auth required by default)
        .route("/health", get(routes::health_check))
        .route("/health/ready", get(routes::readiness_check))
        .route("/health/live", get(routes::liveness_check))

        // Metrics endpoint (no auth required)
        .route("/metrics", get(routes::metrics))

        // Protected proxy endpoints
        .route("/v1/chat/completions", post(routes::chat_completions))
        .route("/v1/completions", post(routes::completions))

        // NOTE: Rate limiting temporarily disabled due to tower_governor API compatibility
        // TODO: Re-enable rate limiting once tower_governor integration is fixed
        // .layer(middleware::create_rate_limiter(&config))

        // Apply authentication middleware
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            middleware::auth_middleware,
        ))

        // Apply tower-http middleware
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())

        // Add shared state
        .with_state(config);

    Ok(app)
}

/// Creates the main application router (legacy compatibility)
pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(routes::health_check))
        .route("/health/ready", get(routes::readiness_check))
}

/// Starts the HTTP server
pub async fn serve(addr: SocketAddr, router: Router) -> anyhow::Result<()> {
    eprintln!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
