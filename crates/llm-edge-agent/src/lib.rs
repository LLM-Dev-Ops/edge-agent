//! LLM Edge Agent - High-performance LLM Intercepting Proxy
//!
//! This crate provides the main application logic for the LLM Edge Agent,
//! integrating all layers into a complete end-to-end system:
//!
//! - Layer 1: HTTP Server (Axum) with auth and rate limiting
//! - Layer 2: Multi-tier caching (L1 Moka + L2 Redis)
//! - Layer 2: Intelligent routing with circuit breakers
//! - Layer 3: Provider adapters (OpenAI, Anthropic)
//! - Cross-cutting: Observability (Prometheus, OpenTelemetry, Logging)

pub mod integration;
pub mod proxy;

pub use integration::{AppConfig, AppState, initialize_app_state, check_system_health};
pub use proxy::{handle_chat_completions, ChatCompletionRequest, ChatCompletionResponse};
