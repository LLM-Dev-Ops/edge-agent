//! Span instrumentation for agent execution.
//!
//! Provides a generic wrapper that creates agent-level spans within a SpanCollector,
//! invokes the agent, and attaches artifacts (agent outputs) to the span.

use agentics_contracts::spans::SpanCollector;
use serde::Serialize;

/// Wraps an agent invocation with span tracking.
///
/// This function:
/// 1. Starts an agent span in the collector
/// 2. Calls the provided async closure (the actual agent work)
/// 3. On success: attaches the serialized response as an `agent_output` artifact, completes the span
/// 4. On error: records the error on the span, completes with error status
///
/// Returns the agent's result unchanged, along with the span index.
pub async fn with_agent_span<F, Fut, T, E>(
    collector: &SpanCollector,
    agent_name: &str,
    f: F,
) -> (Result<T, E>, usize)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    T: Serialize,
    E: std::fmt::Display,
{
    let span_index = collector.start_agent_span(agent_name);

    let result = f().await;

    match &result {
        Ok(value) => {
            if let Ok(data) = serde_json::to_value(value) {
                collector.attach_artifact(span_index, "agent_output", data);
            }
            collector.complete_agent_span(span_index);
        }
        Err(e) => {
            collector.complete_agent_span_with_error(span_index, &e.to_string());
        }
    }

    (result, span_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_agent_span_success() {
        let collector = SpanCollector::new("exec-1", "parent-1");

        let (result, idx) = with_agent_span(&collector, "test-agent", || async {
            Ok::<_, String>(serde_json::json!({"result": "ok"}))
        })
        .await;

        assert!(result.is_ok());
        let graph = collector.finalize_cloned().unwrap();
        assert_eq!(graph.agent_spans.len(), 1);
        assert_eq!(graph.agent_spans[idx].name, "test-agent");
        assert_eq!(
            graph.agent_spans[idx].status,
            agentics_contracts::spans::SpanStatus::Completed
        );
        assert_eq!(graph.agent_spans[idx].artifacts.len(), 1);
        assert_eq!(graph.agent_spans[idx].artifacts[0].artifact_type, "agent_output");
    }

    #[tokio::test]
    async fn test_with_agent_span_error() {
        let collector = SpanCollector::new("exec-1", "parent-1");

        let (result, idx) = with_agent_span(&collector, "test-agent", || async {
            Err::<serde_json::Value, String>("agent failed".to_string())
        })
        .await;

        assert!(result.is_err());
        // Even on error, finalize_cloned should work (there is a span, just failed)
        let graph = collector.finalize_cloned().unwrap();
        assert_eq!(graph.agent_spans.len(), 1);
        assert_eq!(
            graph.agent_spans[idx].status,
            agentics_contracts::spans::SpanStatus::Failed
        );
    }
}
