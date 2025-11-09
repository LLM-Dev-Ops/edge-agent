//! OpenTelemetry tracing configuration

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing with OpenTelemetry support
pub fn init_tracing() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Configure the tracing subscriber with JSON formatting
    let formatting_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .init();

    tracing::info!("Tracing initialized successfully");
    Ok(())
}

/// Create a span for request tracing
#[macro_export]
macro_rules! trace_request {
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}
