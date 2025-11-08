//! OpenTelemetry distributed tracing setup
//!
//! Provides end-to-end tracing for LLM requests:
//! - Request flow tracking across components
//! - Provider request spans
//! - Cache lookup spans
//! - Error tracking and debugging
//! - Integration with Jaeger/Tempo/etc via OTLP

use opentelemetry::{
    global,
    trace::{TraceError, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for traces
    pub service_name: String,
    
    /// Service version
    pub service_version: String,
    
    /// Environment (dev, staging, production)
    pub environment: String,
    
    /// OTLP endpoint (e.g., "http://jaeger:4317")
    pub otlp_endpoint: Option<String>,
    
    /// Sampling ratio (0.0 to 1.0)
    pub sampling_ratio: f64,
    
    /// Enable JSON formatted logs
    pub json_logs: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "llm-edge-agent".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
            sampling_ratio: 1.0, // Sample all traces by default
            json_logs: false,
        }
    }
}

/// Initialize OpenTelemetry tracing
pub fn init_tracing(config: TracingConfig) -> Result<(), TraceError> {
    info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        "Initializing OpenTelemetry tracing"
    );
    
    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);
    
    // Configure tracer provider
    let mut provider_builder = TracerProvider::builder()
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource);
    
    // Configure sampler based on sampling ratio
    let sampler = if config.sampling_ratio >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_ratio <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_ratio)
    };
    
    provider_builder = provider_builder.with_sampler(sampler);
    
    // Set up OTLP exporter if endpoint is configured
    if let Some(endpoint) = config.otlp_endpoint {
        info!(endpoint = %endpoint, "Configuring OTLP exporter");
        
        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(&endpoint),
            )
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(sampler)
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", config.service_name.clone()),
                        KeyValue::new("service.version", config.service_version.clone()),
                    ])),
            )
            .install_batch(runtime::Tokio)
        {
            Ok(provider) => {
                global::set_tracer_provider(provider);
                info!("OTLP exporter configured successfully");
            }
            Err(e) => {
                error!(error = %e, "Failed to initialize OTLP exporter");
                warn!("Continuing without OTLP export");
            }
        }
    } else {
        warn!("No OTLP endpoint configured, traces will not be exported");
    }
    
    // Set up tracing subscriber with OpenTelemetry layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(global::tracer(config.service_name.clone()));
    
    // Set up console/file logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,llm_edge_agent=debug"));
    
    if config.json_logs {
        // JSON formatted logs
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            )
            .init();
    } else {
        // Human-readable logs
        tracing_subscriber::registry()
            .with(env_filter)
            .with(telemetry_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true),
            )
            .init();
    }
    
    info!("Tracing initialized successfully");
    
    Ok(())
}

/// Shutdown tracing gracefully
pub fn shutdown_tracing() {
    info!("Shutting down tracing");
    global::shutdown_tracer_provider();
}

/// Common span attributes for LLM requests
pub mod span_attributes {
    use opentelemetry::KeyValue;
    
    /// Create span attributes for an LLM request
    pub fn llm_request(
        provider: &str,
        model: &str,
        request_id: &str,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("llm.provider", provider.to_string()),
            KeyValue::new("llm.model", model.to_string()),
            KeyValue::new("llm.request_id", request_id.to_string()),
        ]
    }
    
    /// Create span attributes for cache operations
    pub fn cache_operation(
        operation: &str,
        cache_tier: &str,
        key_hash: &str,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("cache.operation", operation.to_string()),
            KeyValue::new("cache.tier", cache_tier.to_string()),
            KeyValue::new("cache.key_hash", key_hash.to_string()),
        ]
    }
    
    /// Create span attributes for provider requests
    pub fn provider_request(
        provider: &str,
        endpoint: &str,
        attempt: u32,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("provider.name", provider.to_string()),
            KeyValue::new("provider.endpoint", endpoint.to_string()),
            KeyValue::new("provider.attempt", attempt as i64),
        ]
    }
    
    /// Create span attributes for errors
    pub fn error(
        error_type: &str,
        error_message: &str,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("error.type", error_type.to_string()),
            KeyValue::new("error.message", error_message.to_string()),
        ]
    }
}

/// Tracing helper macros
#[macro_export]
macro_rules! trace_span {
    ($name:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!(
            $name,
            $(
                $key = $value
            ),*
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "llm-edge-agent");
        assert_eq!(config.sampling_ratio, 1.0);
    }
    
    #[test]
    fn test_span_attributes() {
        let attrs = span_attributes::llm_request(
            "openai",
            "gpt-4",
            "req-123",
        );
        assert_eq!(attrs.len(), 3);
    }
}
