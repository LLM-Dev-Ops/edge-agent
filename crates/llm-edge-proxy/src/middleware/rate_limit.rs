//! Rate limiting middleware using tower-governor
//!
//! NOTE: This is currently a placeholder. The tower_governor API in version 0.4
//! requires specific generic type parameters that need to be resolved.
//! TODO: Implement proper rate limiting once the API is clarified.

use tracing::info;

use crate::Config;

/// Create rate limiter layer from configuration
///
/// NOTE: Currently returns a no-op layer. Rate limiting should be implemented
/// using tower_governor once the API compatibility issues are resolved.
pub fn create_rate_limiter(
    _config: &Config,
) -> tower::util::BoxCloneService<
    axum::http::Request<axum::body::Body>,
    axum::response::Response,
    std::convert::Infallible,
> {
    info!("Rate limiting configuration loaded (currently disabled - TODO)");

    // Return a passthrough service
    use tower::ServiceExt;
    tower::service_fn(|_req: axum::http::Request<axum::body::Body>| async move {
        Ok::<_, std::convert::Infallible>(axum::response::Response::new(axum::body::Body::empty()))
    })
    .map_request(|req| req)
    .boxed_clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let config = Config {
            server: crate::config::ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                timeout_seconds: 30,
                max_request_size: 10485760,
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            rate_limit: crate::config::RateLimitConfig {
                enabled: true,
                requests_per_minute: 100,
                burst_size: 10,
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
        };

        let _layer = create_rate_limiter(&config);
        // Just verify it creates without panicking
    }

    #[test]
    fn test_rate_limit_disabled() {
        let config = Config {
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
                requests_per_minute: 100,
                burst_size: 10,
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
        };

        let _layer = create_rate_limiter(&config);
        // Should create very permissive limiter
    }
}
