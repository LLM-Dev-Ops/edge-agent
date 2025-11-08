use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitoringError {
    #[error("Metrics export error: {0}")]
    MetricsExport(String),

    #[error("Tracing initialization error: {0}")]
    TracingInit(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type MonitoringResult<T> = Result<T, MonitoringError>;
