//! Prometheus metrics for LLM Edge Agent
//!
//! Exposes comprehensive metrics for monitoring:
//! - Request rates and latencies
//! - Cache hit rates
//! - Error rates and types
//! - Provider performance
//! - Token usage and costs

use metrics::{
    counter, describe_counter, describe_gauge, describe_histogram,
    gauge, histogram, Unit,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info};

/// Metrics registry for the application
pub struct MetricsRegistry {
    handle: PrometheusHandle,
}

impl MetricsRegistry {
    /// Initialize metrics exporter and return handle
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let builder = PrometheusBuilder::new();
        
        // Configure histogram buckets for latency measurements
        let builder = builder
            .set_buckets_for_metric(
                Matcher::Full("llm_request_duration_seconds".to_string()),
                &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
            )?
            .set_buckets_for_metric(
                Matcher::Full("llm_cache_lookup_duration_seconds".to_string()),
                &[0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1],
            )?
            .set_buckets_for_metric(
                Matcher::Full("llm_provider_request_duration_seconds".to_string()),
                &[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0],
            )?;
        
        let handle = builder.install_recorder()?;
        
        // Register metric descriptions
        Self::register_metrics();
        
        info!("Metrics registry initialized");
        
        Ok(Self { handle })
    }
    
    /// Get metrics output in Prometheus format
    pub fn render(&self) -> String {
        self.handle.render()
    }
    
    /// Register all metric descriptions
    fn register_metrics() {
        // Request metrics
        describe_counter!(
            "llm_requests_total",
            Unit::Count,
            "Total number of LLM requests received"
        );
        
        describe_counter!(
            "llm_requests_success_total",
            Unit::Count,
            "Total number of successful LLM requests"
        );
        
        describe_counter!(
            "llm_requests_error_total",
            Unit::Count,
            "Total number of failed LLM requests"
        );
        
        describe_histogram!(
            "llm_request_duration_seconds",
            Unit::Seconds,
            "Request duration in seconds"
        );
        
        // Cache metrics
        describe_counter!(
            "llm_cache_hits_total",
            Unit::Count,
            "Total number of cache hits"
        );
        
        describe_counter!(
            "llm_cache_misses_total",
            Unit::Count,
            "Total number of cache misses"
        );
        
        describe_histogram!(
            "llm_cache_lookup_duration_seconds",
            Unit::Seconds,
            "Cache lookup duration in seconds"
        );
        
        describe_gauge!(
            "llm_cache_size_bytes",
            Unit::Bytes,
            "Current cache size in bytes"
        );
        
        // Provider metrics
        describe_counter!(
            "llm_provider_requests_total",
            Unit::Count,
            "Total requests per provider"
        );
        
        describe_counter!(
            "llm_provider_errors_total",
            Unit::Count,
            "Total errors per provider"
        );
        
        describe_histogram!(
            "llm_provider_request_duration_seconds",
            Unit::Seconds,
            "Provider request duration in seconds"
        );
        
        describe_gauge!(
            "llm_provider_circuit_breaker_state",
            Unit::Count,
            "Circuit breaker state (0=closed, 1=half-open, 2=open)"
        );
        
        // Token and cost metrics
        describe_counter!(
            "llm_tokens_total",
            Unit::Count,
            "Total tokens processed"
        );
        
        describe_counter!(
            "llm_tokens_prompt_total",
            Unit::Count,
            "Total prompt tokens"
        );
        
        describe_counter!(
            "llm_tokens_completion_total",
            Unit::Count,
            "Total completion tokens"
        );
        
        describe_counter!(
            "llm_cost_total_cents",
            Unit::Count,
            "Total cost in cents"
        );
        
        // System metrics
        describe_gauge!(
            "llm_active_connections",
            Unit::Count,
            "Number of active client connections"
        );
        
        describe_gauge!(
            "llm_provider_health",
            Unit::Count,
            "Provider health status (0=unhealthy, 1=healthy)"
        );
    }
}

/// Request metrics tracker
pub struct RequestMetrics;

impl RequestMetrics {
    /// Record a new request
    pub fn record_request(provider: &str, model: &str) {
        counter!("llm_requests_total", "provider" => provider.to_string(), "model" => model.to_string()).increment(1);
    }
    
    /// Record a successful request
    pub fn record_success(provider: &str, model: &str, duration: Duration) {
        counter!("llm_requests_success_total", "provider" => provider.to_string(), "model" => model.to_string()).increment(1);
        histogram!("llm_request_duration_seconds", "provider" => provider.to_string(), "model" => model.to_string()).record(duration.as_secs_f64());
    }
    
    /// Record a failed request
    pub fn record_error(provider: &str, model: &str, error_type: &str, duration: Duration) {
        counter!("llm_requests_error_total", 
            "provider" => provider.to_string(), 
            "model" => model.to_string(),
            "error_type" => error_type.to_string()
        ).increment(1);
        histogram!("llm_request_duration_seconds", "provider" => provider.to_string(), "model" => model.to_string()).record(duration.as_secs_f64());
    }
}

/// Cache metrics tracker
pub struct CacheMetrics;

impl CacheMetrics {
    /// Record a cache hit
    pub fn record_hit(cache_tier: &str) {
        counter!("llm_cache_hits_total", "tier" => cache_tier.to_string()).increment(1);
    }
    
    /// Record a cache miss
    pub fn record_miss(cache_tier: &str) {
        counter!("llm_cache_misses_total", "tier" => cache_tier.to_string()).increment(1);
    }
    
    /// Record cache lookup duration
    pub fn record_lookup_duration(cache_tier: &str, duration: Duration) {
        histogram!("llm_cache_lookup_duration_seconds", "tier" => cache_tier.to_string()).record(duration.as_secs_f64());
    }
    
    /// Update cache size
    pub fn update_cache_size(cache_tier: &str, size_bytes: u64) {
        gauge!("llm_cache_size_bytes", "tier" => cache_tier.to_string()).set(size_bytes as f64);
    }
    
    /// Calculate and record cache hit rate
    pub fn calculate_hit_rate(hits: u64, misses: u64) -> f64 {
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}

/// Provider metrics tracker
pub struct ProviderMetrics;

impl ProviderMetrics {
    /// Record a provider request
    pub fn record_request(provider: &str) {
        counter!("llm_provider_requests_total", "provider" => provider.to_string()).increment(1);
    }
    
    /// Record a provider error
    pub fn record_error(provider: &str, error_type: &str) {
        counter!("llm_provider_errors_total", 
            "provider" => provider.to_string(),
            "error_type" => error_type.to_string()
        ).increment(1);
    }
    
    /// Record provider request duration
    pub fn record_duration(provider: &str, duration: Duration) {
        histogram!("llm_provider_request_duration_seconds", "provider" => provider.to_string()).record(duration.as_secs_f64());
    }
    
    /// Update circuit breaker state
    pub fn update_circuit_breaker_state(provider: &str, state: CircuitBreakerState) {
        let state_value = match state {
            CircuitBreakerState::Closed => 0.0,
            CircuitBreakerState::HalfOpen => 1.0,
            CircuitBreakerState::Open => 2.0,
        };
        gauge!("llm_provider_circuit_breaker_state", "provider" => provider.to_string()).set(state_value);
    }
    
    /// Update provider health
    pub fn update_health(provider: &str, is_healthy: bool) {
        let health_value = if is_healthy { 1.0 } else { 0.0 };
        gauge!("llm_provider_health", "provider" => provider.to_string()).set(health_value);
    }
}

/// Circuit breaker state for metrics
#[derive(Debug, Clone, Copy)]
pub enum CircuitBreakerState {
    Closed,
    HalfOpen,
    Open,
}

/// Token and cost metrics tracker
pub struct TokenMetrics;

impl TokenMetrics {
    /// Record token usage
    pub fn record_tokens(
        provider: &str,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
    ) {
        let total_tokens = prompt_tokens + completion_tokens;
        
        counter!("llm_tokens_total", 
            "provider" => provider.to_string(),
            "model" => model.to_string()
        ).increment(total_tokens);
        
        counter!("llm_tokens_prompt_total",
            "provider" => provider.to_string(),
            "model" => model.to_string()
        ).increment(prompt_tokens);
        
        counter!("llm_tokens_completion_total",
            "provider" => provider.to_string(),
            "model" => model.to_string()
        ).increment(completion_tokens);
    }
    
    /// Record cost
    pub fn record_cost(
        provider: &str,
        model: &str,
        cost_cents: f64,
    ) {
        counter!("llm_cost_total_cents",
            "provider" => provider.to_string(),
            "model" => model.to_string()
        ).increment(cost_cents as u64);
    }
    
    /// Calculate cost from tokens
    pub fn calculate_cost(
        prompt_tokens: u64,
        completion_tokens: u64,
        prompt_cost_per_1k: f64,
        completion_cost_per_1k: f64,
    ) -> f64 {
        let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_cost_per_1k;
        let completion_cost = (completion_tokens as f64 / 1000.0) * completion_cost_per_1k;
        prompt_cost + completion_cost
    }
}

/// System metrics tracker
pub struct SystemMetrics;

impl SystemMetrics {
    /// Update active connections count
    pub fn update_active_connections(count: i64) {
        gauge!("llm_active_connections").increment(count as f64);
    }
    
    /// Increment active connections
    pub fn increment_connections() {
        Self::update_active_connections(1);
    }
    
    /// Decrement active connections
    pub fn decrement_connections() {
        Self::update_active_connections(-1);
    }
}

/// Start metrics server
pub async fn start_metrics_server(
    addr: SocketAddr,
    registry: MetricsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::{routing::get, Router};
    
    info!("Starting metrics server on {}", addr);
    
    let app = Router::new().route(
        "/metrics",
        get(move || async move { registry.render() }),
    );
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_hit_rate_calculation() {
        assert_eq!(CacheMetrics::calculate_hit_rate(80, 20), 80.0);
        assert_eq!(CacheMetrics::calculate_hit_rate(0, 0), 0.0);
        assert_eq!(CacheMetrics::calculate_hit_rate(50, 50), 50.0);
    }
    
    #[test]
    fn test_cost_calculation() {
        let cost = TokenMetrics::calculate_cost(1000, 500, 0.03, 0.06);
        assert!((cost - 0.06).abs() < 0.001); // 0.03 + 0.03 = 0.06
    }
}
