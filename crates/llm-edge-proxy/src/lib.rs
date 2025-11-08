//! Core HTTP proxy functionality for LLM Edge Agent
//!
//! This crate provides the foundational proxy server capabilities including:
//! - TLS termination with Rustls
//! - Request/response handling
//! - Protocol detection (HTTP/1.1, HTTP/2, gRPC)
//! - Middleware integration points (auth, rate limiting, timeout)
//! - Health checks and metrics endpoints
//! - OpenTelemetry tracing integration

pub mod config;
pub mod error;
pub mod middleware;
pub mod server;

pub use config::Config;
pub use error::{ProxyError, ProxyResult};
pub use server::{build_app, create_router, serve};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert_eq!(2 + 2, 4);
    }
}
