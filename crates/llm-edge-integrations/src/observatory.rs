//! Observatory Integration Adapter
//!
//! Consumes telemetry stream definitions and structured event pipelines from
//! LLM-Observatory. This adapter pulls telemetry configuration from Observatory
//! without modifying Edge-Agent's existing monitoring infrastructure.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "observatory")]
use llm_observatory_core::{ObservatoryClient, TelemetryStream, EventPipeline, MetricDefinition, TraceDefinition};

/// Configuration for Observatory adapter
#[derive(Debug, Clone)]
pub struct ObservatoryConfig {
    /// Observatory service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable telemetry stream sync
    pub telemetry_sync_enabled: bool,
    /// Enable event pipeline sync
    pub event_pipeline_sync_enabled: bool,
    /// Telemetry sync interval (seconds)
    pub sync_interval: u64,
    /// Cache TTL for telemetry data (seconds)
    pub cache_ttl: u64,
}

impl Default for ObservatoryConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8084".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            telemetry_sync_enabled: true,
            event_pipeline_sync_enabled: true,
            sync_interval: 300,
            cache_ttl: 600,
        }
    }
}

impl ObservatoryConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("OBSERVATORY_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8084".to_string()),
            api_token: std::env::var("OBSERVATORY_API_TOKEN").ok(),
            timeout: std::env::var("OBSERVATORY_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            telemetry_sync_enabled: std::env::var("OBSERVATORY_TELEMETRY_SYNC_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            event_pipeline_sync_enabled: std::env::var("OBSERVATORY_EVENT_PIPELINE_SYNC_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            sync_interval: std::env::var("OBSERVATORY_SYNC_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            cache_ttl: std::env::var("OBSERVATORY_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
        }
    }
}

/// Observatory integration adapter
///
/// This adapter consumes telemetry stream definitions and event pipeline
/// configurations from the upstream Observatory service. It does not modify
/// Edge-Agent's monitoring behavior - it only provides telemetry metadata
/// for consumption and configuration discovery.
pub struct ObservatoryAdapter {
    #[cfg(feature = "observatory")]
    client: Arc<ObservatoryClient>,
    config: ObservatoryConfig,
}

impl ObservatoryAdapter {
    /// Create a new Observatory adapter
    #[cfg(feature = "observatory")]
    pub async fn new(config: &ObservatoryConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing Observatory adapter with endpoint: {}", config.endpoint);

        let client = ObservatoryClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to create Observatory client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("Observatory adapter initialized successfully");
                } else {
                    warn!("Observatory service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("Observatory health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume telemetry stream definitions from Observatory
    ///
    /// Queries Observatory for active telemetry stream configurations.
    /// Does not modify Edge-Agent telemetry - returns data for consumption.
    #[cfg(feature = "observatory")]
    pub async fn get_telemetry_streams(&self) -> Result<Vec<TelemetryStream>, crate::IntegrationError> {
        if !self.config.telemetry_sync_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching telemetry streams from Observatory");

        self.client
            .get_streams()
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch telemetry streams: {}", e)))
    }

    /// Consume telemetry stream by name
    ///
    /// Queries Observatory for a specific telemetry stream configuration.
    #[cfg(feature = "observatory")]
    pub async fn get_telemetry_stream(&self, stream_name: &str) -> Result<Option<TelemetryStream>, crate::IntegrationError> {
        debug!("Fetching telemetry stream: {}", stream_name);

        self.client
            .get_stream(stream_name)
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch telemetry stream: {}", e)))
    }

    /// Consume event pipeline configurations from Observatory
    ///
    /// Queries Observatory for structured event pipeline definitions.
    /// Returns pipeline configurations for informational purposes.
    #[cfg(feature = "observatory")]
    pub async fn get_event_pipelines(&self) -> Result<Vec<EventPipeline>, crate::IntegrationError> {
        if !self.config.event_pipeline_sync_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching event pipelines from Observatory");

        self.client
            .get_pipelines()
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch event pipelines: {}", e)))
    }

    /// Consume event pipeline by name
    ///
    /// Queries Observatory for a specific event pipeline configuration.
    #[cfg(feature = "observatory")]
    pub async fn get_event_pipeline(&self, pipeline_name: &str) -> Result<Option<EventPipeline>, crate::IntegrationError> {
        debug!("Fetching event pipeline: {}", pipeline_name);

        self.client
            .get_pipeline(pipeline_name)
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch event pipeline: {}", e)))
    }

    /// Consume metric definitions from Observatory
    ///
    /// Queries Observatory for standardized metric definitions that should
    /// be collected across the system.
    #[cfg(feature = "observatory")]
    pub async fn get_metric_definitions(&self) -> Result<Vec<MetricDefinition>, crate::IntegrationError> {
        debug!("Fetching metric definitions from Observatory");

        self.client
            .get_metrics()
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch metric definitions: {}", e)))
    }

    /// Consume trace definitions from Observatory
    ///
    /// Queries Observatory for standardized trace span definitions that should
    /// be emitted across the system.
    #[cfg(feature = "observatory")]
    pub async fn get_trace_definitions(&self) -> Result<Vec<TraceDefinition>, crate::IntegrationError> {
        debug!("Fetching trace definitions from Observatory");

        self.client
            .get_traces()
            .await
            .map_err(|e| crate::IntegrationError::Observatory(format!("Failed to fetch trace definitions: {}", e)))
    }

    /// Get all telemetry streams as a map
    ///
    /// Convenience method that returns streams indexed by stream name.
    #[cfg(feature = "observatory")]
    pub async fn get_telemetry_map(&self) -> Result<HashMap<String, TelemetryStream>, crate::IntegrationError> {
        let streams = self.get_telemetry_streams().await?;
        Ok(streams
            .into_iter()
            .map(|stream| (stream.name.clone(), stream))
            .collect())
    }

    /// Check if the Observatory adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "observatory")]
        {
            true
        }
        #[cfg(not(feature = "observatory"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "observatory"))]
impl ObservatoryAdapter {
    pub async fn new(_config: &ObservatoryConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::Observatory(
            "Observatory feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
