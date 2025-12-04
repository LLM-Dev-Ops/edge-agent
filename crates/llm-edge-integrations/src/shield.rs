//! Shield Integration Adapter
//!
//! Consumes security filters, PII detection, and policy-block events from LLM-Shield.
//! This adapter pulls security decisions from Shield without modifying Edge-Agent's
//! existing request processing logic.

use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "shield")]
use llm_shield_sdk::{ShieldClient, SecurityFilter, PiiDetectionResult, PolicyBlockEvent};

/// Configuration for Shield adapter
#[derive(Debug, Clone)]
pub struct ShieldConfig {
    /// Shield service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable PII detection
    pub pii_detection_enabled: bool,
    /// Enable policy blocking
    pub policy_blocking_enabled: bool,
    /// Cache TTL for security filters (seconds)
    pub cache_ttl: u64,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            pii_detection_enabled: true,
            policy_blocking_enabled: true,
            cache_ttl: 300,
        }
    }
}

impl ShieldConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("SHIELD_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_token: std::env::var("SHIELD_API_TOKEN").ok(),
            timeout: std::env::var("SHIELD_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            pii_detection_enabled: std::env::var("SHIELD_PII_DETECTION_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            policy_blocking_enabled: std::env::var("SHIELD_POLICY_BLOCKING_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            cache_ttl: std::env::var("SHIELD_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        }
    }
}

/// Shield integration adapter
///
/// This adapter consumes security decisions from the upstream Shield service.
/// It does not modify Edge-Agent's request processing - it only provides
/// security data that can be used by other components.
pub struct ShieldAdapter {
    #[cfg(feature = "shield")]
    client: Arc<ShieldClient>,
    config: ShieldConfig,
}

impl ShieldAdapter {
    /// Create a new Shield adapter
    #[cfg(feature = "shield")]
    pub async fn new(config: &ShieldConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing Shield adapter with endpoint: {}", config.endpoint);

        let client = ShieldClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::Shield(format!("Failed to create Shield client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("Shield adapter initialized successfully");
                } else {
                    warn!("Shield service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("Shield health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume security filters from Shield
    ///
    /// Queries Shield for active security filters that should be applied
    /// to requests. Does not modify Edge-Agent behavior - returns data for consumption.
    #[cfg(feature = "shield")]
    pub async fn get_security_filters(&self) -> Result<Vec<SecurityFilter>, crate::IntegrationError> {
        debug!("Fetching security filters from Shield");

        self.client
            .get_active_filters()
            .await
            .map_err(|e| crate::IntegrationError::Shield(format!("Failed to fetch security filters: {}", e)))
    }

    /// Consume PII detection results from Shield
    ///
    /// Analyzes text content for PII using Shield's detection engine.
    /// Returns detection results without modifying the original content.
    #[cfg(feature = "shield")]
    pub async fn detect_pii(&self, content: &str) -> Result<PiiDetectionResult, crate::IntegrationError> {
        if !self.config.pii_detection_enabled {
            return Ok(PiiDetectionResult::default());
        }

        debug!("Requesting PII detection from Shield");

        self.client
            .detect_pii(content)
            .await
            .map_err(|e| crate::IntegrationError::Shield(format!("PII detection failed: {}", e)))
    }

    /// Consume policy block events from Shield
    ///
    /// Queries Shield for recent policy violation events.
    /// Useful for auditing and monitoring, does not enforce blocks in Edge-Agent.
    #[cfg(feature = "shield")]
    pub async fn get_policy_block_events(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<PolicyBlockEvent>, crate::IntegrationError> {
        debug!("Fetching policy block events from Shield");

        self.client
            .get_block_events(since)
            .await
            .map_err(|e| crate::IntegrationError::Shield(format!("Failed to fetch policy block events: {}", e)))
    }

    /// Check if the Shield adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "shield")]
        {
            // Synchronous health check - actual health is checked on initialization
            // and periodically by the integration manager
            true
        }
        #[cfg(not(feature = "shield"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "shield"))]
impl ShieldAdapter {
    pub async fn new(_config: &ShieldConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::Shield(
            "Shield feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
