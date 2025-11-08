//! Configuration management for LLM Edge Agent

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub rate_limit: RateLimitConfig,
    pub auth: AuthConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub timeout_seconds: u64,
    pub max_request_size: usize,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub api_keys: Vec<String>,
    pub require_auth_for_health: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub enable_tracing: bool,
    pub enable_metrics: bool,
    pub log_level: String,
    pub otlp_endpoint: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let server = ServerConfig {
            address: std::env::var("SERVER_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            timeout_seconds: std::env::var("SERVER_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            max_request_size: std::env::var("MAX_REQUEST_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse()?,
            enable_tls: std::env::var("ENABLE_TLS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
            tls_cert_path: std::env::var("TLS_CERT_PATH").ok(),
            tls_key_path: std::env::var("TLS_KEY_PATH").ok(),
        };

        let rate_limit = RateLimitConfig {
            enabled: std::env::var("RATE_LIMIT_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            requests_per_minute: std::env::var("RATE_LIMIT_RPM")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,
            burst_size: std::env::var("RATE_LIMIT_BURST")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
        };

        let auth = AuthConfig {
            enabled: std::env::var("AUTH_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            api_keys: std::env::var("API_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            require_auth_for_health: std::env::var("AUTH_HEALTH_CHECK")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
        };

        let observability = ObservabilityConfig {
            enable_tracing: std::env::var("ENABLE_TRACING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            enable_metrics: std::env::var("ENABLE_METRICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
        };

        Ok(Config {
            server,
            rate_limit,
            auth,
            observability,
        })
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.server.timeout_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        std::env::remove_var("SERVER_ADDRESS");
        let config = Config::from_env().unwrap();
        assert_eq!(config.server.address, "0.0.0.0:8080");
    }
}
