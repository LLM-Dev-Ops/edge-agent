//! Execution span types for the Agentics Foundational Execution Unit.
//!
//! These types implement the causal span graph required by the Agentics
//! execution system. Every repo entry creates a repo-level span; every
//! agent execution creates an agent-level span parented to the repo span.
//!
//! # Invariants
//!
//! - Core -> Repo -> Agent(s) hierarchy is always maintained
//! - A repo span with zero agent spans is INVALID
//! - Spans are append-only and causally ordered via parent_span_id
//! - All spans are JSON-serializable without loss

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// The type of execution span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    /// Repo-level span: created on entry to this Foundational Execution Unit.
    Repo,
    /// Agent-level span: created per agent execution within the repo span.
    Agent,
}

/// Status of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    /// Span is in progress.
    InProgress,
    /// Span completed successfully.
    Completed,
    /// Span completed with a failure.
    Failed,
}

/// An artifact attached to a span.
///
/// Artifacts are machine-verifiable evidence produced during execution.
/// They include DecisionEvents, agent outputs, metrics, or any structured data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanArtifact {
    /// Artifact type identifier (e.g., "decision_event", "agent_output").
    pub artifact_type: String,
    /// The artifact data, JSON-serializable.
    pub data: serde_json::Value,
    /// Timestamp when artifact was produced.
    pub timestamp: DateTime<Utc>,
}

/// A single execution span (repo or agent level).
///
/// Spans form a tree: repo spans are children of the Core span,
/// and agent spans are children of the repo span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    /// Unique span ID (UUID v4).
    pub span_id: String,
    /// Parent span ID. For repo spans, this is the caller's (Core's) span ID.
    /// For agent spans, this is the repo span_id.
    pub parent_span_id: String,
    /// The execution_id from the caller, shared across all spans in one execution.
    pub execution_id: String,
    /// Span type (repo or agent).
    pub span_type: SpanType,
    /// Human-readable name. For repo spans: "edge-agent". For agent spans: the agent name.
    pub name: String,
    /// Repository name, always "edge-agent" for this repo.
    pub repo_name: String,
    /// Status of this span.
    pub status: SpanStatus,
    /// When the span started.
    pub started_at: DateTime<Utc>,
    /// When the span ended (None if still in progress).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Duration in microseconds (populated on completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
    /// Artifacts attached to this span.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<SpanArtifact>,
    /// Additional attributes (e.g., error messages).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
}

const REPO_NAME: &str = "edge-agent";

impl ExecutionSpan {
    /// Create a new repo-level span.
    pub fn new_repo(execution_id: &str, parent_span_id: &str) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: parent_span_id.to_string(),
            execution_id: execution_id.to_string(),
            span_type: SpanType::Repo,
            name: REPO_NAME.to_string(),
            repo_name: REPO_NAME.to_string(),
            status: SpanStatus::InProgress,
            started_at: Utc::now(),
            ended_at: None,
            duration_us: None,
            artifacts: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Create a new agent-level span parented to the repo span.
    pub fn new_agent(execution_id: &str, repo_span_id: &str, agent_name: &str) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: repo_span_id.to_string(),
            execution_id: execution_id.to_string(),
            span_type: SpanType::Agent,
            name: agent_name.to_string(),
            repo_name: REPO_NAME.to_string(),
            status: SpanStatus::InProgress,
            started_at: Utc::now(),
            ended_at: None,
            duration_us: None,
            artifacts: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Complete the span successfully.
    pub fn complete(&mut self) {
        let now = Utc::now();
        self.ended_at = Some(now);
        self.status = SpanStatus::Completed;
        self.duration_us = Some(
            (now - self.started_at)
                .num_microseconds()
                .unwrap_or(0) as u64,
        );
    }

    /// Complete the span with an error.
    pub fn complete_with_error(&mut self, error_msg: &str) {
        let now = Utc::now();
        self.ended_at = Some(now);
        self.status = SpanStatus::Failed;
        self.duration_us = Some(
            (now - self.started_at)
                .num_microseconds()
                .unwrap_or(0) as u64,
        );
        self.attributes.insert(
            "error".to_string(),
            serde_json::Value::String(error_msg.to_string()),
        );
    }

    /// Attach an artifact to this span.
    pub fn attach_artifact(&mut self, artifact_type: &str, data: serde_json::Value) {
        self.artifacts.push(SpanArtifact {
            artifact_type: artifact_type.to_string(),
            data,
            timestamp: Utc::now(),
        });
    }
}

/// The complete span graph for one execution through this repo.
///
/// This structure is append-only and causally ordered.
/// It is the mandatory output of every execution through this Foundational Execution Unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanGraph {
    /// The execution_id for this graph.
    pub execution_id: String,
    /// The repo-level span (root of this graph).
    pub repo_span: ExecutionSpan,
    /// Agent-level spans, in causal (append) order.
    pub agent_spans: Vec<ExecutionSpan>,
}

impl SpanGraph {
    /// Create a new span graph with a repo-level root span.
    pub fn new(execution_id: &str, parent_span_id: &str) -> Self {
        Self {
            execution_id: execution_id.to_string(),
            repo_span: ExecutionSpan::new_repo(execution_id, parent_span_id),
            agent_spans: Vec::new(),
        }
    }

    /// Start a new agent span and return its index.
    pub fn start_agent_span(&mut self, agent_name: &str) -> usize {
        let span = ExecutionSpan::new_agent(
            &self.execution_id,
            &self.repo_span.span_id,
            agent_name,
        );
        self.agent_spans.push(span);
        self.agent_spans.len() - 1
    }

    /// Complete an agent span by index.
    pub fn complete_agent_span(&mut self, index: usize) {
        if let Some(span) = self.agent_spans.get_mut(index) {
            span.complete();
        }
    }

    /// Complete an agent span with error.
    pub fn complete_agent_span_with_error(&mut self, index: usize, error: &str) {
        if let Some(span) = self.agent_spans.get_mut(index) {
            span.complete_with_error(error);
        }
    }

    /// Attach an artifact to an agent span.
    pub fn attach_artifact_to_agent(
        &mut self,
        index: usize,
        artifact_type: &str,
        data: serde_json::Value,
    ) {
        if let Some(span) = self.agent_spans.get_mut(index) {
            span.attach_artifact(artifact_type, data);
        }
    }

    /// Finalize the repo span. Returns an error if no agent spans were emitted.
    pub fn finalize(&mut self) -> Result<(), String> {
        if self.agent_spans.is_empty() {
            self.repo_span
                .complete_with_error("No agent spans emitted during execution");
            return Err("No agent spans emitted during execution".to_string());
        }
        self.repo_span.complete();
        Ok(())
    }

    /// Finalize the repo span as failed, preserving all emitted spans.
    pub fn finalize_failed(&mut self, reason: &str) {
        self.repo_span.complete_with_error(reason);
    }
}

/// Thread-safe span collector that can be shared across async handlers.
///
/// Wraps a `SpanGraph` in `Arc<Mutex<>>` for interior mutability.
/// Inserted into Axum request extensions by the execution span middleware.
#[derive(Debug, Clone)]
pub struct SpanCollector {
    inner: Arc<Mutex<SpanGraph>>,
}

impl SpanCollector {
    /// Create a new SpanCollector with a repo-level root span.
    pub fn new(execution_id: &str, parent_span_id: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SpanGraph::new(execution_id, parent_span_id))),
        }
    }

    /// Start a new agent span and return its index.
    pub fn start_agent_span(&self, agent_name: &str) -> usize {
        self.inner.lock().unwrap().start_agent_span(agent_name)
    }

    /// Complete an agent span by index.
    pub fn complete_agent_span(&self, index: usize) {
        self.inner.lock().unwrap().complete_agent_span(index);
    }

    /// Complete an agent span with error.
    pub fn complete_agent_span_with_error(&self, index: usize, error: &str) {
        self.inner
            .lock()
            .unwrap()
            .complete_agent_span_with_error(index, error);
    }

    /// Attach an artifact to an agent span.
    pub fn attach_artifact(
        &self,
        span_index: usize,
        artifact_type: &str,
        data: serde_json::Value,
    ) {
        self.inner
            .lock()
            .unwrap()
            .attach_artifact_to_agent(span_index, artifact_type, data);
    }

    /// Get the repo span ID.
    pub fn repo_span_id(&self) -> String {
        self.inner.lock().unwrap().repo_span.span_id.clone()
    }

    /// Clone the inner graph, finalize it, and return. Safe to call while
    /// other references to the collector still exist.
    pub fn finalize_cloned(&self) -> Result<SpanGraph, String> {
        let mut graph = self.inner.lock().unwrap().clone();
        graph.finalize()?;
        Ok(graph)
    }

    /// Clone the inner graph, finalize as failed, and return.
    /// Always succeeds (does not check for agent spans).
    pub fn finalize_failed_cloned(&self, reason: &str) -> SpanGraph {
        let mut graph = self.inner.lock().unwrap().clone();
        graph.finalize_failed(reason);
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_graph_creation() {
        let graph = SpanGraph::new("exec-1", "parent-1");
        assert_eq!(graph.execution_id, "exec-1");
        assert_eq!(graph.repo_span.parent_span_id, "parent-1");
        assert_eq!(graph.repo_span.span_type, SpanType::Repo);
        assert_eq!(graph.repo_span.name, "edge-agent");
        assert_eq!(graph.repo_span.repo_name, "edge-agent");
        assert_eq!(graph.repo_span.status, SpanStatus::InProgress);
    }

    #[test]
    fn test_agent_span_creation() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        assert_eq!(idx, 0);
        assert_eq!(graph.agent_spans[0].name, "test-agent");
        assert_eq!(graph.agent_spans[0].span_type, SpanType::Agent);
        assert_eq!(
            graph.agent_spans[0].parent_span_id,
            graph.repo_span.span_id
        );
        assert_eq!(graph.agent_spans[0].execution_id, "exec-1");
        assert_eq!(graph.agent_spans[0].repo_name, "edge-agent");
    }

    #[test]
    fn test_finalize_rejects_empty_spans() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let result = graph.finalize();
        assert!(result.is_err());
        assert_eq!(graph.repo_span.status, SpanStatus::Failed);
    }

    #[test]
    fn test_finalize_succeeds_with_agent_spans() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.complete_agent_span(idx);
        assert!(graph.finalize().is_ok());
        assert_eq!(graph.repo_span.status, SpanStatus::Completed);
        assert!(graph.repo_span.ended_at.is_some());
        assert!(graph.repo_span.duration_us.is_some());
    }

    #[test]
    fn test_agent_span_completion() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.complete_agent_span(idx);
        assert_eq!(graph.agent_spans[0].status, SpanStatus::Completed);
        assert!(graph.agent_spans[0].ended_at.is_some());
    }

    #[test]
    fn test_agent_span_error() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.complete_agent_span_with_error(idx, "something failed");
        assert_eq!(graph.agent_spans[0].status, SpanStatus::Failed);
        assert_eq!(
            graph.agent_spans[0].attributes.get("error").unwrap(),
            &serde_json::Value::String("something failed".to_string())
        );
    }

    #[test]
    fn test_artifact_attachment() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.attach_artifact_to_agent(
            idx,
            "decision_event",
            serde_json::json!({"allowed": true}),
        );
        assert_eq!(graph.agent_spans[0].artifacts.len(), 1);
        assert_eq!(
            graph.agent_spans[0].artifacts[0].artifact_type,
            "decision_event"
        );
    }

    #[test]
    fn test_span_graph_serialization() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.attach_artifact_to_agent(idx, "agent_output", serde_json::json!({"result": "ok"}));
        graph.complete_agent_span(idx);
        graph.finalize().unwrap();

        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.contains("exec-1"));
        assert!(json.contains("test-agent"));
        assert!(json.contains("agent_output"));

        // Roundtrip
        let parsed: SpanGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.execution_id, "exec-1");
        assert_eq!(parsed.agent_spans.len(), 1);
    }

    #[test]
    fn test_span_collector_thread_safety() {
        let collector = SpanCollector::new("exec-1", "parent-1");
        let c2 = collector.clone();
        let idx = c2.start_agent_span("agent-1");
        c2.complete_agent_span(idx);
        let graph = collector.finalize_cloned().unwrap();
        assert_eq!(graph.agent_spans.len(), 1);
    }

    #[test]
    fn test_causal_ordering() {
        let collector = SpanCollector::new("exec-1", "parent-1");
        let idx1 = collector.start_agent_span("agent-1");
        collector.complete_agent_span(idx1);
        let idx2 = collector.start_agent_span("agent-2");
        collector.complete_agent_span(idx2);
        let graph = collector.finalize_cloned().unwrap();
        assert_eq!(graph.agent_spans.len(), 2);
        assert!(graph.agent_spans[0].started_at <= graph.agent_spans[1].started_at);
    }

    #[test]
    fn test_multiple_artifacts_on_single_span() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx = graph.start_agent_span("test-agent");
        graph.attach_artifact_to_agent(idx, "decision_event", serde_json::json!({"type": "allow"}));
        graph.attach_artifact_to_agent(idx, "agent_output", serde_json::json!({"result": true}));
        assert_eq!(graph.agent_spans[0].artifacts.len(), 2);
    }

    #[test]
    fn test_finalize_failed_cloned() {
        let collector = SpanCollector::new("exec-1", "parent-1");
        let graph = collector.finalize_failed_cloned("upstream timeout");
        assert_eq!(graph.repo_span.status, SpanStatus::Failed);
        assert_eq!(
            graph.repo_span.attributes.get("error").unwrap(),
            &serde_json::Value::String("upstream timeout".to_string())
        );
    }

    #[test]
    fn test_span_ids_are_unique() {
        let mut graph = SpanGraph::new("exec-1", "parent-1");
        let idx1 = graph.start_agent_span("agent-1");
        let idx2 = graph.start_agent_span("agent-2");
        assert_ne!(graph.agent_spans[idx1].span_id, graph.agent_spans[idx2].span_id);
        assert_ne!(graph.repo_span.span_id, graph.agent_spans[idx1].span_id);
    }

    #[test]
    fn test_parent_span_id_chain() {
        let graph = SpanGraph::new("exec-1", "core-span-123");
        assert_eq!(graph.repo_span.parent_span_id, "core-span-123");

        let mut graph = graph;
        let idx = graph.start_agent_span("my-agent");
        assert_eq!(
            graph.agent_spans[idx].parent_span_id,
            graph.repo_span.span_id
        );
    }
}
