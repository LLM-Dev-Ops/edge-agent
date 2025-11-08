# LLM-Edge-Agent Quick Start Guide

This guide will help you get LLM-Edge-Agent up and running in under 10 minutes.

## Prerequisites

- Docker and Docker Compose installed
- Node.js 18+ (for development)
- API keys for LLM providers (OpenAI, Anthropic, etc.)

## Quick Start: Standalone Deployment

### 1. Clone and Setup

```bash
git clone https://github.com/your-org/llm-edge-agent.git
cd llm-edge-agent
```

### 2. Configure Environment Variables

Create a `.env` file in the root directory:

```bash
cat > .env <<EOF
# Provider API Keys
OPENAI_API_KEY=sk-proj-your-key-here
ANTHROPIC_API_KEY=sk-ant-your-key-here
AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com
AZURE_OPENAI_API_KEY=your-azure-key-here

# Redis (optional, uses default if not set)
REDIS_PASSWORD=your-redis-password

# Authentication
ADMIN_API_KEY=admin-$(openssl rand -hex 16)
CLIENT_API_KEY=client-$(openssl rand -hex 16)

# Monitoring (optional)
GRAFANA_PASSWORD=your-grafana-password
EOF
```

### 3. Start Services

```bash
cd deployments/standalone/docker
docker-compose up -d
```

This will start:
- LLM-Edge-Agent proxy on port 8080
- Redis for caching on port 6379
- Prometheus on port 9091
- Grafana on port 3000
- Jaeger UI on port 16686

### 4. Verify Installation

```bash
# Health check
curl http://localhost:8080/health

# Should return: {"status":"healthy"}
```

### 5. Make Your First Request

```bash
# Using OpenAI-compatible API
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${CLIENT_API_KEY}" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "Hello! What is LLM-Edge-Agent?"
      }
    ]
  }'
```

### 6. View Metrics and Logs

**Prometheus Metrics:**
```bash
open http://localhost:9091
```

**Grafana Dashboard:**
```bash
open http://localhost:3000
# Login: admin / <GRAFANA_PASSWORD from .env>
```

**Jaeger Tracing:**
```bash
open http://localhost:16686
```

**View Logs:**
```bash
docker-compose logs -f llm-edge-agent
```

## Quick Start: Kubernetes Sidecar

### 1. Create Namespace and Secrets

```bash
# Create namespace
kubectl create namespace llm-edge-agent

# Create secrets
kubectl create secret generic llm-api-keys \
  --from-literal=openai=${OPENAI_API_KEY} \
  --from-literal=anthropic=${ANTHROPIC_API_KEY} \
  --from-literal=azure=${AZURE_OPENAI_API_KEY} \
  --from-literal=admin-api-key=${ADMIN_API_KEY} \
  -n llm-edge-agent
```

### 2. Deploy

```bash
cd deployments/sidecar/kubernetes
kubectl apply -f deployment.yaml
```

### 3. Verify

```bash
# Check pod status
kubectl get pods -n llm-edge-agent

# Check logs
kubectl logs -n llm-edge-agent -l app=myapp -c llm-edge-agent-sidecar -f
```

## Testing Cache Performance

```bash
# First request (cache miss)
time curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${CLIENT_API_KEY}" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }'

# Second identical request (cache hit - should be much faster)
time curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${CLIENT_API_KEY}" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }'
```

## Common Configuration Options

### Enable Semantic Caching

Edit `deployments/standalone/config/production.yaml`:

```yaml
cache:
  enabled: true
  primary:
    strategy: "semantic"
    semantic:
      enabled: true
      similarityThreshold: 0.95
```

### Change Routing Strategy

```yaml
routing:
  strategy: "cost-optimized"  # or "least-latency", "round-robin"
```

### Adjust Rate Limits

```yaml
security:
  rateLimiting:
    enabled: true
    perClient:
      windowMs: 60000
      maxRequests: 200  # Increase from default 100
```

## Integration with Your Application

### Python

```python
import openai

# Configure OpenAI client to use LLM-Edge-Agent
openai.api_base = "http://localhost:8080/v1"
openai.api_key = "your-client-api-key"

response = openai.ChatCompletion.create(
    model="gpt-3.5-turbo",
    messages=[
        {"role": "user", "content": "Hello!"}
    ]
)
print(response)
```

### Node.js

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  baseURL: 'http://localhost:8080/v1',
  apiKey: process.env.CLIENT_API_KEY,
});

const response = await openai.chat.completions.create({
  model: 'gpt-3.5-turbo',
  messages: [{ role: 'user', content: 'Hello!' }],
});

console.log(response);
```

### cURL

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${CLIENT_API_KEY}" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## Monitoring Your Deployment

### Key Metrics to Watch

1. **Request Rate**: `rate(llm_edge_agent_requests_total[5m])`
2. **Cache Hit Rate**:
   ```
   sum(rate(llm_edge_agent_cache_hits_total[5m])) /
   (sum(rate(llm_edge_agent_cache_hits_total[5m])) +
    sum(rate(llm_edge_agent_cache_misses_total[5m])))
   ```
3. **P95 Latency**: `histogram_quantile(0.95, rate(llm_edge_agent_request_duration_seconds_bucket[5m]))`
4. **Error Rate**: `rate(llm_edge_agent_errors_total[5m])`
5. **Cost per Day**: `sum(llm_edge_agent_estimated_cost_usd)`

### Pre-built Grafana Dashboards

Import the included dashboards:

```bash
# Navigate to Grafana (http://localhost:3000)
# Go to Dashboards > Import
# Upload: deployments/standalone/grafana/dashboards/llm-edge-agent-overview.json
```

## Troubleshooting

### Proxy not responding

```bash
# Check if service is running
docker-compose ps

# Check logs
docker-compose logs llm-edge-agent

# Restart service
docker-compose restart llm-edge-agent
```

### Cache not working

```bash
# Check Redis connection
docker-compose exec llm-edge-agent redis-cli -h redis ping

# View cache stats
curl http://localhost:9090/metrics | grep cache
```

### High latency

```bash
# Check provider latency
curl http://localhost:9090/metrics | grep provider_latency

# Enable debug logging
docker-compose exec llm-edge-agent \
  sed -i 's/LOG_LEVEL=info/LOG_LEVEL=debug/' /etc/environment
docker-compose restart llm-edge-agent
```

### Authentication errors

```bash
# Verify API key
echo $CLIENT_API_KEY

# Test with correct header
curl -H "X-API-Key: ${CLIENT_API_KEY}" http://localhost:8080/health
```

## Next Steps

1. **Read the full documentation**: See `DEPLOYMENT_AND_ROADMAP.md`
2. **Configure for production**: Update `deployments/standalone/config/production.yaml`
3. **Set up monitoring**: Configure alerts in Prometheus/Grafana
4. **Enable semantic caching**: Improve cache hit rates with semantic matching
5. **Scale horizontally**: Add more proxy instances with load balancer

## Getting Help

- **Documentation**: `/docs` directory
- **Issues**: GitHub Issues
- **Community**: Discord/Slack channel
- **Support**: support@llm-edge-agent.com

## Quick Reference

### Docker Compose Commands

```bash
# Start all services
docker-compose up -d

# Stop all services
docker-compose down

# View logs
docker-compose logs -f [service-name]

# Restart a service
docker-compose restart [service-name]

# Update to latest version
docker-compose pull
docker-compose up -d
```

### Kubernetes Commands

```bash
# Deploy
kubectl apply -f deployment.yaml

# Check status
kubectl get pods -n llm-edge-agent

# View logs
kubectl logs -n llm-edge-agent <pod-name> -c llm-edge-agent-sidecar

# Scale
kubectl scale deployment myapp-with-llm-proxy --replicas=5 -n llm-edge-agent

# Delete
kubectl delete -f deployment.yaml
```

### Health Check Endpoints

- **Health**: `GET /health` - Overall health status
- **Ready**: `GET /ready` - Readiness for traffic
- **Metrics**: `GET /metrics` - Prometheus metrics

### Default Ports

- **8080**: Proxy HTTP server
- **9090**: Metrics endpoint
- **6379**: Redis cache
- **3000**: Grafana dashboard
- **9091**: Prometheus UI
- **16686**: Jaeger tracing UI

---

**Happy proxying!** 🚀
