//! Test configuration for integration tests

use std::time::Duration;
use super::RoutingStrategy;

/// Test configuration builder
#[derive(Clone)]
pub struct TestConfig {
    pub auth_enabled: bool,
    pub auth_key: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub request_timeout: Option<Duration>,
    pub provider_timeout: Option<Duration>,
    pub routing_strategy: RoutingStrategy,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    pub retry_policy: Option<RetryPolicy>,
    pub cache_ttl: Option<Duration>,
    pub l2_cache_enabled: bool,
    pub l2_cache_failure_mode: bool,
    pub tracing_enabled: bool,
    pub pii_redaction: bool,
    pub provider_priorities: Vec<(String, u32)>,
    pub fallback_chain: Vec<String>,
}

#[derive(Clone)]
pub struct RateLimit {
    pub max_requests: u32,
    pub window: Duration,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff: Duration,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            auth_key: None,
            rate_limit: None,
            request_timeout: None,
            provider_timeout: None,
            routing_strategy: RoutingStrategy::RoundRobin,
            circuit_breaker: None,
            retry_policy: None,
            cache_ttl: None,
            l2_cache_enabled: false,
            l2_cache_failure_mode: false,
            tracing_enabled: false,
            pii_redaction: false,
            provider_priorities: vec![],
            fallback_chain: vec![],
        }
    }
}

impl TestConfig {
    /// Create config with authentication enabled
    pub fn with_auth() -> Self {
        Self {
            auth_enabled: true,
            auth_key: Some("test-api-key".to_string()),
            ..Default::default()
        }
    }

    /// Create config with L2 cache enabled
    pub fn with_l2_cache() -> Self {
        Self {
            l2_cache_enabled: true,
            ..Default::default()
        }
    }

    /// Enable rate limiting
    pub fn with_rate_limit(mut self, max_requests: u32, window: Duration) -> Self {
        self.rate_limit = Some(RateLimit {
            max_requests,
            window,
        });
        self
    }

    /// Set request timeout
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Set provider timeout
    pub fn with_provider_timeout(mut self, timeout: Duration) -> Self {
        self.provider_timeout = Some(timeout);
        self
    }

    /// Set routing strategy
    pub fn with_routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.routing_strategy = strategy;
        self
    }

    /// Enable circuit breaker
    pub fn with_circuit_breaker(mut self, failure_threshold: u32, timeout: Duration) -> Self {
        self.circuit_breaker = Some(CircuitBreakerConfig {
            failure_threshold,
            timeout,
        });
        self
    }

    /// Set retry policy
    pub fn with_retry_policy(mut self, max_retries: u32, backoff: Duration) -> Self {
        self.retry_policy = Some(RetryPolicy {
            max_retries,
            backoff,
        });
        self
    }

    /// Set cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    /// Enable L2 cache failure mode (for testing degradation)
    pub fn with_l2_failure_mode(mut self) -> Self {
        self.l2_cache_failure_mode = true;
        self
    }

    /// Enable tracing
    pub fn with_tracing_enabled(mut self) -> Self {
        self.tracing_enabled = true;
        self
    }

    /// Enable PII redaction
    pub fn with_pii_redaction(mut self, enabled: bool) -> Self {
        self.pii_redaction = enabled;
        self
    }

    /// Set provider priorities
    pub fn with_provider_priorities(mut self, priorities: Vec<(&str, u32)>) -> Self {
        self.provider_priorities = priorities
            .into_iter()
            .map(|(name, priority)| (name.to_string(), priority))
            .collect();
        self
    }

    /// Set fallback chain
    pub fn with_fallback_chain(mut self, chain: Vec<&str>) -> Self {
        self.fallback_chain = chain.into_iter().map(|s| s.to_string()).collect();
        self
    }
}
