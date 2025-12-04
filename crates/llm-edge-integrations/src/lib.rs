//! # LLM Edge Integrations
//!
//! This crate provides thin consumption adapters for upstream LLM DevOps repositories.
//! All integrations are additive, backward-compatible, and do not modify existing
//! Edge-Agent logic.
//!
//! ## Integration Modules
//!
//! - `shield`: Consumes security filters, PII detection, policy-block events
//! - `sentinel`: Consumes anomaly flags, risk scores, runtime alerts
//! - `connector_hub`: Consumes provider routing definitions, backend adapter metadata
//! - `cost_ops`: Consumes cost calculations, token-cost projections, account limits
//! - `observatory`: Consumes telemetry stream definitions, structured event pipelines
//! - `policy_engine`: Consumes enforcement rules, policy validation results, routing permissions
//!
//! ## Design Principles
//!
//! 1. **Additive**: No modifications to existing proxy, routing, or interception logic
//! 2. **Backward-Compatible**: All integrations are optional via feature flags
//! 3. **No Circular Dependencies**: Only consumes from upstream, never exports to them
//! 4. **Thin Adapters**: Minimal logic, just data translation and caching
//! 5. **Observable**: All integration points emit telemetry

use std::sync::Arc;
use tracing::{info, warn};

// Re-export integration modules
pub mod shield;
pub mod sentinel;
pub mod connector_hub;
pub mod cost_ops;
pub mod observatory;
pub mod policy_engine;

/// Unified integration manager that coordinates all upstream consumption adapters
///
/// This struct provides a centralized interface for all upstream integrations.
/// Individual adapters can be enabled/disabled via feature flags at compile time.
#[derive(Clone)]
pub struct IntegrationManager {
    /// Shield integration (security filters, PII detection)
    #[cfg(feature = "shield")]
    pub shield: Option<Arc<shield::ShieldAdapter>>,

    /// Sentinel integration (anomaly detection, risk scores)
    #[cfg(feature = "sentinel")]
    pub sentinel: Option<Arc<sentinel::SentinelAdapter>>,

    /// Connector-Hub integration (provider routing, backend adapters)
    #[cfg(feature = "connector-hub")]
    pub connector_hub: Option<Arc<connector_hub::ConnectorHubAdapter>>,

    /// CostOps integration (cost calculations, token projections)
    #[cfg(feature = "cost-ops")]
    pub cost_ops: Option<Arc<cost_ops::CostOpsAdapter>>,

    /// Observatory integration (telemetry streams, event pipelines)
    #[cfg(feature = "observatory")]
    pub observatory: Option<Arc<observatory::ObservatoryAdapter>>,

    /// Policy-Engine integration (enforcement rules, validation results)
    #[cfg(feature = "policy-engine")]
    pub policy_engine: Option<Arc<policy_engine::PolicyEngineAdapter>>,
}

impl Default for IntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationManager {
    /// Create a new integration manager with all adapters uninitialized
    pub fn new() -> Self {
        info!("Creating new IntegrationManager");
        Self {
            #[cfg(feature = "shield")]
            shield: None,
            #[cfg(feature = "sentinel")]
            sentinel: None,
            #[cfg(feature = "connector-hub")]
            connector_hub: None,
            #[cfg(feature = "cost-ops")]
            cost_ops: None,
            #[cfg(feature = "observatory")]
            observatory: None,
            #[cfg(feature = "policy-engine")]
            policy_engine: None,
        }
    }

    /// Initialize all enabled integration adapters
    ///
    /// This method attempts to initialize all integration adapters that are enabled
    /// via feature flags. Failures are logged but non-fatal - the system can operate
    /// with partial integrations.
    pub async fn initialize(&mut self, config: &IntegrationConfig) -> Result<(), IntegrationError> {
        info!("Initializing upstream integrations");

        // Initialize Shield adapter
        #[cfg(feature = "shield")]
        {
            if config.shield_enabled {
                match shield::ShieldAdapter::new(&config.shield_config).await {
                    Ok(adapter) => {
                        info!("Shield integration initialized successfully");
                        self.shield = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize Shield integration: {}", e);
                    }
                }
            }
        }

        // Initialize Sentinel adapter
        #[cfg(feature = "sentinel")]
        {
            if config.sentinel_enabled {
                match sentinel::SentinelAdapter::new(&config.sentinel_config).await {
                    Ok(adapter) => {
                        info!("Sentinel integration initialized successfully");
                        self.sentinel = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize Sentinel integration: {}", e);
                    }
                }
            }
        }

        // Initialize Connector-Hub adapter
        #[cfg(feature = "connector-hub")]
        {
            if config.connector_hub_enabled {
                match connector_hub::ConnectorHubAdapter::new(&config.connector_hub_config).await {
                    Ok(adapter) => {
                        info!("Connector-Hub integration initialized successfully");
                        self.connector_hub = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize Connector-Hub integration: {}", e);
                    }
                }
            }
        }

        // Initialize CostOps adapter
        #[cfg(feature = "cost-ops")]
        {
            if config.cost_ops_enabled {
                match cost_ops::CostOpsAdapter::new(&config.cost_ops_config).await {
                    Ok(adapter) => {
                        info!("CostOps integration initialized successfully");
                        self.cost_ops = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize CostOps integration: {}", e);
                    }
                }
            }
        }

        // Initialize Observatory adapter
        #[cfg(feature = "observatory")]
        {
            if config.observatory_enabled {
                match observatory::ObservatoryAdapter::new(&config.observatory_config).await {
                    Ok(adapter) => {
                        info!("Observatory integration initialized successfully");
                        self.observatory = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize Observatory integration: {}", e);
                    }
                }
            }
        }

        // Initialize Policy-Engine adapter
        #[cfg(feature = "policy-engine")]
        {
            if config.policy_engine_enabled {
                match policy_engine::PolicyEngineAdapter::new(&config.policy_engine_config).await {
                    Ok(adapter) => {
                        info!("Policy-Engine integration initialized successfully");
                        self.policy_engine = Some(Arc::new(adapter));
                    }
                    Err(e) => {
                        warn!("Failed to initialize Policy-Engine integration: {}", e);
                    }
                }
            }
        }

        info!("Integration initialization complete");
        Ok(())
    }

    /// Check health status of all initialized integrations
    pub async fn health_check(&self) -> IntegrationHealth {
        IntegrationHealth {
            #[cfg(feature = "shield")]
            shield_healthy: self
                .shield
                .as_ref()
                .map(|s| s.health_check())
                .unwrap_or(false),
            #[cfg(feature = "sentinel")]
            sentinel_healthy: self
                .sentinel
                .as_ref()
                .map(|s| s.health_check())
                .unwrap_or(false),
            #[cfg(feature = "connector-hub")]
            connector_hub_healthy: self
                .connector_hub
                .as_ref()
                .map(|c| c.health_check())
                .unwrap_or(false),
            #[cfg(feature = "cost-ops")]
            cost_ops_healthy: self
                .cost_ops
                .as_ref()
                .map(|c| c.health_check())
                .unwrap_or(false),
            #[cfg(feature = "observatory")]
            observatory_healthy: self
                .observatory
                .as_ref()
                .map(|o| o.health_check())
                .unwrap_or(false),
            #[cfg(feature = "policy-engine")]
            policy_engine_healthy: self
                .policy_engine
                .as_ref()
                .map(|p| p.health_check())
                .unwrap_or(false),
        }
    }
}

/// Configuration for all integration adapters
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    pub shield_enabled: bool,
    pub shield_config: shield::ShieldConfig,
    pub sentinel_enabled: bool,
    pub sentinel_config: sentinel::SentinelConfig,
    pub connector_hub_enabled: bool,
    pub connector_hub_config: connector_hub::ConnectorHubConfig,
    pub cost_ops_enabled: bool,
    pub cost_ops_config: cost_ops::CostOpsConfig,
    pub observatory_enabled: bool,
    pub observatory_config: observatory::ObservatoryConfig,
    pub policy_engine_enabled: bool,
    pub policy_engine_config: policy_engine::PolicyEngineConfig,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            shield_enabled: false,
            shield_config: shield::ShieldConfig::default(),
            sentinel_enabled: false,
            sentinel_config: sentinel::SentinelConfig::default(),
            connector_hub_enabled: false,
            connector_hub_config: connector_hub::ConnectorHubConfig::default(),
            cost_ops_enabled: false,
            cost_ops_config: cost_ops::CostOpsConfig::default(),
            observatory_enabled: false,
            observatory_config: observatory::ObservatoryConfig::default(),
            policy_engine_enabled: false,
            policy_engine_config: policy_engine::PolicyEngineConfig::default(),
        }
    }
}

impl IntegrationConfig {
    /// Load integration configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            shield_enabled: std::env::var("SHIELD_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            shield_config: shield::ShieldConfig::from_env(),
            sentinel_enabled: std::env::var("SENTINEL_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            sentinel_config: sentinel::SentinelConfig::from_env(),
            connector_hub_enabled: std::env::var("CONNECTOR_HUB_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            connector_hub_config: connector_hub::ConnectorHubConfig::from_env(),
            cost_ops_enabled: std::env::var("COST_OPS_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            cost_ops_config: cost_ops::CostOpsConfig::from_env(),
            observatory_enabled: std::env::var("OBSERVATORY_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            observatory_config: observatory::ObservatoryConfig::from_env(),
            policy_engine_enabled: std::env::var("POLICY_ENGINE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            policy_engine_config: policy_engine::PolicyEngineConfig::from_env(),
        }
    }
}

/// Health status for all integrations
#[derive(Debug, Clone)]
pub struct IntegrationHealth {
    #[cfg(feature = "shield")]
    pub shield_healthy: bool,
    #[cfg(feature = "sentinel")]
    pub sentinel_healthy: bool,
    #[cfg(feature = "connector-hub")]
    pub connector_hub_healthy: bool,
    #[cfg(feature = "cost-ops")]
    pub cost_ops_healthy: bool,
    #[cfg(feature = "observatory")]
    pub observatory_healthy: bool,
    #[cfg(feature = "policy-engine")]
    pub policy_engine_healthy: bool,
}

/// Common error type for integration operations
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("Shield integration error: {0}")]
    Shield(String),
    #[error("Sentinel integration error: {0}")]
    Sentinel(String),
    #[error("Connector-Hub integration error: {0}")]
    ConnectorHub(String),
    #[error("CostOps integration error: {0}")]
    CostOps(String),
    #[error("Observatory integration error: {0}")]
    Observatory(String),
    #[error("Policy-Engine integration error: {0}")]
    PolicyEngine(String),
    #[error("General integration error: {0}")]
    General(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_manager_new() {
        let manager = IntegrationManager::new();
        // Verify it can be created without errors
        assert!(true, "IntegrationManager created successfully");
    }

    #[test]
    fn test_integration_config_default() {
        let config = IntegrationConfig::default();
        assert!(!config.shield_enabled);
        assert!(!config.sentinel_enabled);
        assert!(!config.connector_hub_enabled);
        assert!(!config.cost_ops_enabled);
        assert!(!config.observatory_enabled);
        assert!(!config.policy_engine_enabled);
    }
}
