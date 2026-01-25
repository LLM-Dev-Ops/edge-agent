//! Performance Budget Enforcement
//!
//! Enforces Phase 3 Layer 1 performance budgets:
//! - MAX_TOKENS=1500
//! - MAX_LATENCY_MS=3000
//! - MAX_CALLS_PER_RUN=4

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Performance budget configuration
#[derive(Debug, Clone, Copy)]
pub struct PerformanceBudget {
    /// Maximum tokens per invocation
    pub max_tokens: u32,
    /// Maximum latency in milliseconds
    pub max_latency_ms: u32,
    /// Maximum external calls per run
    pub max_calls_per_run: u32,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            max_tokens: 1500,
            max_latency_ms: 3000,
            max_calls_per_run: 4,
        }
    }
}

impl PerformanceBudget {
    /// Create from Phase 3 defaults
    pub fn phase3_defaults() -> Self {
        Self::default()
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self {
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
        }
    }
}

/// Budget violation types
#[derive(Debug, Clone, thiserror::Error)]
pub enum BudgetViolation {
    #[error("Token budget exceeded: used {used}, max {max}")]
    TokenBudgetExceeded { used: u32, max: u32 },

    #[error("Latency budget exceeded: {elapsed_ms}ms > {max_ms}ms")]
    LatencyBudgetExceeded { elapsed_ms: u64, max_ms: u32 },

    #[error("Call budget exceeded: {used} calls > {max} max")]
    CallBudgetExceeded { used: u32, max: u32 },
}

/// Budget enforcer for a single invocation
pub struct BudgetEnforcer {
    budget: PerformanceBudget,
    start_time: Instant,
    tokens_used: AtomicU32,
    calls_made: AtomicU32,
}

impl BudgetEnforcer {
    /// Create a new budget enforcer
    pub fn new(budget: PerformanceBudget) -> Self {
        Self {
            budget,
            start_time: Instant::now(),
            tokens_used: AtomicU32::new(0),
            calls_made: AtomicU32::new(0),
        }
    }

    /// Create with Phase 3 defaults
    pub fn phase3() -> Self {
        Self::new(PerformanceBudget::phase3_defaults())
    }

    /// Record tokens used and check budget
    pub fn record_tokens(&self, tokens: u32) -> Result<(), BudgetViolation> {
        let total = self.tokens_used.fetch_add(tokens, Ordering::SeqCst) + tokens;

        if total > self.budget.max_tokens {
            error!(
                used = total,
                max = self.budget.max_tokens,
                "Token budget exceeded"
            );
            return Err(BudgetViolation::TokenBudgetExceeded {
                used: total,
                max: self.budget.max_tokens,
            });
        }

        Ok(())
    }

    /// Record an external call and check budget
    pub fn record_call(&self) -> Result<(), BudgetViolation> {
        let total = self.calls_made.fetch_add(1, Ordering::SeqCst) + 1;

        if total > self.budget.max_calls_per_run {
            error!(
                used = total,
                max = self.budget.max_calls_per_run,
                "Call budget exceeded"
            );
            return Err(BudgetViolation::CallBudgetExceeded {
                used: total,
                max: self.budget.max_calls_per_run,
            });
        }

        Ok(())
    }

    /// Check latency budget
    pub fn check_latency(&self) -> Result<(), BudgetViolation> {
        let elapsed = self.start_time.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        if elapsed_ms > self.budget.max_latency_ms as u64 {
            error!(
                elapsed_ms = elapsed_ms,
                max_ms = self.budget.max_latency_ms,
                "Latency budget exceeded"
            );
            return Err(BudgetViolation::LatencyBudgetExceeded {
                elapsed_ms,
                max_ms: self.budget.max_latency_ms,
            });
        }

        Ok(())
    }

    /// Check all budgets
    pub fn check_all(&self) -> Result<(), BudgetViolation> {
        self.check_latency()?;
        // Tokens and calls are checked on record
        Ok(())
    }

    /// Get current usage stats
    pub fn usage(&self) -> BudgetUsage {
        BudgetUsage {
            tokens_used: self.tokens_used.load(Ordering::SeqCst),
            tokens_max: self.budget.max_tokens,
            calls_made: self.calls_made.load(Ordering::SeqCst),
            calls_max: self.budget.max_calls_per_run,
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
            latency_max_ms: self.budget.max_latency_ms,
        }
    }

    /// Get remaining latency budget
    pub fn remaining_latency(&self) -> Duration {
        let elapsed = self.start_time.elapsed();
        let max = Duration::from_millis(self.budget.max_latency_ms as u64);
        max.saturating_sub(elapsed)
    }

    /// Check if latency is approaching limit (>80%)
    pub fn is_latency_critical(&self) -> bool {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        elapsed_ms > (self.budget.max_latency_ms as u64 * 80 / 100)
    }
}

/// Current budget usage
#[derive(Debug, Clone)]
pub struct BudgetUsage {
    pub tokens_used: u32,
    pub tokens_max: u32,
    pub calls_made: u32,
    pub calls_max: u32,
    pub elapsed_ms: u64,
    pub latency_max_ms: u32,
}

impl BudgetUsage {
    /// Convert to JSON-serializable format
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "tokens": {
                "used": self.tokens_used,
                "max": self.tokens_max,
                "utilization": format!("{:.1}%", self.tokens_used as f64 / self.tokens_max as f64 * 100.0)
            },
            "calls": {
                "made": self.calls_made,
                "max": self.calls_max,
                "utilization": format!("{:.1}%", self.calls_made as f64 / self.calls_max as f64 * 100.0)
            },
            "latency": {
                "elapsed_ms": self.elapsed_ms,
                "max_ms": self.latency_max_ms,
                "utilization": format!("{:.1}%", self.elapsed_ms as f64 / self.latency_max_ms as f64 * 100.0)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget() {
        let enforcer = BudgetEnforcer::phase3();

        // Should succeed within budget
        assert!(enforcer.record_tokens(500).is_ok());
        assert!(enforcer.record_tokens(500).is_ok());
        assert!(enforcer.record_tokens(400).is_ok());

        // Should fail over budget
        assert!(matches!(
            enforcer.record_tokens(200),
            Err(BudgetViolation::TokenBudgetExceeded { .. })
        ));
    }

    #[test]
    fn test_call_budget() {
        let enforcer = BudgetEnforcer::phase3();

        // Should succeed within budget (4 calls max)
        assert!(enforcer.record_call().is_ok());
        assert!(enforcer.record_call().is_ok());
        assert!(enforcer.record_call().is_ok());
        assert!(enforcer.record_call().is_ok());

        // Should fail on 5th call
        assert!(matches!(
            enforcer.record_call(),
            Err(BudgetViolation::CallBudgetExceeded { .. })
        ));
    }

    #[test]
    fn test_usage_tracking() {
        let enforcer = BudgetEnforcer::phase3();
        enforcer.record_tokens(100).unwrap();
        enforcer.record_call().unwrap();

        let usage = enforcer.usage();
        assert_eq!(usage.tokens_used, 100);
        assert_eq!(usage.calls_made, 1);
    }
}
