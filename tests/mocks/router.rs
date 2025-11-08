//! Mock routing implementations for testing

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::helpers::*;

/// Mock routing engine
pub struct MockRoutingEngine {
    state: Arc<RwLock<RouterState>>,
}

struct RouterState {
    strategy: RoutingStrategy,
    providers: Vec<ProviderConfig>,
    circuit_breakers: HashMap<String, CircuitBreakerState>,
}

#[derive(Clone)]
struct ProviderConfig {
    name: String,
    priority: u32,
    cost_per_1k_tokens: f64,
    avg_latency_ms: u64,
    healthy: bool,
}

impl MockRoutingEngine {
    pub fn new(strategy: RoutingStrategy) -> Self {
        let providers = vec![
            ProviderConfig {
                name: "openai".to_string(),
                priority: 1,
                cost_per_1k_tokens: 0.03,
                avg_latency_ms: 100,
                healthy: true,
            },
            ProviderConfig {
                name: "anthropic".to_string(),
                priority: 2,
                cost_per_1k_tokens: 0.025,
                avg_latency_ms: 120,
                healthy: true,
            },
        ];

        let mut circuit_breakers = HashMap::new();
        for provider in &providers {
            circuit_breakers.insert(provider.name.clone(), CircuitBreakerState::Closed);
        }

        Self {
            state: Arc::new(RwLock::new(RouterState {
                strategy,
                providers,
                circuit_breakers,
            })),
        }
    }

    pub async fn select_provider(&self, _request: &ChatCompletionRequest) -> Option<String> {
        let state = self.state.read().await;

        match state.strategy {
            RoutingStrategy::RoundRobin => {
                // Simple round-robin
                state.providers.first().map(|p| p.name.clone())
            }
            RoutingStrategy::Failover => {
                // Select by priority
                state.providers
                    .iter()
                    .filter(|p| p.healthy)
                    .min_by_key(|p| p.priority)
                    .map(|p| p.name.clone())
            }
            RoutingStrategy::LeastLatency => {
                // Select fastest
                state.providers
                    .iter()
                    .filter(|p| p.healthy)
                    .min_by_key(|p| p.avg_latency_ms)
                    .map(|p| p.name.clone())
            }
            RoutingStrategy::CostOptimized => {
                // Select cheapest
                state.providers
                    .iter()
                    .filter(|p| p.healthy)
                    .min_by(|a, b| a.cost_per_1k_tokens.partial_cmp(&b.cost_per_1k_tokens).unwrap())
                    .map(|p| p.name.clone())
            }
            RoutingStrategy::HealthAware => {
                // Select first healthy
                state.providers
                    .iter()
                    .find(|p| p.healthy)
                    .map(|p| p.name.clone())
            }
        }
    }

    pub async fn set_provider_health(&self, provider: &str, healthy: bool) {
        let mut state = self.state.write().await;
        if let Some(p) = state.providers.iter_mut().find(|p| p.name == provider) {
            p.healthy = healthy;
        }
    }

    pub async fn set_provider_latency(&self, provider: &str, latency_ms: u64) {
        let mut state = self.state.write().await;
        if let Some(p) = state.providers.iter_mut().find(|p| p.name == provider) {
            p.avg_latency_ms = latency_ms;
        }
    }

    pub async fn get_circuit_breaker_state(&self, provider: &str) -> CircuitBreakerState {
        let state = self.state.read().await;
        state.circuit_breakers
            .get(provider)
            .copied()
            .unwrap_or(CircuitBreakerState::Closed)
    }

    pub async fn set_circuit_breaker_state(&self, provider: &str, cb_state: CircuitBreakerState) {
        let mut state = self.state.write().await;
        state.circuit_breakers.insert(provider.to_string(), cb_state);
    }

    pub async fn record_failure(&self, provider: &str) {
        // Mock circuit breaker logic
        let current_state = self.get_circuit_breaker_state(provider).await;
        if current_state == CircuitBreakerState::Closed {
            // After certain failures, open the circuit
            self.set_circuit_breaker_state(provider, CircuitBreakerState::Open).await;
        }
    }

    pub async fn record_success(&self, provider: &str) {
        let current_state = self.get_circuit_breaker_state(provider).await;
        if current_state == CircuitBreakerState::HalfOpen {
            self.set_circuit_breaker_state(provider, CircuitBreakerState::Closed).await;
        }
    }
}

/// Mock circuit breaker
pub struct MockCircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<std::sync::atomic::AtomicU32>,
    failure_threshold: u32,
}

impl MockCircuitBreaker {
    pub fn new(failure_threshold: u32) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            failure_threshold,
        }
    }

    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let state = *self.state.read().await;

        match state {
            CircuitBreakerState::Open => {
                Err(CircuitBreakerError::Open)
            }
            CircuitBreakerState::HalfOpen => {
                match f() {
                    Ok(result) => {
                        self.record_success().await;
                        Ok(result)
                    }
                    Err(err) => {
                        self.record_failure().await;
                        Err(CircuitBreakerError::CallFailed(err))
                    }
                }
            }
            CircuitBreakerState::Closed => {
                match f() {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        self.record_failure().await;
                        Err(CircuitBreakerError::CallFailed(err))
                    }
                }
            }
        }
    }

    async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if count >= self.failure_threshold {
            *self.state.write().await = CircuitBreakerState::Open;
        }
    }

    async fn record_success(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.state.write().await = CircuitBreakerState::Closed;
    }

    pub async fn state(&self) -> CircuitBreakerState {
        *self.state.read().await
    }

    pub async fn reset(&self) {
        *self.state.write().await = CircuitBreakerState::Closed;
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub async fn trip(&self) {
        *self.state.write().await = CircuitBreakerState::Open;
    }

    pub async fn half_open(&self) {
        *self.state.write().await = CircuitBreakerState::HalfOpen;
    }
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    Open,
    CallFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Circuit breaker is open"),
            Self::CallFailed(err) => write!(f, "Call failed: {}", err),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitBreakerError<E> {}
