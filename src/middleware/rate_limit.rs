//! Rate limiting middleware using tower-governor

use axum::extract::Request;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;
use tower::Layer;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tracing::info;

use crate::config::Config;

/// Create rate limiter layer from configuration
pub fn create_rate_limiter(config: &Config) -> GovernorLayer<'static, NotKeyed, InMemoryState> {
    if !config.rate_limit.enabled {
        info!("Rate limiting disabled");
        // Return a very permissive rate limiter
        let governor_conf = Box::new(
            GovernorConfigBuilder::default()
                .per_minute(NonZeroU32::new(1_000_000).unwrap())
                .burst_size(NonZeroU32::new(100_000).unwrap())
                .finish()
                .unwrap(),
        );
        return GovernorLayer {
            config: Box::leak(governor_conf),
        };
    }

    info!(
        requests_per_minute = config.rate_limit.requests_per_minute,
        burst_size = config.rate_limit.burst_size,
        "Rate limiting enabled"
    );

    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_minute(NonZeroU32::new(config.rate_limit.requests_per_minute).unwrap())
            .burst_size(NonZeroU32::new(config.rate_limit.burst_size).unwrap())
            .finish()
            .unwrap(),
    );

    GovernorLayer {
        config: Box::leak(governor_conf),
    }
}

/// Custom rate limiter that can extract keys from requests
pub struct KeyedRateLimiter {
    limiter: Arc<GovernorRateLimiter<String, InMemoryState, DefaultClock>>,
}

impl KeyedRateLimiter {
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap())
            .allow_burst(NonZeroU32::new(burst_size).unwrap());
        
        let limiter = Arc::new(GovernorRateLimiter::keyed(quota));
        
        Self { limiter }
    }

    /// Check rate limit for a specific key (e.g., API key, user ID)
    pub fn check_key(&self, key: &str) -> bool {
        self.limiter.check_key(&key.to_string()).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_keyed_rate_limiter() {
        let limiter = KeyedRateLimiter::new(60, 10); // 60 per minute, burst of 10
        
        let key = "test-key";
        
        // First 10 requests should succeed (burst)
        for _ in 0..10 {
            assert!(limiter.check_key(key));
        }
        
        // 11th request should fail
        assert!(!limiter.check_key(key));
        
        // Different key should have its own limit
        assert!(limiter.check_key("other-key"));
    }

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

        let layer = create_rate_limiter(&config);
        // Just verify it creates without panicking
        assert!(true);
    }
}
