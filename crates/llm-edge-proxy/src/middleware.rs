//! Middleware implementations for the proxy
//!
//! Includes:
//! - Rate limiting with tower-governor
//! - API key authentication
//! - Request validation
//! - Timeout handling

pub mod auth;
pub mod rate_limit;
pub mod timeout;

pub use auth::auth_middleware;
pub use rate_limit::create_rate_limiter;
pub use timeout::TimeoutLayer;
