//! Integration module - orchestrates all system components
//!
//! This module provides the central integration point that wires together:
//! - Cache Manager (L1 + L2)
//! - Routing Engine
//! - Provider Adapters (OpenAI, Anthropic)
//! - Observability (Metrics, Tracing, Logging)
//! - Security (Auth, PII detection)

use llm_edge_cache::{l2::L2Config, CacheManager};
use llm_edge_providers::{anthropic::AnthropicAdapter, openai::OpenAIAdapter, LLMProvider};
use std::sync::Arc;
use tracing::{info, warn};

/// Application state shared across all request handlers
///
/// This state is cloned for each request (using Arc) and contains
/// all the components needed to process LLM requests.
#[derive(Clone)]
pub struct AppState {
    /// Multi-tier cache manager (L1 + optional L2)
    pub cache_manager: Arc<CacheManager>,

    /// OpenAI provider (optional)
    pub openai_provider: Option<Arc<dyn LLMProvider>>,

    /// Anthropic provider (optional)
    pub anthropic_provider: Option<Arc<dyn LLMProvider>>,

    /// Application configuration
    pub config: Arc<AppConfig>,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Server host address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Enable L2 cache (Redis)
    pub enable_l2_cache: bool,

    /// Redis connection URL
    pub redis_url: Option<String>,

    /// OpenAI API key
    pub openai_api_key: Option<String>,

    /// Anthropic API key
    pub anthropic_api_key: Option<String>,

    /// Enable request tracing
    pub enable_tracing: bool,

    /// Enable metrics export
    pub enable_metrics: bool,

    /// Metrics port
    pub metrics_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_l2_cache: false,
            redis_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            enable_tracing: true,
            enable_metrics: true,
            metrics_port: 9090,
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            enable_l2_cache: std::env::var("ENABLE_L2_CACHE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            redis_url: std::env::var("REDIS_URL").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            enable_tracing: std::env::var("ENABLE_TRACING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_metrics: std::env::var("ENABLE_METRICS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            metrics_port: std::env::var("METRICS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9090),
        }
    }
}

/// Initialize the application state
///
/// This function:
/// 1. Creates the cache manager (L1 + optional L2)
/// 2. Initializes provider adapters
/// 3. Sets up observability
/// 4. Returns the complete application state
pub async fn initialize_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    info!("Initializing LLM Edge Agent application state");

    // Step 1: Initialize cache manager
    info!("Initializing cache layer");
    let cache_manager = if config.enable_l2_cache {
        if let Some(ref redis_url) = config.redis_url {
            info!("L2 cache enabled with Redis: {}", redis_url);
            let l2_config = L2Config {
                redis_url: redis_url.clone(),
                ttl_seconds: 3600, // 1 hour default
                connection_timeout_ms: 1000,
                operation_timeout_ms: 100,
                key_prefix: "llm-edge:".to_string(),
            };
            Arc::new(CacheManager::with_l2(l2_config).await)
        } else {
            warn!("L2 cache enabled but no Redis URL provided, using L1 only");
            Arc::new(CacheManager::new())
        }
    } else {
        info!("Using L1 cache only (in-memory)");
        Arc::new(CacheManager::new())
    };

    // Step 2: Initialize provider adapters
    info!("Initializing provider adapters");

    let openai_provider: Option<Arc<dyn LLMProvider>> =
        if let Some(ref api_key) = config.openai_api_key {
            info!("Initializing OpenAI provider");
            Some(Arc::new(OpenAIAdapter::new(api_key.clone())))
        } else {
            warn!("OpenAI API key not provided, OpenAI provider will not be available");
            None
        };

    let anthropic_provider: Option<Arc<dyn LLMProvider>> =
        if let Some(ref api_key) = config.anthropic_api_key {
            info!("Initializing Anthropic provider");
            Some(Arc::new(AnthropicAdapter::new(api_key.clone())))
        } else {
            warn!("Anthropic API key not provided, Anthropic provider will not be available");
            None
        };

    // Verify at least one provider is available
    if openai_provider.is_none() && anthropic_provider.is_none() {
        return Err(anyhow::anyhow!(
            "No LLM providers configured. Please set OPENAI_API_KEY or ANTHROPIC_API_KEY"
        ));
    }

    // Step 3: Build application state
    let app_state = AppState {
        cache_manager,
        openai_provider,
        anthropic_provider,
        config: Arc::new(config),
    };

    info!("Application state initialized successfully");
    Ok(app_state)
}

/// Health check for all system components
pub async fn check_system_health(state: &AppState) -> SystemHealthStatus {
    let cache_health = state.cache_manager.health_check().await;

    let openai_healthy = if let Some(ref provider) = state.openai_provider {
        matches!(
            provider.health().await,
            llm_edge_providers::adapter::HealthStatus::Healthy
        )
    } else {
        false
    };

    let anthropic_healthy = if let Some(ref provider) = state.anthropic_provider {
        matches!(
            provider.health().await,
            llm_edge_providers::adapter::HealthStatus::Healthy
        )
    } else {
        false
    };

    SystemHealthStatus {
        cache_l1_healthy: cache_health.l1_healthy,
        cache_l2_healthy: cache_health.l2_healthy,
        cache_l2_configured: cache_health.l2_configured,
        openai_healthy,
        openai_configured: state.openai_provider.is_some(),
        anthropic_healthy,
        anthropic_configured: state.anthropic_provider.is_some(),
    }
}

/// Overall system health status
#[derive(Debug, Clone)]
pub struct SystemHealthStatus {
    pub cache_l1_healthy: bool,
    pub cache_l2_healthy: bool,
    pub cache_l2_configured: bool,
    pub openai_healthy: bool,
    pub openai_configured: bool,
    pub anthropic_healthy: bool,
    pub anthropic_configured: bool,
}

impl SystemHealthStatus {
    pub fn is_healthy(&self) -> bool {
        // System is healthy if:
        // 1. L1 cache is healthy (always should be)
        // 2. L2 cache is healthy (if configured)
        // 3. At least one provider is healthy
        let cache_healthy =
            self.cache_l1_healthy && (!self.cache_l2_configured || self.cache_l2_healthy);

        let provider_healthy = self.openai_healthy || self.anthropic_healthy;

        cache_healthy && provider_healthy
    }

    pub fn status_string(&self) -> String {
        if self.is_healthy() {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(!config.enable_l2_cache);
    }

    #[test]
    fn test_system_health_all_healthy() {
        let status = SystemHealthStatus {
            cache_l1_healthy: true,
            cache_l2_healthy: true,
            cache_l2_configured: true,
            openai_healthy: true,
            openai_configured: true,
            anthropic_healthy: false,
            anthropic_configured: false,
        };

        assert!(status.is_healthy());
        assert_eq!(status.status_string(), "healthy");
    }

    #[test]
    fn test_system_health_degraded() {
        let status = SystemHealthStatus {
            cache_l1_healthy: true,
            cache_l2_healthy: false,
            cache_l2_configured: true,
            openai_healthy: false,
            openai_configured: true,
            anthropic_healthy: false,
            anthropic_configured: false,
        };

        assert!(!status.is_healthy());
        assert_eq!(status.status_string(), "degraded");
    }

    #[test]
    fn test_system_health_l2_not_configured() {
        let status = SystemHealthStatus {
            cache_l1_healthy: true,
            cache_l2_healthy: false,
            cache_l2_configured: false, // L2 not configured, so its health doesn't matter
            openai_healthy: true,
            openai_configured: true,
            anthropic_healthy: false,
            anthropic_configured: false,
        };

        assert!(status.is_healthy());
    }
}
