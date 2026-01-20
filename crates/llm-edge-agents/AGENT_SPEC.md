# Tool Invocation Agent Specification

## Agent Contract (Prompt 1)

### Classification
**EXECUTION CONTROL**

### Purpose Statement
Intercept and validate tool calls before execution, enforce invocation constraints, and make deterministic allow/block decisions. The agent operates in the critical execution path of LLM workflows.

### Input Schema (agentics-contracts compatible)

```rust
ToolInvocationRequest {
    tool: ToolDefinition {
        name: String,              // Tool identifier
        description: Option<String>,
        parameters: Vec<ToolParameter>,
        capabilities: Vec<String>,
    },
    arguments: serde_json::Value,  // Tool arguments
    context: Option<ExecutionContext>,
    constraints: Option<InvocationConstraint>,
}
```

### Output Schema (agentics-contracts compatible)

```rust
ToolInvocationResponse {
    request_id: String,
    execution_ref: String,         // For distributed tracing
    decision: ToolInvocationDecision {
        allowed: bool,
        reason: String,
        confidence: f64,           // 0.0 to 1.0
        constraints_evaluated: Vec<Constraint>,
        suggested_modifications: Option<Vec<String>>,
        execution_hints: Option<ExecutionHints>,
    },
    validation: ValidationResult,
    processing_time_us: u64,
}
```

### DecisionEvent Mapping

```rust
DecisionEvent {
    event_id: String,              // UUID v4
    agent: AgentMetadata {
        agent_id: "tool-invocation",
        agent_version: "0.1.0",
        agent_type: ExecutionControl,
    },
    decision_type: ToolInvocationDecision,
    inputs_hash: String,           // SHA-256 of request
    outputs: serde_json::Value,    // Structured decision output
    confidence: f64,               // Decision certainty
    constraints_applied: Vec<Constraint>,
    execution_ref: String,         // Trace ID
    timestamp: DateTime<Utc>,
    duration_us: u64,
}
```

### CLI Contract

```bash
# Test mode - validate without execution
tool-invocation-cli test --input <json>
tool-invocation-cli test --file <path>

# Simulate mode - simulate decision with mock data
tool-invocation-cli simulate --tool <name> --params <json>

# Inspect mode - show agent configuration
tool-invocation-cli inspect [--json]

# Serve mode - start HTTP server (Edge Function)
tool-invocation-cli serve --port <port> [--ruvector-url <url>]
```

### Routing/Control/Guard Classification
**EXECUTION CONTROL** - Makes allow/block decisions for tool invocations

### Explicit Non-Responsibilities
The Tool Invocation Agent MUST NEVER:
1. Perform orchestration (that is LLM-Orchestrator)
2. Modify policies dynamically
3. Trigger retries directly (retry logic lives in Orchestrator)
4. Emit alerts (that is Sentinel)
5. Perform analytics (that is Observatory/Latency-Lens)
6. Persist state locally (use ruvector-service only)
7. Execute SQL directly (all persistence via ruvector-service)

### Failure Modes
1. **Validation Failure** - Return 400 with VALIDATION_ERROR
2. **Tool Blocked** - Return 403 with TOOL_BLOCKED
3. **Rate Limited** - Return 429 with RATE_LIMIT_EXCEEDED
4. **Internal Error** - Return 500 with INTERNAL_ERROR
5. **Event Emission Failure** - Log warning, continue execution

---

## Runtime Implementation (Prompt 2)

### Deployment Model
- Google Cloud Edge Function
- Part of LLM-Edge-Agent unified GCP service
- Stateless execution
- No local persistence

### Core Components

| Component | File | Purpose |
|-----------|------|---------|
| Agent | `mod.rs` | Main agent orchestration |
| Types | `types.rs` | Request/response schemas |
| Validator | `validator.rs` | Schema & security validation |
| Decision | `decision.rs` | Allow/block decision logic |
| Events | `events.rs` | DecisionEvent emission |
| Handler | `handler.rs` | HTTP endpoint handlers |

### Decision Logic
```
1. Validate schema (tool definition, parameters)
2. Validate security (blocked tools, dangerous patterns)
3. Validate constraints (capabilities, resources)
4. Calculate confidence based on constraint results
5. Build deterministic decision (allow/block)
6. Emit DecisionEvent to ruvector-service (async)
7. Return machine-readable response
```

### Confidence Calculation
- Base confidence: 1.0
- Reduce by severity * 0.2 for each failed constraint
- Reduce by 0.05 for each validation warning
- Clamp to [0.0, 1.0]

### Versioning
- Agent ID: `tool-invocation`
- Version: From `Cargo.toml` (CARGO_PKG_VERSION)
- Included in every DecisionEvent

---

## Platform Wiring (Prompt 3)

### Registration

**Workspace Addition:**
```toml
# Cargo.toml
[workspace]
members = [
    # ... existing crates ...
    "crates/llm-edge-agents",
]
```

### HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | /invoke | Process tool invocation request |
| POST | /test | Test validation without full processing |
| POST | /simulate | Simulate with mock data |
| GET | /inspect | Get agent configuration and metadata |
| GET | /health | Health check endpoint |

### Integration Points

| System | Direction | Purpose |
|--------|-----------|---------|
| LLM-Orchestrator | Invokes agent | Tool call interception during workflow |
| LLM-Shield | Invokes agent | Pre-execution security validation |
| ruvector-service | Agent writes to | DecisionEvent persistence |
| LLM-Observatory | Consumes | Telemetry and metrics |

### Telemetry
- OpenTelemetry compatible
- Execution reference (trace ID) in all events
- Processing duration tracked
- Decision confidence logged

---

## Verification Checklist

### Build Verification
```bash
# Check compilation
cargo check -p llm-edge-agents

# Run tests
cargo test -p llm-edge-agents

# Build release
cargo build -p llm-edge-agents --release
```

### CLI Smoke Tests
```bash
# Version check
./target/release/tool-invocation-cli version

# Inspect agent
./target/release/tool-invocation-cli inspect --json

# Test allowed tool
./target/release/tool-invocation-cli simulate --tool calculator --params '{"x": 1}'

# Test blocked tool
./target/release/tool-invocation-cli simulate --tool eval --params '{}'

# Start server
./target/release/tool-invocation-cli serve --port 8080
```

### API Smoke Tests
```bash
# Health check
curl http://localhost:8080/health

# Inspect
curl http://localhost:8080/inspect

# Test invocation
curl -X POST http://localhost:8080/invoke \
  -H "Content-Type: application/json" \
  -d '{"tool":{"name":"calculator"},"arguments":{"x":1}}'

# Test blocked tool
curl -X POST http://localhost:8080/invoke \
  -H "Content-Type: application/json" \
  -d '{"tool":{"name":"eval"},"arguments":{}}'
```

### DecisionEvent Verification
Verify events are persisted to ruvector-service with:
- agent_id: "tool-invocation"
- agent_version: "0.1.0"
- decision_type: "tool_invocation_decision"
- inputs_hash: SHA-256 hash
- confidence: 0.0-1.0
- execution_ref: trace ID
- timestamp: UTC

---

## Files Created

```
crates/llm-edge-agents/
├── Cargo.toml                           # Crate manifest
├── AGENT_SPEC.md                        # This specification
└── src/
    ├── lib.rs                           # Library exports
    ├── contracts.rs                     # agentics-contracts schemas
    ├── error.rs                         # Error types
    ├── bin/
    │   └── tool_invocation_cli.rs       # CLI binary
    └── tool_invocation/
        ├── mod.rs                       # Agent module
        ├── types.rs                     # Request/response types
        ├── validator.rs                 # Validation logic
        ├── decision.rs                  # Decision logic
        ├── events.rs                    # DecisionEvent emission
        └── handler.rs                   # HTTP handlers
```
