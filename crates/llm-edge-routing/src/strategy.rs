//! Routing strategies

/// A routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub provider_name: String,
    pub model: String,
    pub score: f64,
    pub reason: String,
}

/// Routing strategy enum
#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Route to the lowest cost provider
    CostBased,
    /// Route to the fastest provider (lowest latency)
    LatencyBased,
    /// Route based on multiple factors with weights
    Hybrid { cost_weight: f64, latency_weight: f64, reliability_weight: f64 },
    /// Simple round-robin
    RoundRobin,
}

impl RoutingStrategy {
    pub fn default_hybrid() -> Self {
        Self::Hybrid {
            cost_weight: 0.4,
            latency_weight: 0.4,
            reliability_weight: 0.2,
        }
    }
}
