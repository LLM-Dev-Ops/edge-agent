//! LLM Edge Agent - Multi-Tier Caching System
//!
//! A high-performance caching system for LLM responses with:
//! - L1: In-memory cache (Moka) with TinyLFU eviction
//! - L2: Distributed cache (Redis) with persistence
//! - Prometheus metrics integration
//! - Sub-millisecond L1 latency, 1-2ms L2 latency

pub mod cache;

// Re-export commonly used types
pub use cache::{CacheLookupResult, CacheManager};
