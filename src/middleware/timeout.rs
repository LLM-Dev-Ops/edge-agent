//! Request timeout middleware

use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use std::time::Duration;
use tower::{Layer, Service};
use tower::timeout::TimeoutLayer as TowerTimeoutLayer;
use tracing::debug;

/// Timeout layer wrapper
#[derive(Clone)]
pub struct TimeoutLayer {
    inner: TowerTimeoutLayer,
}

impl TimeoutLayer {
    /// Create a new timeout layer with the specified duration
    pub fn new(duration: Duration) -> Self {
        debug!(timeout_secs = duration.as_secs(), "Configuring request timeout");
        Self {
            inner: TowerTimeoutLayer::new(duration),
        }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = <TowerTimeoutLayer as Layer<S>>::Service;

    fn layer(&self, service: S) -> Self::Service {
        self.inner.layer(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_layer_creation() {
        let duration = Duration::from_secs(30);
        let layer = TimeoutLayer::new(duration);
        // Just verify it creates without panicking
        assert!(true);
    }
}
