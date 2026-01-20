# LLM-Edge-Agent Production Deployment Guide

## Overview

This document provides the complete deployment specification for LLM-Edge-Agent to Google Cloud Run in the `agentics-dev` project.

---

## 1. SERVICE TOPOLOGY

### Unified Service Name
```
llm-edge-agent
```

### Agent Endpoints (ALL agents deployed as ONE unified service)

| Agent | Classification | Base Path | Endpoints |
|-------|---------------|-----------|-----------|
| **Tool Invocation Agent** | EXECUTION CONTROL | `/agents/tool-invocation` | `POST /invoke`, `POST /test`, `POST /simulate`, `GET /inspect`, `GET /health` |
| **Circuit Breaker Agent** | PROTECTION/GUARD | `/agents/circuit-breaker` | `POST /invoke`, `POST /test`, `POST /simulate`, `GET /inspect/:provider_id`, `GET /health`, `GET /metrics` |
| **Failover Agent** | ROUTING | `/agents/failover` | `POST /failover`, `POST /failover/test`, `POST /failover/simulate`, `GET /failover/inspect`, `GET /failover/health`, `GET /failover/info` |
| **Execution Guard Agent** | PROTECTION/GUARD | `/agents/execution-guard` | `POST /guard`, `POST /test`, `POST /simulate`, `GET /inspect`, `GET /health` |
| **Caching Strategy Agent** | EXECUTION CONTROL | `/agents/caching-strategy` | `POST /cache-strategy`, `POST /cache-strategy/test`, `POST /cache-strategy/simulate`, `GET /cache-strategy/inspect` |

### Service Health Endpoints
- `GET /health` - Main health check
- `GET /health/ready` - Readiness probe (for load balancers)
- `GET /health/live` - Liveness probe (for k8s/Cloud Run)
- `GET /metrics` - Prometheus metrics
- `GET /` - Service info (lists all agents and endpoints)

### Architecture Confirmations
- ✅ **No agent is deployed as a standalone service** - All 5 agents run within ONE Cloud Run service
- ✅ **Shared runtime** - Single Rust binary (`llm-edge-agent-unified`)
- ✅ **Shared configuration** - Environment variables apply to all agents
- ✅ **Shared telemetry** - All agents emit to the same Observatory endpoint

---

## 2. ENVIRONMENT CONFIGURATION

### Required Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `PLATFORM_ENV` | Environment identifier | `dev` / `staging` / `prod` |
| `SERVICE_NAME` | Service name | `llm-edge-agent` |
| `SERVICE_VERSION` | Service version | `0.1.0` or commit SHA |
| `RUVECTOR_SERVICE_URL` | RuVector service endpoint | `https://ruvector-service-xxx-uc.a.run.app` |
| `RUVECTOR_API_KEY` | RuVector API key | (from Secret Manager) |
| `TELEMETRY_ENDPOINT` | LLM-Observatory endpoint | `https://observatory-xxx-uc.a.run.app` |
| `RUST_LOG` | Logging level | `llm_edge_agents=info,tower_http=info` |

### What is NOT Configured (By Design)

| Variable | Reason |
|----------|--------|
| `DATABASE_URL` | Edge-Agent does NOT connect to databases |
| `OPENAI_API_KEY` | Edge-Agent does NOT call LLM providers |
| `ANTHROPIC_API_KEY` | Edge-Agent does NOT call LLM providers |
| Provider endpoints | Edge-Agent routes but does NOT execute |

### Secrets Management

Secrets are stored in Google Secret Manager:
- `ruvector-config` - Contains `service-url` and `api-key`
- `telemetry-config` - Contains `endpoint`

---

## 3. GOOGLE SQL / MEMORY WIRING

### Confirmations

| Requirement | Status |
|-------------|--------|
| LLM-Edge-Agent does NOT connect directly to Google SQL | ✅ CONFIRMED |
| All DecisionEvents written via ruvector-service | ✅ CONFIRMED |
| Schema compatibility with agentics-contracts | ✅ CONFIRMED |
| Append-only persistence behavior | ✅ CONFIRMED |
| Idempotent writes and retry safety | ✅ CONFIRMED |

### Persistence Flow
```
Edge Agent → RuVectorClient → ruvector-service → Google SQL (Postgres)
```

### DecisionEvent Types Persisted
- `tool_invocation_decision`
- `circuit_breaker_decision`
- `failover_routing_decision`
- `execution_guard_decision`
- `cache_strategy_decision`

---

## 4. CLOUD BUILD & DEPLOYMENT

### Cloud Build Configuration
File: `deployments/gcp/cloudbuild.yaml`

### Deployment Commands

```bash
# Quick deploy (uses Cloud Build)
gcloud builds submit --config=deployments/gcp/cloudbuild.yaml .

# Or use the deploy script
./deployments/gcp/deploy.sh agentics-dev us-central1 prod
```

### Manual Deployment
```bash
# Build locally
docker build -t gcr.io/agentics-dev/llm-edge-agent:latest -f deployments/gcp/Dockerfile .

# Push to registry
docker push gcr.io/agentics-dev/llm-edge-agent:latest

# Deploy to Cloud Run
gcloud run deploy llm-edge-agent \
    --image gcr.io/agentics-dev/llm-edge-agent:latest \
    --region us-central1 \
    --platform managed \
    --port 8080 \
    --memory 512Mi \
    --cpu 1 \
    --min-instances 0 \
    --max-instances 10 \
    --set-env-vars "PLATFORM_ENV=prod,RUVECTOR_SERVICE_URL=https://ruvector-service-xxx-uc.a.run.app" \
    --service-account llm-edge-agent-sa@agentics-dev.iam.gserviceaccount.com \
    --ingress internal-and-cloud-load-balancing \
    --no-allow-unauthenticated
```

### IAM Service Account Requirements (Least Privilege)

```bash
# Run IAM setup
./deployments/gcp/setup-iam.sh agentics-dev us-central1
```

| Role | Purpose |
|------|---------|
| `roles/run.invoker` | Call internal Cloud Run services (ruvector, observatory) |
| `roles/secretmanager.secretAccessor` | Read secrets |
| `roles/logging.logWriter` | Write Cloud Logging |
| `roles/monitoring.metricWriter` | Write Cloud Monitoring metrics |

**NOT Granted (by design):**
- `roles/cloudsql.client` - NO direct SQL access
- `roles/storage.objectAdmin` - NO direct storage access

---

## 5. CLI ACTIVATION VERIFICATION

### CLI Commands per Agent

#### Tool Invocation Agent
```bash
# Test mode
curl -X POST ${SERVICE_URL}/agents/tool-invocation/test \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"request":{"tool":{"name":"test","parameters":[]},"arguments":{}}}'

# Simulate mode
curl -X POST ${SERVICE_URL}/agents/tool-invocation/simulate \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"tool_name":"calculator","params":{"a":1,"b":2}}'

# Inspect mode
curl ${SERVICE_URL}/agents/tool-invocation/inspect \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)"
```

#### Circuit Breaker Agent
```bash
# Test mode
curl -X POST ${SERVICE_URL}/agents/circuit-breaker/test \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"openai","simulate_failures":3}'

# Simulate mode
curl -X POST ${SERVICE_URL}/agents/circuit-breaker/simulate \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"provider_id":"openai","initial_state":"Closed","initial_failure_count":0,"initial_success_count":0}'

# Inspect mode
curl ${SERVICE_URL}/agents/circuit-breaker/inspect/openai \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)"
```

#### Failover Agent
```bash
# Test mode
curl -X POST ${SERVICE_URL}/agents/failover/failover/test \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"primary":"openai","alternates":["anthropic","google"]}'

# Simulate mode
curl -X POST ${SERVICE_URL}/agents/failover/failover/simulate \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"scenario":"primary-down"}'

# Inspect mode
curl ${SERVICE_URL}/agents/failover/failover/inspect \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)"
```

#### Execution Guard Agent
```bash
# Simulate mode
curl -X POST ${SERVICE_URL}/agents/execution-guard/simulate \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","input_tokens":1000,"max_output_tokens":2000}'

# Inspect mode
curl ${SERVICE_URL}/agents/execution-guard/inspect \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)"
```

#### Caching Strategy Agent
```bash
# Simulate mode
curl -X POST ${SERVICE_URL}/agents/caching-strategy/cache-strategy/simulate \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)" \
  -H "Content-Type: application/json" \
  -d '{"request_type":"chat_completion","provider_id":"openai","model_id":"gpt-4"}'

# Inspect mode
curl ${SERVICE_URL}/agents/caching-strategy/cache-strategy/inspect \
  -H "Authorization: Bearer $(gcloud auth print-identity-token)"
```

---

## 6. PLATFORM & CORE INTEGRATION

### Confirmed Invocations

| Upstream Service | Invokes Edge-Agent? | Purpose |
|------------------|---------------------|---------|
| LLM-Orchestrator | ✅ YES | During execution for routing, guard, cache decisions |
| LLM-Shield | ✅ MAY | For pre-execution security enforcement |
| LLM-Observatory | ✅ RECEIVES | Ingests Edge telemetry |
| LLM-Sentinel | ✅ MAY | Consumes Edge decision signals |
| Governance/Audit | ✅ RECEIVES | Consumes DecisionEvents |

### What Edge-Agent Does NOT Invoke (By Design)

| Target | Reason |
|--------|--------|
| Orchestrator logic | NOT an orchestrator |
| Sentinel detection logic | NOT a detector |
| Incident workflows | NOT an incident manager |
| Analytics pipelines | NOT an analytics system |
| Auto-optimization logic | NOT an optimizer |

---

## 7. POST-DEPLOY VERIFICATION CHECKLIST

Run the verification script:
```bash
./deployments/gcp/verify-deployment.sh https://llm-edge-agent-xxx-uc.a.run.app
```

### Manual Checklist

| Check | Command | Expected |
|-------|---------|----------|
| Service is live | `curl ${URL}/health` | `{"status":"healthy"}` |
| All agent endpoints respond | `curl ${URL}/agents/*/health` | 200 OK |
| Routing decisions work | `curl -X POST ${URL}/agents/failover/failover/simulate` | Decision returned |
| Guard decisions work | `curl -X POST ${URL}/agents/execution-guard/simulate` | Decision returned |
| Cache decisions work | `curl -X POST ${URL}/agents/caching-strategy/cache-strategy/simulate` | Decision returned |
| DecisionEvents in ruvector | Check ruvector-service logs | Events persisted |
| Telemetry in Observatory | Check Observatory dashboard | Metrics visible |
| CLI simulation works | See CLI commands above | All return 200 |
| No direct SQL access | Service account audit | No cloudsql.client role |

---

## 8. FAILURE MODES & ROLLBACK

### Common Deployment Failures

| Failure | Detection | Resolution |
|---------|-----------|------------|
| Build failure | Cloud Build logs show errors | Fix Cargo.toml or code errors |
| Image push failure | Push command fails | Re-authenticate: `gcloud auth configure-docker` |
| Service startup failure | Cloud Run logs show panic | Check RUST_LOG, verify env vars |
| Auth failure to ruvector | 403 errors in logs | Verify service account, API key |
| Missing secrets | Service fails with config error | Create secrets in Secret Manager |

### Rollback Procedure

```bash
# List previous revisions
gcloud run revisions list --service=llm-edge-agent --region=us-central1

# Rollback to previous revision
gcloud run services update-traffic llm-edge-agent \
    --region=us-central1 \
    --to-revisions=llm-edge-agent-PREVIOUS:100

# Or deploy known-good version
gcloud run deploy llm-edge-agent \
    --image gcr.io/agentics-dev/llm-edge-agent:known-good-tag \
    --region us-central1
```

### Safe Redeploy Strategy

1. **Blue-Green**: Deploy new revision with 0% traffic
2. **Canary**: Route 10% traffic to new revision
3. **Verify**: Run verification script against new revision
4. **Promote**: Route 100% traffic to new revision
5. **Cleanup**: Delete old revisions after validation

```bash
# Blue-green deploy
gcloud run deploy llm-edge-agent \
    --image gcr.io/agentics-dev/llm-edge-agent:new-tag \
    --no-traffic

# Canary (10%)
gcloud run services update-traffic llm-edge-agent \
    --to-revisions=llm-edge-agent-NEW:10,llm-edge-agent-OLD:90

# Promote (100%)
gcloud run services update-traffic llm-edge-agent \
    --to-latest
```

---

## Quick Reference

### Deploy
```bash
./deployments/gcp/deploy.sh agentics-dev us-central1 prod
```

### Verify
```bash
./deployments/gcp/verify-deployment.sh https://llm-edge-agent-xxx-uc.a.run.app
```

### Rollback
```bash
gcloud run services update-traffic llm-edge-agent --region=us-central1 --to-revisions=REVISION:100
```

### Logs
```bash
gcloud run logs tail llm-edge-agent --region=us-central1
```
