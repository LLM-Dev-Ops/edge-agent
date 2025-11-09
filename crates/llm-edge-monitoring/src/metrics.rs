//! Prometheus metrics

use metrics::{counter, gauge, histogram};

/// Records a successful request
pub fn record_request_success(provider: &str, model: &str, latency_ms: u64) {
    counter!("llm_edge_requests_total", "provider" => provider.to_string(), "model" => model.to_string(), "status" => "success").increment(1);
    histogram!("llm_edge_request_duration_ms", "provider" => provider.to_string(), "model" => model.to_string()).record(latency_ms as f64);
}

/// Records a failed request
pub fn record_request_failure(provider: &str, model: &str, error_type: &str) {
    counter!("llm_edge_requests_total", "provider" => provider.to_string(), "model" => model.to_string(), "status" => "error", "error_type" => error_type.to_string()).increment(1);
}

/// Records a cache hit
pub fn record_cache_hit(tier: &str) {
    counter!("llm_edge_cache_hits_total", "tier" => tier.to_string()).increment(1);
}

/// Records a cache miss
pub fn record_cache_miss(tier: &str) {
    counter!("llm_edge_cache_misses_total", "tier" => tier.to_string()).increment(1);
}

/// Records token usage
pub fn record_token_usage(provider: &str, model: &str, input_tokens: usize, output_tokens: usize) {
    counter!("llm_edge_tokens_total", "provider" => provider.to_string(), "model" => model.to_string(), "type" => "input").increment(input_tokens as u64);
    counter!("llm_edge_tokens_total", "provider" => provider.to_string(), "model" => model.to_string(), "type" => "output").increment(output_tokens as u64);
}

/// Records cost
pub fn record_cost(provider: &str, model: &str, cost_usd: f64) {
    counter!("llm_edge_cost_usd_total", "provider" => provider.to_string(), "model" => model.to_string()).increment(cost_usd as u64);
}

/// Records active requests
pub fn record_active_requests(count: usize) {
    gauge!("llm_edge_active_requests").set(count as f64);
}

/// Records provider health
pub fn record_provider_health(provider: &str, is_healthy: bool) {
    gauge!("llm_edge_provider_available", "provider" => provider.to_string()).set(if is_healthy {
        1.0
    } else {
        0.0
    });
}
