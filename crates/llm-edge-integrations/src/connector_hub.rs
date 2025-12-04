//! Connector-Hub Integration Adapter
//!
//! Consumes provider routing definitions and backend adapter metadata from
//! LLM-Connector-Hub. This adapter pulls routing configuration from Connector-Hub
//! without modifying Edge-Agent's existing routing logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "connector-hub")]
use connector_hub_core::{ConnectorHubClient, ProviderRoute, BackendAdapter, AdapterMetadata};

/// Configuration for Connector-Hub adapter
#[derive(Debug, Clone)]
pub struct ConnectorHubConfig {
    /// Connector-Hub service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable provider routing sync
    pub routing_sync_enabled: bool,
    /// Enable adapter metadata sync
    pub adapter_metadata_sync_enabled: bool,
    /// Routing sync interval (seconds)
    pub sync_interval: u64,
    /// Cache TTL for routing data (seconds)
    pub cache_ttl: u64,
}

impl Default for ConnectorHubConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8082".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            routing_sync_enabled: true,
            adapter_metadata_sync_enabled: true,
            sync_interval: 300,
            cache_ttl: 600,
        }
    }
}

impl ConnectorHubConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("CONNECTOR_HUB_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8082".to_string()),
            api_token: std::env::var("CONNECTOR_HUB_API_TOKEN").ok(),
            timeout: std::env::var("CONNECTOR_HUB_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            routing_sync_enabled: std::env::var("CONNECTOR_HUB_ROUTING_SYNC_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            adapter_metadata_sync_enabled: std::env::var("CONNECTOR_HUB_ADAPTER_METADATA_SYNC_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            sync_interval: std::env::var("CONNECTOR_HUB_SYNC_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            cache_ttl: std::env::var("CONNECTOR_HUB_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
        }
    }
}

/// Connector-Hub integration adapter
///
/// This adapter consumes provider routing definitions and backend adapter
/// metadata from the upstream Connector-Hub service. It does not modify
/// Edge-Agent's routing behavior - it only provides routing data for consumption.
pub struct ConnectorHubAdapter {
    #[cfg(feature = "connector-hub")]
    client: Arc<ConnectorHubClient>,
    config: ConnectorHubConfig,
}

impl ConnectorHubAdapter {
    /// Create a new Connector-Hub adapter
    #[cfg(feature = "connector-hub")]
    pub async fn new(config: &ConnectorHubConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing Connector-Hub adapter with endpoint: {}", config.endpoint);

        let client = ConnectorHubClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::ConnectorHub(format!("Failed to create Connector-Hub client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("Connector-Hub adapter initialized successfully");
                } else {
                    warn!("Connector-Hub service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("Connector-Hub health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume provider routing definitions from Connector-Hub
    ///
    /// Queries Connector-Hub for current provider routing rules.
    /// Does not modify Edge-Agent routing - returns data for consumption.
    #[cfg(feature = "connector-hub")]
    pub async fn get_provider_routes(&self) -> Result<Vec<ProviderRoute>, crate::IntegrationError> {
        if !self.config.routing_sync_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching provider routes from Connector-Hub");

        self.client
            .get_routes()
            .await
            .map_err(|e| crate::IntegrationError::ConnectorHub(format!("Failed to fetch provider routes: {}", e)))
    }

    /// Consume provider route by provider name
    ///
    /// Queries Connector-Hub for routing configuration for a specific provider.
    #[cfg(feature = "connector-hub")]
    pub async fn get_provider_route(&self, provider: &str) -> Result<Option<ProviderRoute>, crate::IntegrationError> {
        debug!("Fetching provider route for: {}", provider);

        self.client
            .get_route(provider)
            .await
            .map_err(|e| crate::IntegrationError::ConnectorHub(format!("Failed to fetch provider route: {}", e)))
    }

    /// Consume backend adapter metadata from Connector-Hub
    ///
    /// Queries Connector-Hub for backend adapter capabilities and configuration.
    /// Returns metadata for informational and monitoring purposes.
    #[cfg(feature = "connector-hub")]
    pub async fn get_backend_adapters(&self) -> Result<Vec<BackendAdapter>, crate::IntegrationError> {
        if !self.config.adapter_metadata_sync_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching backend adapters from Connector-Hub");

        self.client
            .get_adapters()
            .await
            .map_err(|e| crate::IntegrationError::ConnectorHub(format!("Failed to fetch backend adapters: {}", e)))
    }

    /// Consume adapter metadata by adapter ID
    ///
    /// Queries Connector-Hub for detailed metadata about a specific adapter.
    #[cfg(feature = "connector-hub")]
    pub async fn get_adapter_metadata(&self, adapter_id: &str) -> Result<Option<AdapterMetadata>, crate::IntegrationError> {
        debug!("Fetching adapter metadata for: {}", adapter_id);

        self.client
            .get_adapter_metadata(adapter_id)
            .await
            .map_err(|e| crate::IntegrationError::ConnectorHub(format!("Failed to fetch adapter metadata: {}", e)))
    }

    /// Get all provider routing definitions as a map
    ///
    /// Convenience method that returns routes indexed by provider name.
    #[cfg(feature = "connector-hub")]
    pub async fn get_routing_map(&self) -> Result<HashMap<String, ProviderRoute>, crate::IntegrationError> {
        let routes = self.get_provider_routes().await?;
        Ok(routes
            .into_iter()
            .map(|route| (route.provider_name.clone(), route))
            .collect())
    }

    /// Check if the Connector-Hub adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "connector-hub")]
        {
            true
        }
        #[cfg(not(feature = "connector-hub"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "connector-hub"))]
impl ConnectorHubAdapter {
    pub async fn new(_config: &ConnectorHubConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::ConnectorHub(
            "Connector-Hub feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
