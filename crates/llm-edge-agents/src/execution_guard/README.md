# Execution Guard Agent

## Classification: PROTECTION / GUARD

## Purpose Statement

The Execution Guard Agent is a stateless, deterministic runtime guard that enforces
execution safety constraints. It validates execution envelopes against configurable
resource limits and emits allow/block decisions.

## Scope

- Enforce token limits (input, output, total)
- Enforce time limits (max execution duration)
- Enforce memory limits
- Enforce cost limits (per-request budget)
- Enforce rate limits (requests per window)
- Enforce concurrency limits (max parallel executions)
- Block unsafe execution envelopes
- Emit guard decisions to ruvector-service

## Decision Type

`decision_type: "execution_guard_decision"`

## Input Schema Reference

Uses schemas from `agentics-contracts`:
- `ExecutionEnvelope` - The execution request to validate
- `ExecutionContext` - Distributed tracing context
- `ResourceLimits` - Configurable safety constraints

## Output Schema Reference

- `ExecutionGuardResponse` - Full response with decision and validation
- `ExecutionGuardDecision` - The allow/block decision with confidence
- `ValidationResult` - Detailed validation breakdown

## DecisionEvent Mapping

| Field | Value |
|-------|-------|
| `agent_id` | `"execution-guard"` |
| `agent_version` | CARGO_PKG_VERSION |
| `decision_type` | `ExecutionGuardDecision` |
| `inputs_hash` | SHA-256 of request_id + model + tokens |
| `outputs` | `{ allowed, reason, model, validation_summary }` |
| `confidence` | 0.95-1.0 (deterministic threshold-based) |
| `constraints_applied` | All evaluated resource/rate constraints |
| `execution_ref` | Trace ID from context |

### Data NOT Persisted

- Raw request payloads
- Sensitive metadata (API keys, credentials)
- PII from context
- Full execution envelope details

## CLI Contract

```bash
# Test mode - validate without full processing
llm-edge-agent agent execution-guard test --input <json>

# Simulate mode - simulate decision with mock data
llm-edge-agent agent execution-guard simulate \
  --model gpt-4 \
  --input-tokens 5000 \
  --max-output-tokens 2000 \
  --current-concurrency 3

# Inspect mode - show agent configuration and contract
llm-edge-agent agent execution-guard inspect
```

## HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/guard` | Process an execution guard request (allow/block) |
| POST | `/test` | Test validation without event emission |
| POST | `/simulate` | Simulate with mock envelope data |
| GET | `/inspect` | Get agent configuration and contract |
| GET | `/health` | Health check for load balancers |

## Constraints Enforced

| Constraint Type | Config Field | Default |
|-----------------|--------------|---------|
| Input tokens | `max_input_tokens` | 100,000 |
| Output tokens | `max_output_tokens` | 16,000 |
| Total tokens | `max_total_tokens` | 128,000 |
| Duration | `max_duration_ms` | 120,000ms |
| Memory | `max_memory_bytes` | 1GB |
| Cost | `max_cost_micros` | $10.00 |
| Concurrency | `max_concurrency` | 10 |
| Rate | `rate_limit_requests` | 100/min |

## Upstream Invokers

1. **LLM-Orchestrator** - Before executing LLM calls
2. **LLM-Shield** - For pre-execution resource validation
3. **CostOps** - For budget enforcement checks
4. **Direct API** - For testing and validation

## Non-Responsibilities

This agent MUST NEVER:

1. ❌ Perform orchestration (that is LLM-Orchestrator)
2. ❌ Modify policies dynamically
3. ❌ Trigger retries directly (retry logic lives in Orchestrator)
4. ❌ Emit alerts (that is Sentinel)
5. ❌ Perform analytics (that is Observatory/Latency-Lens)
6. ❌ Persist state locally (use ruvector-service only)
7. ❌ Execute SQL directly

## Failure Modes

| Failure | HTTP Status | Description |
|---------|-------------|-------------|
| `TOKEN_LIMIT_EXCEEDED` | 403 | Input/output/total tokens exceed limits |
| `COST_LIMIT_EXCEEDED` | 403 | Estimated cost exceeds budget |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests in window |
| `CONCURRENCY_LIMIT_EXCEEDED` | 429 | Too many parallel executions |
| `MODEL_BLOCKED` | 403 | Model is blocked by policy |
| `MODEL_NOT_ALLOWED` | 403 | Model not in allowlist |
| `DURATION_EXCEEDED` | 403 | Estimated duration too long |
| `MEMORY_EXCEEDED` | 403 | Estimated memory too high |
| `VALIDATION_ERROR` | 400 | Request schema validation failed |

## Versioning Rules

- Agent version follows semantic versioning (CARGO_PKG_VERSION)
- Breaking changes to input/output schemas require major version bump
- New constraint types require minor version bump
- Bug fixes use patch version

## Example Usage

```rust
use llm_edge_agents::{
    ExecutionGuardAgent, ExecutionGuardConfig, ExecutionGuardRequest,
    ExecutionEnvelope, ResourceLimits,
};
use llm_edge_agents::contracts::ExecutionContext;

// Create agent with default config
let config = ExecutionGuardConfig::default();
let agent = ExecutionGuardAgent::new(config);

// Build request
let request = ExecutionGuardRequest {
    envelope: ExecutionEnvelope {
        request_id: "req-123".to_string(),
        model: "gpt-4".to_string(),
        provider: Some("openai".to_string()),
        input_tokens: 5000,
        max_output_tokens: 2000,
        estimated_duration_ms: Some(10000),
        estimated_memory_bytes: None,
        estimated_cost_micros: Some(100000), // $0.10
        current_concurrency: 3,
        metadata: std::collections::HashMap::new(),
    },
    context: Some(ExecutionContext::new()),
    limits: None, // Use defaults
    current_rate: Some(25),
};

// Process request
let response = agent.process(request).await?;

if response.decision.allowed {
    println!("Execution allowed: {}", response.decision.reason);
    // Proceed with LLM call
} else {
    println!("Execution blocked: {}", response.decision.reason);
    for violation in &response.decision.violations {
        println!("  - {}: {}", violation.constraint_type, violation.description);
    }
}
```

## Platform Registration

### agentics-contracts Registration

```yaml
agent:
  id: execution-guard
  version: 0.1.0
  classification: protection
  decision_type: execution_guard_decision

endpoints:
  - method: POST
    path: /guard
    request_schema: ExecutionGuardRequest
    response_schema: ExecutionGuardResponse
```

### LLM-Edge-Agent Service Registration

```yaml
agents:
  execution-guard:
    handler: execution_guard_handler
    health_check: /health
    endpoints:
      - /guard
      - /test
      - /simulate
      - /inspect
```

## Verification Checklist

- [x] Agent contract defined in agentics-contracts schemas
- [x] Agent registered in llm-edge-agents/src/lib.rs
- [x] DecisionEvent schema matches agentics-contracts
- [x] HTTP handler compatible with Google Cloud Edge Functions
- [x] CLI commands documented (test/simulate/inspect)
- [x] Telemetry emission via ruvector-service client
- [x] Health check endpoint available
- [x] Non-responsibilities explicitly documented
- [x] Failure modes enumerated with HTTP status codes

## Smoke Test Commands

```bash
# Health check
curl http://localhost:8080/health

# Inspect agent
curl http://localhost:8080/inspect | jq .

# Simulate allowed request
curl -X POST http://localhost:8080/simulate \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "input_tokens": 1000, "max_output_tokens": 2000}'

# Simulate blocked request (token limit)
curl -X POST http://localhost:8080/simulate \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4", "input_tokens": 200000, "max_output_tokens": 50000}'

# Full guard request
curl -X POST http://localhost:8080/guard \
  -H "Content-Type: application/json" \
  -d '{
    "envelope": {
      "request_id": "test-123",
      "model": "gpt-4",
      "input_tokens": 5000,
      "max_output_tokens": 2000,
      "current_concurrency": 2
    },
    "current_rate": 10
  }'
```
