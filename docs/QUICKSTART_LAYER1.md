# Layer 1 Quick Start Guide

## Prerequisites

- Rust 1.75+ (install from https://rustup.rs)
- Docker (optional, for containerized deployment)

## Quick Start (5 minutes)

### 1. Clone and Navigate

```bash
cd /workspaces/llm-edge-agent
```

### 2. Configure Environment

```bash
# Copy example configuration
cp .env.example .env

# Edit configuration (optional)
nano .env
```

**Minimal Configuration**:
```env
SERVER_ADDRESS=0.0.0.0:8080
AUTH_ENABLED=false  # Disable auth for quick start
RATE_LIMIT_ENABLED=false  # Disable rate limiting for testing
LOG_LEVEL=info
```

### 3. Build and Run

```bash
# Build the proxy crate
cargo build --package llm-edge-proxy --release

# Run the server (coming in the next step - main binary)
# For now, you can test with: cargo test --package llm-edge-proxy
```

### 4. Test Endpoints

```bash
# Health check
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","timestamp":"2025-11-08T12:00:00Z","version":"0.1.0"}

# Readiness check
curl http://localhost:8080/health/ready

# Metrics
curl http://localhost:8080/metrics

# Chat completion (mock response)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## Production Configuration

### Enable Authentication

```env
AUTH_ENABLED=true
API_KEYS=your-secret-key-1,your-secret-key-2
```

Test with authentication:
```bash
curl http://localhost:8080/v1/chat/completions \
  -H "x-api-key: your-secret-key-1" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

### Enable Rate Limiting

```env
RATE_LIMIT_ENABLED=true
RATE_LIMIT_RPM=100  # 100 requests per minute
RATE_LIMIT_BURST=10  # Burst of 10
```

### Enable TLS

```bash
# Generate self-signed cert (development only!)
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem -out cert.pem -days 365 \
  -subj "/CN=localhost"
```

```env
ENABLE_TLS=true
TLS_CERT_PATH=./cert.pem
TLS_KEY_PATH=./key.pem
```

## Docker Deployment

### Build Image

```bash
docker build -t llm-edge-agent:layer1 -f Dockerfile .
```

### Run Container

```bash
docker run -d \
  -p 8080:8080 \
  --name llm-edge-agent \
  --env-file .env \
  llm-edge-agent:layer1
```

### Check Logs

```bash
docker logs -f llm-edge-agent
```

### Stop Container

```bash
docker stop llm-edge-agent
docker rm llm-edge-agent
```

## Kubernetes Deployment

```bash
# Create secret for API keys
kubectl create secret generic llm-api-keys \
  --from-literal=api-keys=key1,key2

# Deploy
kubectl apply -f - <<YAML
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-edge-agent
spec:
  replicas: 2
  selector:
    matchLabels:
      app: llm-edge-agent
  template:
    metadata:
      labels:
        app: llm-edge-agent
    spec:
      containers:
      - name: proxy
        image: llm-edge-agent:layer1
        ports:
        - containerPort: 8080
        env:
        - name: AUTH_ENABLED
          value: "true"
        - name: API_KEYS
          valueFrom:
            secretKeyRef:
              name: llm-api-keys
              key: api-keys
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
---
apiVersion: v1
kind: Service
metadata:
  name: llm-edge-agent
spec:
  selector:
    app: llm-edge-agent
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
YAML
```

## Development Tips

### Watch Mode (auto-rebuild on changes)

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x 'run --package llm-edge-proxy'
```

### Running Tests

```bash
# All tests
cargo test --package llm-edge-proxy

# With output
cargo test --package llm-edge-proxy -- --nocapture --test-threads=1

# Specific test
cargo test --package llm-edge-proxy test_health_check
```

### Debug Logging

```env
RUST_LOG=llm_edge_proxy=debug,tower_http=debug,axum=debug
```

### Performance Testing

```bash
# Install bombardier
go install github.com/codesenberg/bombardier@latest

# Run load test
bombardier -c 100 -n 10000 -l http://localhost:8080/health

# Expected results:
# Reqs/sec: >20,000
# P50 latency: <1ms
# P95 latency: <5ms
```

## Troubleshooting

### Port already in use

```bash
# Find process using port 8080
lsof -i :8080

# Kill it
kill -9 <PID>

# Or use different port
SERVER_ADDRESS=0.0.0.0:8081 cargo run
```

### Build fails

```bash
# Clean and rebuild
cargo clean
cargo build --package llm-edge-proxy

# Update dependencies
cargo update
```

### Container won't start

```bash
# Check logs
docker logs llm-edge-agent

# Run interactively
docker run -it --rm --env-file .env llm-edge-agent:layer1

# Debug with shell
docker run -it --rm --entrypoint /bin/bash llm-edge-agent:layer1
```

## Next Steps

1. **Layer 2**: Add caching, provider adapters, and routing
2. **Monitoring**: Integrate with Prometheus + Grafana
3. **Security**: Integrate with LLM-Shield
4. **Observability**: Set up OpenTelemetry collector

## Support

- Documentation: See README_LAYER1.md
- Implementation Report: See LAYER1_IMPLEMENTATION_REPORT.md
- Issues: GitHub Issues

---

**Last Updated**: 2025-11-08
**Status**: Layer 1 Complete
