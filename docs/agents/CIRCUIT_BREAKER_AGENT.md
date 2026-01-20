# Circuit Breaker Agent

## Overview

**Agent Name:** Circuit Breaker Agent
**Agent ID:** `circuit_breaker_agent`
**Classification:** PROTECTION / GUARD
**Decision Type:** `circuit_breaker_decision`

## Purpose

Prevent cascading failures by blocking execution paths when thresholds are exceeded.

## Classification

| Attribute | Value |
|-----------|-------|
| Type | PROTECTION / GUARD |
| Decision Type | `circuit_breaker_decision` |
| Stateless | Yes |
| Persistence | ruvector-service only |
| Deployment | Google Cloud Edge Function |

## Scope

- Track failure signals (input-provided only)
- Apply circuit open / close decisions
- Emit block or allow outcomes

## Contract Definition (PROMPT 1)

### Input Schema

Reference: `llm_edge_agents::circuit_breaker::types::CircuitBreakerRequest`

```rust
pub struct CircuitBreakerRequest {
    pub execution_ref: String,           // Required: Execution reference
    pub provider_id: String,             // Required: Provider to check
    pub current_state: CircuitState,     // Current circuit state
    pub failure_count: u32,              // Current failure count
    pub success_count: u32,              // Current success count
    pub last_failure_time: Option<DateTime<Utc>>,
    pub failure_signal: Option<FailureSignal>,  // New failure to record
    pub success_signal: Option<SuccessSignal>,  // New success to record
    pub config: CircuitBreakerConfig,    // Policy configuration
    pub correlation_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### Output Schema

Reference: `llm_edge_agents::circuit_breaker::types::CircuitBreakerResponse`

```rust
pub struct CircuitBreakerResponse {
    pub decision: CircuitBreakerDecision,  // Allow, Block, AllowWithProbe
    pub new_state: CircuitState,           // Updated circuit state
    pub new_failure_count: u32,
    pub new_success_count: u32,
    pub new_last_failure_time: Option<DateTime<Utc>>,
    pub retry_after_seconds: Option<u32>,  // For blocked requests
    pub confidence: f64,                   // 1.0 (deterministic)
    pub reason: String,
    pub state_transition: Option<StateTransition>,
    pub duration_us: u64,
}
```

### DecisionEvent Mapping

Every invocation emits exactly ONE DecisionEvent:

```json
{
    "event_id": "uuid",
    "agent": {
        "agent_id": "circuit_breaker_agent",
        "agent_version": "0.1.0",
        "agent_type": "protection"
    },
    "decision_type": "circuit_breaker_decision",
    "inputs_hash": "sha256:...",
    "outputs": {
        "decision": "block|allow|allow_with_probe",
        "new_state": "closed|open|half_open",
        "new_failure_count": 5,
        "reason": "..."
    },
    "confidence": 1.0,
    "constraints_applied": [...],
    "execution_ref": "...",
    "timestamp": "2024-01-20T00:00:00Z",
    "duration_us": 150
}
```

### Confidence Semantics

- **1.0**: Deterministic, threshold-based decision
- All decisions are threshold-based, so confidence is always 1.0

### Constraints Applied

1. **failure_threshold**: Whether failure count exceeds threshold
2. **success_threshold**: Whether success count reaches recovery threshold (half-open)
3. **timeout_elapsed**: Whether timeout has passed for half-open transition

## CLI Contract

### test command

```bash
circuit-breaker-cli test \
    --provider-id <PROVIDER> \
    --failures <COUNT> \
    --successes <COUNT> \
    --failure-threshold <N> \
    --success-threshold <N> \
    --timeout-seconds <N> \
    --format <json|text>
```

### simulate command

```bash
circuit-breaker-cli simulate \
    --provider-id <PROVIDER> \
    --state <closed|open|half_open> \
    --failures <COUNT> \
    --successes <COUNT> \
    --add-failure \
    --add-success \
    --format <json|text>
```

### inspect command

```bash
circuit-breaker-cli inspect \
    --provider-id <PROVIDER> \
    --format <json|text>
```

### serve command

```bash
circuit-breaker-cli serve \
    --port <PORT> \
    --host <HOST>
```

## Upstream Systems

This agent is invoked by:
- **LLM-Orchestrator**: During execution flow for circuit state evaluation

This agent may invoke:
- **LLM-Shield**: For security enforcement (if configured)

## Non-Responsibilities

This agent MUST NEVER:

1. **Perform orchestration** - That is LLM-Orchestrator's responsibility
2. **Modify policies dynamically** - Policies are provided as input, not modified
3. **Trigger retries directly** - Retry logic lives in LLM-Orchestrator
4. **Emit alerts** - That is LLM-Sentinel's responsibility
5. **Perform analytics** - That is LLM-Observatory / Latency-Lens
6. **Persist state locally** - All persistence via ruvector-service only
7. **Execute SQL directly** - Only ruvector-service client calls
8. **Connect to Google SQL** - Persistence is abstracted

## Failure Modes

| Failure Mode | Behavior |
|--------------|----------|
| Invalid input | Return validation error, no DecisionEvent |
| ruvector-service unavailable | Log error, continue (non-blocking persistence) |
| Timeout | Return timeout error |
| Internal error | Return internal error with details |

## Persistence (ruvector-service)

### What IS Persisted

- DecisionEvent for every invocation
- Circuit breaker state updates:
  - provider_id
  - state (closed/open/half_open)
  - failure_count
  - success_count
  - last_failure_time
  - last_state_change
  - updated_at

### What is NOT Persisted

- Raw request content
- API keys or credentials
- Full error stack traces
- User identifiers (only anonymized if present)
- Internal processing state

## Versioning

- Agent version follows semantic versioning
- Breaking changes increment major version
- New features increment minor version
- Bug fixes increment patch version
- DecisionEvent includes agent_version for compatibility tracking

## HTTP Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/invoke` | POST | Main agent invocation |
| `/test` | POST | Test scenario execution |
| `/simulate` | POST | Dry-run simulation |
| `/inspect/:provider_id` | GET | Inspect state and config |
| `/health` | GET | Health check |
| `/metrics` | GET | Prometheus metrics |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| RUVECTOR_ENABLED | false | Enable persistence |
| RUVECTOR_ENDPOINT | http://localhost:3000 | ruvector-service URL |
| RUVECTOR_API_KEY | - | API key for auth |
| CB_FAILURE_THRESHOLD | 5 | Default failure threshold |
| CB_SUCCESS_THRESHOLD | 3 | Default success threshold |
| CB_TIMEOUT_SECONDS | 30 | Default timeout |

## Integration with LLM-Edge-Agent Platform

### Registration

The agent is registered in:
- `crates/llm-edge-agents/src/lib.rs` (Rust module)
- `crates/llm-edge-agents/Cargo.toml` (binary target)

### Telemetry

The agent emits telemetry via:
- `tracing` macros (info!, warn!, error!)
- OpenTelemetry-compatible span context
- Prometheus metrics

### Health Checks

The agent exposes health status:
- Agent readiness
- ruvector-service connectivity

## Smoke Test Commands

```bash
# Test circuit opening after failures
circuit-breaker-cli test --provider-id openai --failures 6 --format json

# Test circuit recovery
circuit-breaker-cli test --provider-id openai --failures 5 --successes 4 --format json

# Simulate half-open state
circuit-breaker-cli simulate --provider-id openai --state half_open --add-success

# Inspect configuration
circuit-breaker-cli inspect --provider-id openai --format json

# Start HTTP server
circuit-breaker-cli serve --port 8080

# Test via HTTP
curl -X POST http://localhost:8080/invoke \
  -H "Content-Type: application/json" \
  -d '{"execution_ref":"test","provider_id":"openai","current_state":"closed","failure_count":0,"success_count":0,"config":{"provider_id":"openai","failure_threshold":5,"success_threshold":3,"timeout_seconds":30},"timestamp":"2024-01-20T00:00:00Z"}'
```

## Verification Checklist

- [x] Imports schemas from agentics-contracts / contracts module
- [x] Validates all inputs against contracts
- [x] Emits telemetry compatible with LLM-Observatory
- [x] Emits exactly ONE DecisionEvent per invocation
- [x] Exposes CLI-invokable endpoints (test/simulate/inspect)
- [x] Deployable as Google Edge Function
- [x] Returns deterministic, machine-readable output
- [x] Does NOT orchestrate workflows
- [x] Does NOT retry execution
- [x] Does NOT modify system policies
