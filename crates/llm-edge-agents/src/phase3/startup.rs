//! Phase 3 Startup Hardening
//!
//! Enforces startup requirements for Phase 3 Layer 1 agents.
//! Service MUST crashloop on misconfiguration.

use std::time::Duration;
use tracing::{error, info, warn};

/// Phase 3 configuration
#[derive(Debug, Clone)]
pub struct Phase3Config {
    /// Agent phase (MUST be "phase3")
    pub agent_phase: String,
    /// Agent layer (MUST be "layer1")
    pub agent_layer: String,
    /// RuVector service URL (REQUIRED)
    pub ruvector_url: String,
    /// RuVector API key (REQUIRED from Secret Manager)
    pub ruvector_api_key: String,
    /// Performance budgets
    pub max_tokens: u32,
    pub max_latency_ms: u32,
    pub max_calls_per_run: u32,
    /// Startup timeout
    pub startup_timeout: Duration,
}

impl Phase3Config {
    /// Load configuration from environment with validation
    pub fn from_env() -> Result<Self, StartupError> {
        let config = Self {
            agent_phase: std::env::var("AGENT_PHASE")
                .unwrap_or_else(|_| String::new()),
            agent_layer: std::env::var("AGENT_LAYER")
                .unwrap_or_else(|_| String::new()),
            ruvector_url: std::env::var("RUVECTOR_SERVICE_URL")
                .or_else(|_| std::env::var("RUVECTOR_ENDPOINT"))
                .unwrap_or_else(|_| String::new()),
            ruvector_api_key: std::env::var("RUVECTOR_API_KEY")
                .unwrap_or_else(|_| String::new()),
            max_tokens: std::env::var("MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1500),
            max_latency_ms: std::env::var("MAX_LATENCY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            max_calls_per_run: std::env::var("MAX_CALLS_PER_RUN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            startup_timeout: Duration::from_secs(
                std::env::var("STARTUP_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30)
            ),
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration - HARD FAIL on invalid
    fn validate(&self) -> Result<(), StartupError> {
        // AGENT_PHASE must be "phase3"
        if self.agent_phase != "phase3" {
            return Err(StartupError::InvalidPhase {
                expected: "phase3".to_string(),
                actual: self.agent_phase.clone(),
            });
        }

        // AGENT_LAYER must be "layer1"
        if self.agent_layer != "layer1" {
            return Err(StartupError::InvalidLayer {
                expected: "layer1".to_string(),
                actual: self.agent_layer.clone(),
            });
        }

        // RUVECTOR_SERVICE_URL is REQUIRED
        if self.ruvector_url.is_empty() {
            return Err(StartupError::MissingRequired("RUVECTOR_SERVICE_URL"));
        }

        // RUVECTOR_API_KEY is REQUIRED (from Secret Manager)
        if self.ruvector_api_key.is_empty() {
            return Err(StartupError::MissingRequired("RUVECTOR_API_KEY"));
        }

        Ok(())
    }
}

/// Startup errors that cause crashloop
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Invalid AGENT_PHASE: expected '{expected}', got '{actual}'")]
    InvalidPhase { expected: String, actual: String },

    #[error("Invalid AGENT_LAYER: expected '{expected}', got '{actual}'")]
    InvalidLayer { expected: String, actual: String },

    #[error("Missing required environment variable: {0}")]
    MissingRequired(&'static str),

    #[error("RuVector service unavailable: {0}")]
    RuVectorUnavailable(String),

    #[error("Startup timeout exceeded")]
    StartupTimeout,

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

/// Phase 3 startup guard - validates all requirements before service start
pub struct Phase3StartupGuard {
    config: Phase3Config,
}

impl Phase3StartupGuard {
    /// Create a new startup guard from environment
    /// WILL PANIC (crashloop) on misconfiguration
    pub fn new() -> Result<Self, StartupError> {
        info!("Phase 3 Startup Guard initializing...");

        let config = Phase3Config::from_env().map_err(|e| {
            error!("FATAL: Phase 3 startup validation failed: {}", e);
            error!("Service will crashloop until configuration is fixed");
            e
        })?;

        info!(
            phase = %config.agent_phase,
            layer = %config.agent_layer,
            ruvector_url = %config.ruvector_url,
            max_tokens = config.max_tokens,
            max_latency_ms = config.max_latency_ms,
            max_calls_per_run = config.max_calls_per_run,
            "Phase 3 configuration validated"
        );

        Ok(Self { config })
    }

    /// Validate RuVector connectivity (HARD FAIL if unavailable)
    pub async fn validate_ruvector(&self) -> Result<(), StartupError> {
        info!("Validating RuVector service connectivity...");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| StartupError::RuVectorUnavailable(e.to_string()))?;

        let health_url = format!("{}/health", self.config.ruvector_url);

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("RuVector service is available and healthy");
                Ok(())
            }
            Ok(response) => {
                let status = response.status();
                error!("RuVector health check failed with status: {}", status);
                Err(StartupError::RuVectorUnavailable(
                    format!("Health check returned status {}", status)
                ))
            }
            Err(e) => {
                error!("RuVector service unreachable: {}", e);
                Err(StartupError::RuVectorUnavailable(e.to_string()))
            }
        }
    }

    /// Get the validated configuration
    pub fn config(&self) -> &Phase3Config {
        &self.config
    }

    /// Run all startup validations
    pub async fn run_validations(&self) -> Result<(), StartupError> {
        // Validate RuVector connectivity
        self.validate_ruvector().await?;

        info!("All Phase 3 startup validations passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_phase_error() {
        std::env::set_var("AGENT_PHASE", "phase2");
        std::env::set_var("AGENT_LAYER", "layer1");
        std::env::set_var("RUVECTOR_SERVICE_URL", "http://test");
        std::env::set_var("RUVECTOR_API_KEY", "test-key");

        let result = Phase3Config::from_env();
        assert!(matches!(result, Err(StartupError::InvalidPhase { .. })));

        // Cleanup
        std::env::remove_var("AGENT_PHASE");
        std::env::remove_var("AGENT_LAYER");
        std::env::remove_var("RUVECTOR_SERVICE_URL");
        std::env::remove_var("RUVECTOR_API_KEY");
    }

    #[test]
    fn test_missing_ruvector_url() {
        std::env::set_var("AGENT_PHASE", "phase3");
        std::env::set_var("AGENT_LAYER", "layer1");
        std::env::remove_var("RUVECTOR_SERVICE_URL");
        std::env::remove_var("RUVECTOR_ENDPOINT");
        std::env::set_var("RUVECTOR_API_KEY", "test-key");

        let result = Phase3Config::from_env();
        assert!(matches!(result, Err(StartupError::MissingRequired(_))));

        // Cleanup
        std::env::remove_var("AGENT_PHASE");
        std::env::remove_var("AGENT_LAYER");
        std::env::remove_var("RUVECTOR_API_KEY");
    }
}
