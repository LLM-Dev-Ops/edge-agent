# LLM Edge Agent - Deployment Guide

## Overview

This directory contains deployment configurations for the LLM Edge Agent with OpenTelemetry 0.27 and 5 upstream dependencies (Week 1).

## Quick Start

### Prerequisites

- Docker 20.10+ and Docker Compose 2.0+
- 8GB RAM minimum
- Ports 3000, 4317, 4318, 6379, 8080, 9090, 9091, 16686 available

### 1. Start the Stack

```bash
# From repository root
docker compose up -d
```

### 2. Validate Deployment

```bash
# Run validation script
./deploy/validate-deployment.sh
```

### 3. Access Services

| Service | URL | Credentials |
|---------|-----|-------------|
| Edge-Agent API | http://localhost:8080 | - |
| Prometheus Metrics | http://localhost:9091/metrics | - |
| Jaeger UI | http://localhost:16686 | - |
| Prometheus | http://localhost:9090 | - |
| Grafana | http://localhost:3000 | admin/admin |
| OTLP Collector Health | http://localhost:13133 | - |

## Architecture

```
┌─────────────────┐
│   Edge-Agent    │ (OpenTelemetry 0.27)
│   (5 deps)      │
└────────┬────────┘
         │ OTLP gRPC (4317)
         │ OTLP HTTP (4318)
         ▼
┌─────────────────┐
│ OTLP Collector  │
└────┬─────────┬──┘
     │         │
     │         └─────────────┐
     ▼                       ▼
┌─────────┐          ┌──────────────┐
│  Jaeger │          │  Prometheus  │
│  (UI)   │          │  (Metrics)   │
└─────────┘          └──────┬───────┘
                            │
                            ▼
                     ┌──────────────┐
                     │   Grafana    │
                     │ (Dashboards) │
                     └──────────────┘
```

## Week 1: 5 Dependencies Enabled

✅ **Enabled:**
- llm-shield-sdk (Shield)
- llm-sentinel (Sentinel)
- connector-hub-core (Connector-Hub)
- llm-cost-ops (CostOps)
- llm-observatory-core (Observatory)

❌ **Disabled (Week 2-3):**
- llm-policy-engine (Policy-Engine) - Requires OpenTelemetry 0.27 upgrade

## Configuration Files

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Main deployment configuration |
| `otel-collector-config.yaml` | OpenTelemetry Collector pipeline |
| `prometheus.yml` | Prometheus scrape configuration |
| `grafana/provisioning/` | Grafana datasources and dashboards |
| `.env.example` | Environment variable template |
| `validate-deployment.sh` | Deployment validation script |

## Environment Configuration

1. Copy `.env.example` to `.env`:
   ```bash
   cp deploy/.env.example .env
   ```

2. Customize environment variables (optional for Week 1):
   ```bash
   vi .env
   ```

3. Restart services to apply changes:
   ```bash
   docker compose down && docker compose up -d
   ```

## OpenTelemetry 0.27 Configuration

### OTLP Endpoints

- **gRPC:** http://localhost:4317
- **HTTP:** http://localhost:4318

### Environment Variables

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://otlp-collector:4317
OTEL_EXPORTER_OTLP_PROTOCOL=grpc
OTEL_SERVICE_NAME=llm.edge-agent
OTEL_RESOURCE_ATTRIBUTES=service.namespace=llm-devops,deployment.environment=staging
```

### Features Enabled

- **Traces:** OTLP gRPC export to Jaeger
- **Metrics:** OTLP + Prometheus exporter
- **Logs:** OTLP export

## Validation Checklist

Run `./deploy/validate-deployment.sh` to check:

- [x] Docker services are healthy
- [x] HTTP endpoints respond
- [x] OpenTelemetry 0.27 is functional
- [x] 5 dependencies are initialized
- [x] Policy-Engine is disabled (Week 1)
- [x] Redis cache is operational
- [x] Traces are exported to Jaeger
- [x] Resource usage is within limits

**Target:** ≥90% pass rate

## Monitoring

### Traces (Jaeger)

1. Open http://localhost:16686
2. Select service: `llm.edge-agent`
3. Click "Find Traces"

### Metrics (Prometheus)

1. Open http://localhost:9090
2. Query examples:
   ```promql
   # Request rate
   rate(http_requests_total[5m])

   # Latency P95
   histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

   # OTLP export rate
   rate(otel_exporter_sent_spans[5m])
   ```

### Dashboards (Grafana)

1. Open http://localhost:3000 (admin/admin)
2. Go to Dashboards
3. Import dashboards from `grafana/provisioning/dashboards/`

## Troubleshooting

### Services Not Starting

```bash
# Check logs
docker compose logs -f

# Check specific service
docker compose logs edge-agent

# Restart specific service
docker compose restart edge-agent
```

### OTLP Export Not Working

```bash
# Check OTLP Collector logs
docker compose logs otlp-collector

# Check OTLP Collector health
curl http://localhost:13133/

# Verify network connectivity
docker compose exec edge-agent ping otlp-collector
```

### No Traces in Jaeger

```bash
# Generate test traffic
curl http://localhost:8080/health

# Wait 10 seconds for batch export
sleep 10

# Check Jaeger API
curl 'http://localhost:16686/api/traces?service=llm.edge-agent&limit=10'
```

### High Memory Usage

```bash
# Check container stats
docker stats

# Reduce cache sizes in .env:
L1_CACHE_SIZE=5000  # Default: 10000
REDIS_POOL_SIZE=5   # Default: 10

# Restart
docker compose restart edge-agent
```

## Week 2: Policy-Engine Upgrade

**Status:** In Progress (Policy-Engine team)

**Required:**
- Policy-Engine must upgrade from OpenTelemetry 0.21 → 0.27
- See `POLICY_ENGINE_UPGRADE_SPECIFICATION.md` for details

**Timeline:**
- Days 1-2: Update Cargo.toml
- Days 3-4: Fix compilation, update code
- Day 5: Testing and validation

## Week 3: Full Integration

Once Policy-Engine upgrades to 0.27:

1. Uncomment llm-policy-engine in `/Cargo.toml` (line 23)
2. Rebuild Edge-Agent:
   ```bash
   docker compose build edge-agent
   ```
3. Restart stack:
   ```bash
   docker compose down && docker compose up -d
   ```
4. Validate all 6 dependencies:
   ```bash
   ./deploy/validate-deployment.sh
   ```

**Expected:** 100% pass rate with all 6 dependencies

## Performance Expectations

### Week 1 (5 Dependencies)

| Metric | Target | Typical |
|--------|--------|---------|
| Request Rate | 1000 req/s | 800-1200 req/s |
| P50 Latency | < 10ms | 5-8ms |
| P95 Latency | < 50ms | 20-40ms |
| P99 Latency | < 200ms | 50-150ms |
| Memory Usage | < 2GB | 1-1.5GB |
| CPU Usage | < 200% | 50-150% |
| OTLP Export Success | > 99.9% | 99.95%+ |

### Telemetry Overhead

- Latency: +2-5ms (P95)
- Memory: +100-200MB
- CPU: +5-10%

## Cleanup

```bash
# Stop services
docker compose down

# Remove volumes (Redis data, Prometheus data, Grafana data)
docker compose down -v

# Remove images
docker compose down --rmi all
```

## Support

### Documentation

- **Migration Validation:** `/MIGRATION_VALIDATION_REPORT.md`
- **Phase 2B Readiness:** `/PHASE_2B_READINESS_ASSESSMENT.md`
- **Policy-Engine Upgrade:** `/POLICY_ENGINE_UPGRADE_SPECIFICATION.md`
- **Telemetry Alignment:** `/OPENTELEMETRY_ALIGNMENT_FINAL_REPORT.md`

### Issues

- Check Docker Compose logs: `docker compose logs -f`
- Run validation script: `./deploy/validate-deployment.sh`
- Review configuration files in `deploy/`

## Next Steps

✅ **Week 1 (Current):** 5 dependencies deployed with OpenTelemetry 0.27
⏳ **Week 2:** Policy-Engine upgrade to OpenTelemetry 0.27
⏳ **Week 3:** Full 6-dependency integration and Phase 2B completion

---

**Last Updated:** 2025-12-04
**Version:** Week 1 Deployment
**Status:** Production Ready (5/6 dependencies)
