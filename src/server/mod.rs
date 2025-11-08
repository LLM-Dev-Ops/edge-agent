//! HTTP server implementation using Axum

pub mod routes;
pub mod tls;
pub mod tracing;

use crate::config::Config;
use crate::error::AppError;
use crate::middleware::{auth, rate_limit, timeout};
use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};

/// Build the Axum application with all middleware and routes
pub async fn build_app(config: Config) -> Result<Router, AppError> {
    // Build middleware stack
    let middleware_stack = ServiceBuilder::new()
        // Add tracing for all requests
        .layer(TraceLayer::new_for_http())
        // Add compression
        .layer(CompressionLayer::new())
        // Add CORS (permissive for now, should be configurable)
        .layer(CorsLayer::permissive())
        // Add request timeout
        .layer(timeout::TimeoutLayer::new(config.timeout_duration()));

    // Build the router
    let app = Router::new()
        // Health check endpoint (no auth required by default)
        .route("/health", get(routes::health_check))
        .route("/health/ready", get(routes::readiness_check))
        .route("/health/live", get(routes::liveness_check))
        
        // Metrics endpoint (no auth required)
        .route("/metrics", get(routes::metrics))
        
        // Protected proxy endpoints
        .route("/v1/chat/completions", post(routes::chat_completions))
        .route("/v1/completions", post(routes::completions))
        
        // Apply rate limiting to proxy endpoints
        .layer(rate_limit::create_rate_limiter(&config))
        
        // Apply authentication middleware
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            auth::auth_middleware,
        ))
        
        // Apply global middleware
        .layer(middleware_stack)
        
        // Add shared state
        .with_state(config);

    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let config = Config::from_env().unwrap_or_else(|_| {
            // Provide test defaults
            Config {
                server: crate::config::ServerConfig {
                    address: "127.0.0.1:8080".to_string(),
                    timeout_seconds: 30,
                    max_request_size: 10485760,
                    enable_tls: false,
                    tls_cert_path: None,
                    tls_key_path: None,
                },
                rate_limit: crate::config::RateLimitConfig {
                    enabled: false,
                    requests_per_minute: 1000,
                    burst_size: 100,
                },
                auth: crate::config::AuthConfig {
                    enabled: false,
                    api_keys: vec![],
                    require_auth_for_health: false,
                },
                observability: crate::config::ObservabilityConfig {
                    enable_tracing: false,
                    enable_metrics: false,
                    log_level: "info".to_string(),
                    otlp_endpoint: None,
                },
            }
        });

        let app = build_app(config).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
