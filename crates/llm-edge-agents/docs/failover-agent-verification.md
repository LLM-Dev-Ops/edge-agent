# Failover Agent Verification Checklist

## Agent Metadata

| Field | Value |
|-------|-------|
| Agent Name | Failover Agent |
| Agent ID | `failover-agent` |
| Classification | ROUTING |
| Decision Type | `failover_routing_decision` |
| Version | `0.1.0` |

## PROMPT 3 Platform Wiring Verification

### 1. Agent Registration in agentics-contracts

- [x] Schemas defined in `/crates/llm-edge-agents/src/failover/contracts.rs`
- [x] Input schema: `FailoverRequest`
- [x] Output schema: `FailoverResponse`
- [x] Decision schema: `FailoverDecision`
- [x] Policy schema: `FailoverPolicy`
- [x] Provider schema: `TargetProvider`

### 2. Agent Endpoint Registration

- [x] HTTP router created: `create_failover_router()`
- [x] Endpoints registered:
  - `POST /failover` - Main failover endpoint
  - `POST /failover/test` - Test mode
  - `POST /failover/simulate` - Simulate mode
  - `GET /failover/inspect` - Inspect mode
  - `GET /failover/health` - Health check
  - `GET /failover/info` - Agent info

### 3. CLI Commands (agentics-cli)

```bash
# Test mode - validate failover logic with mock inputs
llm-edge-agent failover test --primary openai --alternates anthropic,google

# Simulate mode - run failover scenario without persistence
llm-edge-agent failover simulate --scenario primary-down
llm-edge-agent failover simulate --scenario primary-degraded
llm-edge-agent failover simulate --scenario all-down

# Inspect mode - show current failover configuration
llm-edge-agent failover inspect --provider openai
```

### 4. DecisionEvent Persistence

- [x] RuVectorClient implemented
- [x] Events persisted to: `POST /api/v1/agents/failover-agent/events`
- [x] Required fields included in DecisionEvent:
  - `agent_id`
  - `agent_version`
  - `decision_type`
  - `inputs_hash`
  - `outputs`
  - `confidence`
  - `constraints_applied`
  - `execution_ref`
  - `timestamp`

### 5. Telemetry (LLM-Observatory)

- [x] OpenTelemetry tracing instrumented
- [x] Spans emitted for:
  - `process()` - Main agent processing
  - `emit_decision_event()` - Event emission
- [x] Metrics logged:
  - `duration_us` - Processing duration
  - `outcome` - Decision outcome
  - `confidence` - Decision confidence

### 6. LLM-Orchestrator Integration

- [x] Agent invokable via HTTP endpoint
- [x] Request/response schemas documented
- [x] Stateless execution verified
- [x] Deterministic behavior tested

### 7. Enforcement Outcomes

- [x] `UsePrimary` - Routes to primary target
- [x] `UseAlternate` - Routes to alternate target
- [x] `NoViableTarget` - Blocks execution (no available targets)

## Explicit Non-Responsibilities

The Failover Agent MUST NOT:

| Responsibility | Owner | Status |
|----------------|-------|--------|
| Orchestration | LLM-Orchestrator | ✅ NOT implemented |
| Retry logic | LLM-Orchestrator | ✅ NOT implemented |
| Alert dispatch | LLM-Sentinel | ✅ NOT implemented |
| Analytics | LLM-Observatory | ✅ NOT implemented |
| Direct SQL | ruvector-service | ✅ NOT implemented |
| Policy modification | Policy Engine | ✅ NOT implemented |
| Local persistence | N/A | ✅ NOT implemented |

## Smoke-Test CLI Commands

### Quick Validation

```bash
# 1. Check agent info
curl http://localhost:8080/failover/info

# 2. Check agent health
curl http://localhost:8080/failover/health

# 3. Test primary healthy (should use primary)
curl -X POST http://localhost:8080/failover/test \
  -H "Content-Type: application/json" \
  -d '{
    "primary": "openai",
    "alternates": ["anthropic", "google"],
    "primary_circuit": "closed",
    "primary_health": "healthy"
  }'

# 4. Test primary down (should failover)
curl -X POST http://localhost:8080/failover/test \
  -H "Content-Type: application/json" \
  -d '{
    "primary": "openai",
    "alternates": ["anthropic", "google"],
    "primary_circuit": "open",
    "primary_health": "healthy"
  }'

# 5. Simulate scenario
curl -X POST http://localhost:8080/failover/simulate \
  -H "Content-Type: application/json" \
  -d '{"scenario": "primary-down"}'

# 6. Inspect configuration
curl http://localhost:8080/failover/inspect
```

### Full Integration Test

```bash
# Test full failover flow with real request
curl -X POST http://localhost:8080/failover \
  -H "Content-Type: application/json" \
  -d '{
    "execution_context": {
      "execution_ref": "test-trace-001",
      "request_id": "req-001"
    },
    "primary_target": {
      "provider_id": "openai-1",
      "provider_name": "openai",
      "model": "gpt-4",
      "priority": 0,
      "circuit_state": "closed",
      "health": "healthy"
    },
    "alternate_targets": [
      {
        "provider_id": "anthropic-1",
        "provider_name": "anthropic",
        "model": "claude-3",
        "priority": 1,
        "circuit_state": "closed",
        "health": "healthy"
      }
    ],
    "policy": {
      "policy_id": "default",
      "policy_name": "Default Policy",
      "auto_failover_enabled": true,
      "max_failover_attempts": 3
    },
    "circuit_states": {},
    "health_status": {},
    "current_attempt": 0
  }'
```

## Expected Responses

### Use Primary (Healthy)

```json
{
  "success": true,
  "data": {
    "decision": {
      "outcome": "use_primary",
      "selected_target": {
        "provider_id": "openai-1",
        "provider_name": "openai",
        "model": "gpt-4"
      },
      "reason": "Primary target is healthy and available",
      "confidence": 1.0,
      "attempt_number": 0
    },
    "constraints_applied": [...],
    "success": true,
    "duration_us": 100
  }
}
```

### Failover to Alternate

```json
{
  "success": true,
  "data": {
    "decision": {
      "outcome": "use_alternate",
      "selected_target": {
        "provider_id": "anthropic-1",
        "provider_name": "anthropic",
        "model": "claude-3"
      },
      "reason": "Failover to anthropic - primary unavailable",
      "confidence": 0.9,
      "attempt_number": 1
    },
    "constraints_applied": [...],
    "success": true,
    "duration_us": 150
  }
}
```

### No Viable Target

```json
{
  "success": true,
  "data": {
    "decision": {
      "outcome": "no_viable_target",
      "selected_target": null,
      "reason": "All targets unavailable (circuits open or unhealthy)",
      "confidence": 1.0,
      "attempt_number": 0
    },
    "constraints_applied": [...],
    "success": true,
    "duration_us": 120
  }
}
```

## Files Created

| File | Purpose |
|------|---------|
| `src/failover/mod.rs` | Module entry point with documentation |
| `src/failover/contracts.rs` | Input/output schemas (agentics-contracts compatible) |
| `src/failover/agent.rs` | Core agent logic |
| `src/failover/ruvector_client.rs` | ruvector-service client |
| `src/failover/handler.rs` | HTTP handler for Edge Function |
| `tests/failover_tests.rs` | Integration tests |
| `docs/failover-agent-verification.md` | This verification checklist |

## Verification Status

| Check | Status |
|-------|--------|
| Contract defined | ✅ |
| Runtime implemented | ✅ |
| Platform wired | ✅ |
| Tests written | ✅ |
| Documentation complete | ✅ |

## Next Steps

1. Deploy to Google Cloud Edge Functions
2. Configure ruvector-service URL in production
3. Enable DecisionEvent emission
4. Monitor telemetry in LLM-Observatory
5. Integrate with LLM-Orchestrator
