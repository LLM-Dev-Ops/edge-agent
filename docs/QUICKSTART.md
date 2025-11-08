# LLM Edge Agent - Quick Start Guide

Get started with LLM Edge Agent in under 10 minutes!

## Prerequisites

- Rust 1.75 or higher
- Docker and Docker Compose (for dependencies)
- API keys for at least one LLM provider (OpenAI or Anthropic)

## Quick Start Options

### Option 1: Local Development (Rust)

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/llm-edge-agent.git
   cd llm-edge-agent
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env and add your API keys
   ```

3. **Start Redis (optional, for L2 cache)**
   ```bash
   docker run -d -p 6379:6379 redis:7-alpine
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run --release
   ```

5. **Test the proxy**
   ```bash
   curl http://localhost:8080/health
   # Should return: "ok"
   ```

### Option 2: Docker Compose (Recommended for Testing)

1. **Clone and configure**
   ```bash
   git clone https://github.com/yourusername/llm-edge-agent.git
   cd llm-edge-agent
   cp .env.example .env
   # Edit .env with your API keys
   ```

2. **Start the stack**
   ```bash
   cd deployments/standalone/docker
   docker-compose up -d
   ```

3. **Access the services**
   - Proxy: http://localhost:8080
   - Prometheus: http://localhost:9091
   - Grafana: http://localhost:3000 (admin/admin)

### Option 3: Kubernetes Sidecar

See [deployments/sidecar/kubernetes/deployment.yaml](deployments/sidecar/kubernetes/deployment.yaml) for complete manifests.

```bash
kubectl apply -f deployments/sidecar/kubernetes/deployment.yaml
```

## Making Your First Request

### OpenAI-compatible endpoint

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "user", "content": "Hello, world!"}
    ]
  }'
```

### Using with OpenAI Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="your-api-key"  # LLM Edge Agent API key
)

response = client.chat.completions.create(
    model="gpt-3.5-turbo",
    messages=[
        {"role": "user", "content": "Hello, world!"}
    ]
)

print(response.choices[0].message.content)
```

## Configuration

### Basic Configuration

Edit `config/config.yaml`:

```yaml
server:
  port: 8080

cache:
  l1:
    enabled: true
    max_capacity: 1000

providers:
  openai:
    enabled: true
    api_key: "${OPENAI_API_KEY}"
```

### Environment-specific Configuration

- Development: `config/development.yaml`
- Production: `config/production.yaml`

Set `CONFIG_ENV=production` to use production config.

## Monitoring

### Metrics

Access Prometheus metrics:
```bash
curl http://localhost:9091/metrics
```

### Health Checks

```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
```

## Development Workflow

### Run tests
```bash
cargo test
```

### Run with Redis integration tests
```bash
docker run -d -p 6379:6379 redis:7-alpine
cargo test -- --ignored
```

### Check code quality
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

### Generate documentation
```bash
cargo doc --open
```

## Troubleshooting

### Port already in use
```bash
# Change the port in config/config.yaml
server:
  port: 8081
```

### Redis connection failed
```bash
# Check Redis is running
docker ps | grep redis

# Test Redis connection
redis-cli ping
```

### API key issues
- Ensure your provider API keys are correctly set in `.env`
- Check the logs for authentication errors: `RUST_LOG=debug cargo run`

## Next Steps

1. **Read the Architecture**: See [plans/LLM_EDGE_AGENT_CONSOLIDATED_PLAN.md](plans/LLM_EDGE_AGENT_CONSOLIDATED_PLAN.md)
2. **Configure Providers**: Add more LLM providers in `config/config.yaml`
3. **Enable L2 Cache**: Set up Redis cluster for distributed caching
4. **Deploy to Production**: See deployment guides in `deployments/`
5. **Monitor Performance**: Set up Grafana dashboards

## Getting Help

- Documentation: [docs/](docs/)
- Issues: GitHub Issues
- Discussions: GitHub Discussions

## License

Apache-2.0
