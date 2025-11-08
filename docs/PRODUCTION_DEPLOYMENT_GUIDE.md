# LLM Edge Agent - Production Deployment Guide

**Version**: 1.0.0
**Date**: 2025-11-08
**Status**: Production Ready

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [System Requirements](#system-requirements)
3. [Deployment Options](#deployment-options)
4. [Configuration](#configuration)
5. [Security Hardening](#security-hardening)
6. [Monitoring & Observability](#monitoring--observability)
7. [Performance Tuning](#performance-tuning)
8. [Troubleshooting](#troubleshooting)
9. [Maintenance](#maintenance)

---

## Prerequisites

### Required Software

- **Rust**: 1.75+ (for building)
- **Docker**: 20.10+ (for containerized deployment)
- **Kubernetes**: 1.27+ (for Kubernetes deployment)
- **Redis**: 7.0+ (for L2 caching)

### Required Accounts & API Keys

At least one LLM provider is required:

- **OpenAI**: API key from https://platform.openai.com/api-keys
- **Anthropic**: API key from https://console.anthropic.com/

### Optional Services

- **Prometheus**: Metrics collection (port 9090)
- **Grafana**: Metrics visualization (port 3000)
- **Jaeger**: Distributed tracing (port 16686)

---

## System Requirements

### Minimum Resources (Development)

- **CPU**: 2 cores
- **RAM**: 4 GB
- **Disk**: 10 GB
- **Network**: 100 Mbps

### Recommended Resources (Production)

- **CPU**: 4+ cores
- **RAM**: 8 GB
- **Disk**: 50 GB SSD
- **Network**: 1 Gbps

### High Availability (Enterprise)

- **CPU**: 8+ cores per node
- **RAM**: 16 GB per node
- **Disk**: 100 GB SSD per node
- **Nodes**: 3+ (for redundancy)
- **Network**: 10 Gbps

---

## Deployment Options

### Option 1: Standalone Binary

**Best for**: Development, testing, small deployments

```bash
# 1. Build release binary
cargo build --release

# 2. Set environment variables
export OPENAI_API_KEY="sk-..."
export REDIS_URL="redis://localhost:6379"
export HOST="0.0.0.0"
export PORT="8080"

# 3. Run
./target/release/llm-edge-agent
```

**Pros**: Simple, fast startup
**Cons**: Single point of failure, manual scaling

---

### Option 2: Docker Container

**Best for**: Consistent environments, easy scaling

#### Build Docker Image

```bash
# Multi-stage build for minimal image size
docker build -t llm-edge-agent:latest .
```

#### Run with Docker

```bash
docker run -d \
  --name llm-edge-agent \
  -p 8080:8080 \
  -p 9090:9090 \
  -e OPENAI_API_KEY="sk-..." \
  -e REDIS_URL="redis://redis:6379" \
  llm-edge-agent:latest
```

#### Docker Compose (Recommended)

```yaml
# docker-compose.yml
version: '3.8'

services:
  llm-edge-agent:
    image: llm-edge-agent:latest
    ports:
      - "8080:8080"  # HTTP server
      - "9090:9090"  # Metrics
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - REDIS_URL=redis://redis:6379
      - ENABLE_L2_CACHE=true
      - RUST_LOG=info
    depends_on:
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
    restart: unless-stopped

volumes:
  redis-data:
  prometheus-data:
  grafana-data:
```

**Start the stack**:
```bash
docker-compose up -d
```

---

### Option 3: Kubernetes Deployment

**Best for**: Production, high availability, auto-scaling

#### Deployment Manifest

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-edge-agent
  namespace: llm-edge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-edge-agent
  template:
    metadata:
      labels:
        app: llm-edge-agent
    spec:
      containers:
      - name: llm-edge-agent
        image: llm-edge-agent:1.0.0
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: llm-edge-secrets
              key: openai-api-key
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: HOST
          value: "0.0.0.0"
        - name: PORT
          value: "8080"
        resources:
          requests:
            memory: "2Gi"
            cpu: "1"
          limits:
            memory: "4Gi"
            cpu: "2"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: llm-edge-agent
  namespace: llm-edge
spec:
  type: ClusterIP
  selector:
    app: llm-edge-agent
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
```

#### Create Secrets

```bash
kubectl create secret generic llm-edge-secrets \
  --from-literal=openai-api-key="sk-..." \
  --from-literal=anthropic-api-key="sk-ant-..." \
  -n llm-edge
```

#### Deploy

```bash
kubectl apply -f k8s/deployment.yaml
```

#### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: llm-edge-agent-hpa
  namespace: llm-edge
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: llm-edge-agent
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

---

## Configuration

### Environment Variables

#### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `OPENAI_API_KEY` | OpenAI API key (if using OpenAI) | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API key (if using Anthropic) | `sk-ant-...` |

#### Server Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HOST` | Server bind address | `0.0.0.0` |
| `PORT` | Server HTTP port | `8080` |
| `METRICS_PORT` | Prometheus metrics port | `9090` |

#### Cache Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `ENABLE_L2_CACHE` | Enable Redis L2 cache | `true` |
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |
| `L1_CACHE_SIZE` | L1 cache max entries | `1000` |
| `L1_TTL_SECONDS` | L1 cache TTL | `300` (5min) |
| `L2_TTL_SECONDS` | L2 cache TTL | `3600` (1hr) |

#### Observability

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `ENABLE_TRACING` | Enable OpenTelemetry tracing | `true` |
| `OTLP_ENDPOINT` | OpenTelemetry collector endpoint | `http://jaeger:4317` |

---

## Security Hardening

### TLS/HTTPS

Enable TLS in production:

```bash
# Set TLS certificate paths
export TLS_CERT_PATH="/etc/ssl/certs/server.crt"
export TLS_KEY_PATH="/etc/ssl/private/server.key"
```

### API Key Management

**DO NOT** hardcode API keys. Use:

1. **Environment variables** (development)
2. **Kubernetes Secrets** (Kubernetes)
3. **AWS Secrets Manager** (AWS)
4. **HashiCorp Vault** (enterprise)

### Network Policies (Kubernetes)

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: llm-edge-agent-netpol
  namespace: llm-edge
spec:
  podSelector:
    matchLabels:
      app: llm-edge-agent
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - namespaceSelector: {}
    ports:
    - protocol: TCP
      port: 443  # HTTPS to LLM providers
    - protocol: TCP
      port: 6379 # Redis
```

### Rate Limiting

Configure per-client rate limits:

```yaml
# config/rate-limits.yaml
rate_limiting:
  global:
    requests_per_second: 10000
  per_client:
    tier_free: 10
    tier_pro: 100
    tier_enterprise: 1000
```

---

## Monitoring & Observability

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'llm-edge-agent'
    static_configs:
      - targets: ['llm-edge-agent:9090']
```

### Key Metrics to Monitor

| Metric | Alert Threshold | Description |
|--------|----------------|-------------|
| `llm_edge_requests_total` | N/A | Total requests |
| `llm_edge_cache_hit_rate` | <60% | Cache performance |
| `llm_edge_request_duration_seconds` | P95 >2s | Latency |
| `llm_edge_provider_errors_total` | >1% | Provider health |
| `llm_edge_cost_total_cents` | >budget | Cost tracking |

### Grafana Dashboards

Import the pre-built dashboards from `/dashboards/`:

1. Request Overview
2. Cache Performance
3. Cost Analytics
4. System Health

### Jaeger Tracing

Enable distributed tracing:

```bash
# Start Jaeger all-in-one
docker run -d \
  --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

Access UI: http://localhost:16686

---

## Performance Tuning

### Rust Compilation Optimizations

Already configured in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
```

### Redis Tuning

```conf
# redis.conf
maxmemory 2gb
maxmemory-policy allkeys-lru
save ""  # Disable RDB snapshots for pure cache
appendonly yes
```

### Tokio Runtime Tuning

Set worker threads based on CPU cores:

```bash
export TOKIO_WORKER_THREADS=8  # 2x CPU cores
```

### Connection Pooling

Configured in code:
- HTTP client pool: 20 max idle connections
- Redis pool: 20 connections
- Keep-alive: 90 seconds

---

## Troubleshooting

### Common Issues

#### 1. "Connection refused" to Redis

**Symptom**: L2 cache failures, warnings in logs

**Solution**:
```bash
# Check Redis is running
docker ps | grep redis

# Check connectivity
redis-cli -h localhost -p 6379 ping

# Verify REDIS_URL environment variable
echo $REDIS_URL
```

#### 2. High memory usage

**Symptom**: OOM kills, slow performance

**Solution**:
```bash
# Reduce L1 cache size
export L1_CACHE_SIZE=500

# Monitor memory
watch -n 1 'ps aux | grep llm-edge-agent'
```

#### 3. Slow response times

**Symptom**: P95 latency >2s

**Solution**:
```bash
# Check cache hit rate
curl http://localhost:9090/metrics | grep cache_hit

# Check provider latency
curl http://localhost:9090/metrics | grep provider_duration

# Enable debug logging
export RUST_LOG=debug
```

#### 4. Provider API errors

**Symptom**: 502 Bad Gateway, provider timeouts

**Solution**:
```bash
# Verify API keys
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Check circuit breaker state
curl http://localhost:9090/metrics | grep circuit_breaker_state
```

---

## Maintenance

### Health Checks

```bash
# General health
curl http://localhost:8080/health

# Readiness probe (Kubernetes)
curl http://localhost:8080/health/ready

# Liveness probe (Kubernetes)
curl http://localhost:8080/health/live
```

### Log Rotation

Configure logrotate (if using file logging):

```conf
/var/log/llm-edge-agent/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0644 llm llm
    sharedscripts
    postrotate
        killall -USR1 llm-edge-agent
    endscript
}
```

### Backup & Recovery

#### Redis Backup

```bash
# Manual backup
redis-cli SAVE

# Copy RDB file
cp /var/lib/redis/dump.rdb /backups/redis-$(date +%Y%m%d).rdb
```

#### Configuration Backup

```bash
# Backup environment and configs
tar -czf llm-edge-backup-$(date +%Y%m%d).tar.gz \
  .env config/ k8s/
```

### Upgrades

1. **Backup current version**
2. **Test in staging environment**
3. **Rolling update (Kubernetes)**:
   ```bash
   kubectl set image deployment/llm-edge-agent \
     llm-edge-agent=llm-edge-agent:1.1.0
   ```
4. **Monitor metrics during rollout**
5. **Rollback if issues**:
   ```bash
   kubectl rollout undo deployment/llm-edge-agent
   ```

---

## Production Checklist

Before going live, verify:

- [ ] API keys configured securely (not hardcoded)
- [ ] TLS/HTTPS enabled
- [ ] Redis cluster deployed with persistence
- [ ] Prometheus + Grafana monitoring active
- [ ] Alerting configured (PagerDuty, Slack, etc.)
- [ ] Log aggregation setup (ELK, Loki, CloudWatch)
- [ ] Backup strategy defined
- [ ] Disaster recovery plan documented
- [ ] Load testing completed (>1000 req/s sustained)
- [ ] Security audit passed
- [ ] Documentation updated
- [ ] Team trained on operations

---

## Support & Resources

- **Documentation**: `/docs/`
- **GitHub Issues**: https://github.com/yourusername/llm-edge-agent/issues
- **Slack**: #llm-edge-agent
- **Email**: support@example.com

---

**Last Updated**: 2025-11-08
**Document Version**: 1.0.0
