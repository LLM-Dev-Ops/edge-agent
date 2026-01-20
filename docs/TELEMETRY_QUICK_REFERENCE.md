# OpenTelemetry & Tracing Dependency Quick Reference

**Scan Date:** December 4, 2025  
**Status:** Complete  
**Repositories Analyzed:** 6 (1 direct, 5 indirect)

---

## EDGE-AGENT DIRECT DEPENDENCIES

### Workspace-Level (Root Cargo.toml, lines 83-88)

| Crate | Version | Features | Type | Exporter |
|-------|---------|----------|------|----------|
| opentelemetry | 0.26 | - | Direct | OTLP |
| opentelemetry-otlp | 0.26 | trace | Direct | OTLP (gRPC) |
| tracing | 0.1 | - | Direct | Structured logs |
| tracing-subscriber | 0.3 | env-filter, json | Direct | JSON formatter |
| metrics | 0.23 | - | Direct | Metrics collection |
| metrics-exporter-prometheus | 0.15 | - | Direct | Prometheus |

### Transitive Dependencies (via Cargo.lock)

| Crate | Version | Source |
|-------|---------|--------|
| opentelemetry-proto | 0.26.1 | opentelemetry-otlp |
| opentelemetry_sdk | 0.26.0 | opentelemetry-otlp |
| tracing-attributes | 0.1.30 | tracing |
| tracing-core | 0.1.34 | tracing, tracing-subscriber |
| tracing-log | 0.2.0 | tracing-subscriber |
| tracing-serde | 0.2.0 | tracing-subscriber |
| tonic | 0.12.3 | opentelemetry-otlp |
| prost | 0.13.5 | tonic |

---

## CRATE-LEVEL USAGE IN EDGE-AGENT

| Crate | Imports | Exported |
|-------|---------|----------|
| llm-edge-monitoring | opentelemetry, opentelemetry-otlp, tracing, metrics, prometheus | YES |
| llm-edge-proxy | opentelemetry, tracing, tracing-subscriber | via monitoring |
| llm-edge-providers | opentelemetry, tracing | NO |
| llm-edge-agent (main) | tracing, tracing-subscriber, metrics, prometheus | YES |
| llm-edge-routing | tracing, metrics | NO |
| llm-edge-cache | tracing, metrics | NO |
| llm-edge-security | tracing | NO |

---

## UPSTREAM REPOSITORIES TELEMETRY

### Observatory
- **URL:** https://github.com/LLM-Dev-Ops/observatory
- **Status:** Workspace (10 crates)
- **Key:** opentelemetry 0.27, metrics 0.24, prometheus 0.16

### Sentinel
- **URL:** https://github.com/LLM-Dev-Ops/sentinel
- **Status:** Workspace (6 crates)
- **Key:** opentelemetry 0.27, tonic 0.12, prost 0.13

### Shield
- **URL:** https://github.com/LLM-Dev-Ops/shield
- **Status:** Workspace (15 crates)
- **Key:** tracing 0.1, tracing-subscriber 0.3 (no OTLP)

### CostOps
- **URL:** https://github.com/LLM-Dev-Ops/cost-ops
- **Status:** Workspace (5 crates)
- **Key:** tracing 0.1, metrics 0.21, prometheus 0.12 (no OTLP)

### Policy-Engine
- **URL:** https://github.com/LLM-Dev-Ops/policy-engine
- **Status:** Single crate
- **Key:** opentelemetry 0.21 (rt-tokio), opentelemetry-jaeger 0.20, tonic 0.11, prost 0.12
- **CRITICAL:** Version conflict with Edge-Agent (0.21 vs 0.26)

### Connector-Hub
- **URL:** https://github.com/LLM-Dev-Ops/connector-hub
- **Status:** Monorepo (Rust + TypeScript)
- **Key:** tracing 0.1, tracing-subscriber 0.3 (no OTLP)

---

## VERSION COMPATIBILITY MATRIX

```
                Edge-Agent  Observatory  Sentinel  Policy-Engine  CostOps  Shield
opentelemetry    0.26       0.27         0.27      0.21*         -        -
tracing          0.1        0.1          0.1       0.1            0.1      0.1
tracing-sub      0.3        0.3          -         -              0.3      0.3
metrics          0.23       0.24         -         0.21           0.21     -
prometheus-exp   0.15       0.16         -         0.13           0.12     -
tonic            -          0.12         0.12      0.11           -        -
prost            -          0.13         0.13      0.12           -        -

* CONFLICT: Policy-Engine uses 0.21, Edge-Agent uses 0.26
```

---

## EXPORTERS ENABLED

### Edge-Agent
- **OTLP:** opentelemetry-otlp 0.26 (trace only, via gRPC)
- **Prometheus:** metrics-exporter-prometheus 0.15 (HTTP pull)
- **Missing:** Jaeger, Zipkin, Datadog

### Observatory
- **OTLP:** opentelemetry 0.27
- **Prometheus:** metrics-exporter-prometheus 0.16

### Sentinel
- **OTLP:** opentelemetry 0.27
- **Infrastructure:** Kafka, RabbitMQ, InfluxDB

### Policy-Engine
- **Jaeger:** opentelemetry-jaeger 0.20
- **Prometheus:** metrics-exporter-prometheus 0.13

### CostOps
- **Prometheus:** metrics-exporter-prometheus 0.12

### Shield
- **Tracing only:** No OTLP export (log-based)

---

## SIGNALS SUPPORT

| Repo | Traces | Metrics | Logs | Status |
|------|--------|---------|------|--------|
| Edge-Agent | YES | YES | PARTIAL | Active |
| Observatory | YES | YES | UNKNOWN | Production |
| Sentinel | YES | UNKNOWN | UNKNOWN | Production |
| Policy-Engine | YES (Jaeger) | YES | UNKNOWN | Production |
| CostOps | UNKNOWN | YES | UNKNOWN | Production |
| Shield | TRACING_ONLY | NO | YES | Limited |

---

## KEY ISSUES & RECOMMENDATIONS

### Critical Issues
1. **Policy-Engine Version Conflict**
   - Uses opentelemetry 0.21 with rt-tokio feature
   - Edge-Agent uses 0.26
   - Recommendation: Upgrade Policy-Engine to 0.26

2. **Missing Jaeger Support in Edge-Agent**
   - Policy-Engine exports to Jaeger
   - Edge-Agent needs opentelemetry-jaeger 0.26 for compatibility

### Feature Gaps
1. **OTLP incomplete signal types**
   - Only trace feature enabled
   - Should enable: logs, metrics

2. **Shield has no OTLP export**
   - Uses tracing/tracing-subscriber only
   - Consider adding opentelemetry support

---

## FILE LOCATIONS

- **Full Inventory:** `/workspaces/edge-agent/TELEMETRY_METADATA_INVENTORY.md` (1044 lines)
- **Workspace Config:** `/workspaces/edge-agent/Cargo.toml` (lines 83-88)
- **Monitoring Crate:** `/workspaces/edge-agent/crates/llm-edge-monitoring/Cargo.toml`
- **Lock File:** `/workspaces/edge-agent/Cargo.lock`

---

**Last Updated:** December 4, 2025  
**Scanner:** TelemetryMetadataScanner v1.0
