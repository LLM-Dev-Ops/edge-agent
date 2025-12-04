//! CostOps Integration Adapter
//!
//! Consumes cost calculations, token-cost projections, and account-level limits
//! from LLM-CostOps. This adapter pulls cost data from CostOps without modifying
//! Edge-Agent's existing request processing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "cost-ops")]
use llm_cost_ops::{CostOpsClient, CostCalculation, TokenCostProjection, AccountLimit, UsageReport};

/// Configuration for CostOps adapter
#[derive(Debug, Clone)]
pub struct CostOpsConfig {
    /// CostOps service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable cost tracking
    pub cost_tracking_enabled: bool,
    /// Enable token projection
    pub token_projection_enabled: bool,
    /// Enable account limits sync
    pub account_limits_enabled: bool,
    /// Cost data cache TTL (seconds)
    pub cache_ttl: u64,
}

impl Default for CostOpsConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8083".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            cost_tracking_enabled: true,
            token_projection_enabled: true,
            account_limits_enabled: true,
            cache_ttl: 300,
        }
    }
}

impl CostOpsConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("COST_OPS_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8083".to_string()),
            api_token: std::env::var("COST_OPS_API_TOKEN").ok(),
            timeout: std::env::var("COST_OPS_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            cost_tracking_enabled: std::env::var("COST_OPS_COST_TRACKING_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            token_projection_enabled: std::env::var("COST_OPS_TOKEN_PROJECTION_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            account_limits_enabled: std::env::var("COST_OPS_ACCOUNT_LIMITS_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            cache_ttl: std::env::var("COST_OPS_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        }
    }
}

/// CostOps integration adapter
///
/// This adapter consumes cost calculations, token projections, and account limits
/// from the upstream CostOps service. It does not modify Edge-Agent's request
/// processing - it only provides cost data for consumption and observability.
pub struct CostOpsAdapter {
    #[cfg(feature = "cost-ops")]
    client: Arc<CostOpsClient>,
    config: CostOpsConfig,
}

impl CostOpsAdapter {
    /// Create a new CostOps adapter
    #[cfg(feature = "cost-ops")]
    pub async fn new(config: &CostOpsConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing CostOps adapter with endpoint: {}", config.endpoint);

        let client = CostOpsClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::CostOps(format!("Failed to create CostOps client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("CostOps adapter initialized successfully");
                } else {
                    warn!("CostOps service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("CostOps health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume cost calculation for a request
    ///
    /// Queries CostOps for the calculated cost of a given request based on
    /// token usage, provider pricing, and other factors. Does not enforce
    /// any cost limits - returns data for consumption.
    #[cfg(feature = "cost-ops")]
    pub async fn calculate_request_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Result<CostCalculation, crate::IntegrationError> {
        if !self.config.cost_tracking_enabled {
            return Ok(CostCalculation::default());
        }

        debug!(
            "Calculating cost for provider: {}, model: {}, input_tokens: {}, output_tokens: {}",
            provider, model, input_tokens, output_tokens
        );

        self.client
            .calculate_cost(provider, model, input_tokens, output_tokens)
            .await
            .map_err(|e| crate::IntegrationError::CostOps(format!("Cost calculation failed: {}", e)))
    }

    /// Consume token cost projection
    ///
    /// Queries CostOps for projected costs based on estimated token usage.
    /// Useful for budgeting and cost estimation before making requests.
    #[cfg(feature = "cost-ops")]
    pub async fn project_token_cost(
        &self,
        provider: &str,
        model: &str,
        estimated_tokens: u64,
    ) -> Result<TokenCostProjection, crate::IntegrationError> {
        if !self.config.token_projection_enabled {
            return Ok(TokenCostProjection::default());
        }

        debug!(
            "Projecting token cost for provider: {}, model: {}, estimated_tokens: {}",
            provider, model, estimated_tokens
        );

        self.client
            .project_cost(provider, model, estimated_tokens)
            .await
            .map_err(|e| crate::IntegrationError::CostOps(format!("Token cost projection failed: {}", e)))
    }

    /// Consume account limits from CostOps
    ///
    /// Queries CostOps for account-level spending limits and current usage.
    /// Returns limit data for monitoring and alerting purposes.
    #[cfg(feature = "cost-ops")]
    pub async fn get_account_limits(&self, account_id: &str) -> Result<AccountLimit, crate::IntegrationError> {
        if !self.config.account_limits_enabled {
            return Ok(AccountLimit::default());
        }

        debug!("Fetching account limits for: {}", account_id);

        self.client
            .get_limits(account_id)
            .await
            .map_err(|e| crate::IntegrationError::CostOps(format!("Failed to fetch account limits: {}", e)))
    }

    /// Consume usage report from CostOps
    ///
    /// Queries CostOps for detailed usage and cost breakdown for an account
    /// over a specified time period.
    #[cfg(feature = "cost-ops")]
    pub async fn get_usage_report(
        &self,
        account_id: &str,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<UsageReport, crate::IntegrationError> {
        debug!(
            "Fetching usage report for account: {} from {} to {}",
            account_id, start_time, end_time
        );

        self.client
            .get_usage_report(account_id, start_time, end_time)
            .await
            .map_err(|e| crate::IntegrationError::CostOps(format!("Failed to fetch usage report: {}", e)))
    }

    /// Check if account is approaching limit
    ///
    /// Helper method to determine if an account is close to its spending limit.
    #[cfg(feature = "cost-ops")]
    pub fn is_approaching_limit(&self, limit: &AccountLimit, threshold_percent: f64) -> bool {
        if limit.limit == 0.0 {
            return false;
        }
        let usage_percent = (limit.current_usage / limit.limit) * 100.0;
        usage_percent >= threshold_percent
    }

    /// Check if the CostOps adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "cost-ops")]
        {
            true
        }
        #[cfg(not(feature = "cost-ops"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "cost-ops"))]
impl CostOpsAdapter {
    pub async fn new(_config: &CostOpsConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::CostOps(
            "CostOps feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
