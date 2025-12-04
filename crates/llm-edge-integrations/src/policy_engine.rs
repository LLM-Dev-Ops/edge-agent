//! Policy-Engine Integration Adapter
//!
//! Consumes enforcement rules, policy validation results, and routing permissions
//! from LLM-Policy-Engine. This adapter pulls policy decisions from Policy-Engine
//! without modifying Edge-Agent's existing authorization logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(feature = "policy-engine")]
use llm_policy_engine::{PolicyEngineClient, EnforcementRule, PolicyValidationResult, RoutingPermission, PolicyDecision};

/// Configuration for Policy-Engine adapter
#[derive(Debug, Clone)]
pub struct PolicyEngineConfig {
    /// Policy-Engine service endpoint URL
    pub endpoint: String,
    /// API authentication token
    pub api_token: Option<String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Enable policy validation
    pub policy_validation_enabled: bool,
    /// Enable routing permission checks
    pub routing_permission_enabled: bool,
    /// Enable enforcement rule sync
    pub enforcement_rule_sync_enabled: bool,
    /// Policy sync interval (seconds)
    pub sync_interval: u64,
    /// Cache TTL for policy data (seconds)
    pub cache_ttl: u64,
}

impl Default for PolicyEngineConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8085".to_string(),
            api_token: None,
            timeout: Duration::from_secs(5),
            policy_validation_enabled: true,
            routing_permission_enabled: true,
            enforcement_rule_sync_enabled: true,
            sync_interval: 300,
            cache_ttl: 600,
        }
    }
}

impl PolicyEngineConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            endpoint: std::env::var("POLICY_ENGINE_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8085".to_string()),
            api_token: std::env::var("POLICY_ENGINE_API_TOKEN").ok(),
            timeout: std::env::var("POLICY_ENGINE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5)),
            policy_validation_enabled: std::env::var("POLICY_ENGINE_POLICY_VALIDATION_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            routing_permission_enabled: std::env::var("POLICY_ENGINE_ROUTING_PERMISSION_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enforcement_rule_sync_enabled: std::env::var("POLICY_ENGINE_ENFORCEMENT_RULE_SYNC_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            sync_interval: std::env::var("POLICY_ENGINE_SYNC_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            cache_ttl: std::env::var("POLICY_ENGINE_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
        }
    }
}

/// Policy-Engine integration adapter
///
/// This adapter consumes enforcement rules, policy validation results, and
/// routing permissions from the upstream Policy-Engine service. It does not
/// modify Edge-Agent's authorization behavior - it only provides policy data
/// for consumption and decision support.
pub struct PolicyEngineAdapter {
    #[cfg(feature = "policy-engine")]
    client: Arc<PolicyEngineClient>,
    config: PolicyEngineConfig,
}

impl PolicyEngineAdapter {
    /// Create a new Policy-Engine adapter
    #[cfg(feature = "policy-engine")]
    pub async fn new(config: &PolicyEngineConfig) -> Result<Self, crate::IntegrationError> {
        info!("Initializing Policy-Engine adapter with endpoint: {}", config.endpoint);

        let client = PolicyEngineClient::builder()
            .endpoint(&config.endpoint)
            .timeout(config.timeout)
            .api_token(config.api_token.clone())
            .build()
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Failed to create Policy-Engine client: {}", e)))?;

        // Test connectivity
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("Policy-Engine adapter initialized successfully");
                } else {
                    warn!("Policy-Engine service is unhealthy but adapter will continue");
                }
            }
            Err(e) => {
                warn!("Policy-Engine health check failed: {}. Adapter will retry on first use.", e);
            }
        }

        Ok(Self {
            client: Arc::new(client),
            config: config.clone(),
        })
    }

    /// Consume enforcement rules from Policy-Engine
    ///
    /// Queries Policy-Engine for active enforcement rules that define
    /// security and governance policies. Does not enforce rules - returns
    /// data for consumption.
    #[cfg(feature = "policy-engine")]
    pub async fn get_enforcement_rules(&self) -> Result<Vec<EnforcementRule>, crate::IntegrationError> {
        if !self.config.enforcement_rule_sync_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching enforcement rules from Policy-Engine");

        self.client
            .get_rules()
            .await
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Failed to fetch enforcement rules: {}", e)))
    }

    /// Consume enforcement rule by ID
    ///
    /// Queries Policy-Engine for a specific enforcement rule.
    #[cfg(feature = "policy-engine")]
    pub async fn get_enforcement_rule(&self, rule_id: &str) -> Result<Option<EnforcementRule>, crate::IntegrationError> {
        debug!("Fetching enforcement rule: {}", rule_id);

        self.client
            .get_rule(rule_id)
            .await
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Failed to fetch enforcement rule: {}", e)))
    }

    /// Consume policy validation result
    ///
    /// Queries Policy-Engine to validate a request against defined policies.
    /// Returns validation result without enforcing any decisions.
    #[cfg(feature = "policy-engine")]
    pub async fn validate_request(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        context: HashMap<String, String>,
    ) -> Result<PolicyValidationResult, crate::IntegrationError> {
        if !self.config.policy_validation_enabled {
            return Ok(PolicyValidationResult::default());
        }

        debug!(
            "Validating request: user={}, resource={}, action={}",
            user_id, resource, action
        );

        self.client
            .validate(user_id, resource, action, context)
            .await
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Policy validation failed: {}", e)))
    }

    /// Consume routing permissions from Policy-Engine
    ///
    /// Queries Policy-Engine for routing permissions that define which
    /// providers and models a user or account is allowed to access.
    #[cfg(feature = "policy-engine")]
    pub async fn get_routing_permissions(&self, user_id: &str) -> Result<Vec<RoutingPermission>, crate::IntegrationError> {
        if !self.config.routing_permission_enabled {
            return Ok(Vec::new());
        }

        debug!("Fetching routing permissions for user: {}", user_id);

        self.client
            .get_permissions(user_id)
            .await
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Failed to fetch routing permissions: {}", e)))
    }

    /// Consume policy decision for a specific action
    ///
    /// Queries Policy-Engine for a policy decision on whether an action
    /// should be allowed or denied based on all applicable rules.
    #[cfg(feature = "policy-engine")]
    pub async fn get_policy_decision(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
    ) -> Result<PolicyDecision, crate::IntegrationError> {
        debug!(
            "Getting policy decision: user={}, resource={}, action={}",
            user_id, resource, action
        );

        self.client
            .evaluate_policy(user_id, resource, action)
            .await
            .map_err(|e| crate::IntegrationError::PolicyEngine(format!("Policy decision failed: {}", e)))
    }

    /// Check if user is permitted to route to a specific provider
    ///
    /// Helper method to check routing permissions for a provider.
    #[cfg(feature = "policy-engine")]
    pub async fn is_provider_permitted(&self, user_id: &str, provider: &str) -> Result<bool, crate::IntegrationError> {
        let permissions = self.get_routing_permissions(user_id).await?;
        Ok(permissions.iter().any(|p| p.provider == provider && p.permitted))
    }

    /// Get all enforcement rules as a map
    ///
    /// Convenience method that returns rules indexed by rule ID.
    #[cfg(feature = "policy-engine")]
    pub async fn get_rules_map(&self) -> Result<HashMap<String, EnforcementRule>, crate::IntegrationError> {
        let rules = self.get_enforcement_rules().await?;
        Ok(rules
            .into_iter()
            .map(|rule| (rule.id.clone(), rule))
            .collect())
    }

    /// Check if the Policy-Engine adapter is healthy
    pub fn health_check(&self) -> bool {
        #[cfg(feature = "policy-engine")]
        {
            true
        }
        #[cfg(not(feature = "policy-engine"))]
        {
            false
        }
    }
}

// Stub implementations when feature is disabled
#[cfg(not(feature = "policy-engine"))]
impl PolicyEngineAdapter {
    pub async fn new(_config: &PolicyEngineConfig) -> Result<Self, crate::IntegrationError> {
        Err(crate::IntegrationError::PolicyEngine(
            "Policy-Engine feature not enabled".to_string(),
        ))
    }

    pub fn health_check(&self) -> bool {
        false
    }
}
