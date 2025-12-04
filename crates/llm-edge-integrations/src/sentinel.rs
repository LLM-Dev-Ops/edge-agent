//! Sentinel Integration Adapter
//!
//! Consumes anomaly flags, risk scores, and runtime alerts from LLM-Sentinel.
//! This adapter pulls anomaly detection data from Sentinel without modifying
//! Edge-Agent's existing monitoring logic.

use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "sentinel")]
use llm_sentinel::{SentinelClient, AnomalyFlag, RiskScore, RuntimeAlert};

/// Configuration for Sentinel adapter
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// Sentinel service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable anomaly detection
    pub anomaly_detection_enabled: bool,
    /// Enable risk scoring
    pub risk_scoring_enabled: bool,
    /// Risk score threshold for alerts (0.0 - 1.0)
    pub risk_threshold: f64,
    /// Alert polling interval (seconds)
    pub alert_poll_interval: u64,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8081".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            anomaly_detection_enabled: true,
            risk_scoring_enabled: true,
            risk_threshold: 0.7,
            alert_poll_interval: 60,
        }
    }
}

impl SentinelConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("SENTINEL_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            api_token: std::env::var("SENTINEL_API_TOKEN").ok(),
            timeout: std::env::var("SENTINEL_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            anomaly_detection_enabled: std::env::var("SENTINEL_ANOMALY_DETECTION_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            risk_scoring_enabled: std::env::var("SENTINEL_RISK_SCORING_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            risk_threshold: std::env::var("SENTINEL_RISK_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.7),
            alert_poll_interval: std::env::var("SENTINEL_ALERT_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }
}

/// Sentinel integration adapter
///
/// This adapter consumes anomaly detection and risk scoring data from
/// the upstream Sentinel service. It does not modify Edge-Agent's monitoring
/// behavior - it only provides anomaly data for consumption.
pub struct SentinelAdapter {
    #[cfg(feature = "sentinel")]
    client: Arc<SentinelClient>,
    config: SentinelConfig,
}

impl SentinelAdapter {
    /// Create a new Sentinel adapter
    #[cfg(feature = "sentinel")]
    pub async fn new(config: &SentinelConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing Sentinel adapter with endpoint: {}", config.endpoint);

        let client = SentinelClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::Sentinel(format!("Failed to create Sentinel client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("Sentinel adapter initialized successfully");
                } else {
                    warn!("Sentinel service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("Sentinel health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume anomaly flags from Sentinel
    ///
    /// Queries Sentinel for active anomaly flags that indicate unusual
    /// behavior patterns. Does not modify Edge-Agent behavior - returns data for consumption.
    #[cfg(feature = "sentinel")]
    pub async fn get_anomaly_flags(&self) -> Result<Vec<AnomalyFlag>, crate::IntegrationError> {
        if !self.config.anomaly_detection_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching anomaly flags from Sentinel");

        self.client
            .get_active_anomalies()
            .await
            .map_err(|e| crate::IntegrationError::Sentinel(format!("Failed to fetch anomaly flags: {}", e)))
    }

    /// Consume risk score from Sentinel
    ///
    /// Calculates a risk score for a given request or user based on Sentinel's
    /// ML models. Returns the score without enforcing any policies.
    #[cfg(feature = "sentinel")]
    pub async fn calculate_risk_score(&self, request_id: &str, user_id: Option<&str>) -> Result<RiskScore, crate::IntegrationError> {
        if !self.config.risk_scoring_enabled {
            return Ok(RiskScore::default());
        }

        debug!("Calculating risk score for request: {}", request_id);

        self.client
            .calculate_risk(request_id, user_id)
            .await
            .map_err(|e| crate::IntegrationError::Sentinel(format!("Risk score calculation failed: {}", e)))
    }

    /// Consume runtime alerts from Sentinel
    ///
    /// Queries Sentinel for recent runtime alerts that indicate security
    /// or performance issues. Returns alerts for monitoring and observability.
    #[cfg(feature = "sentinel")]
    pub async fn get_runtime_alerts(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<RuntimeAlert>, crate::IntegrationError> {
        debug!("Fetching runtime alerts from Sentinel");

        self.client
            .get_alerts(since)
            .await
            .map_err(|e| crate::IntegrationError::Sentinel(format!("Failed to fetch runtime alerts: {}", e)))
    }

    /// Check if a risk score exceeds the configured threshold
    #[cfg(feature = "sentinel")]
    pub fn is_high_risk(&self, risk_score: &RiskScore) -> bool {
        risk_score.score >= self.config.risk_threshold
    }

    /// Check if the Sentinel adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "sentinel")]
        {
            true
        }
        #[cfg(not(feature = "sentinel"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "sentinel"))]
impl SentinelAdapter {
    pub async fn new(_config: &SentinelConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::Sentinel(
            "Sentinel feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
