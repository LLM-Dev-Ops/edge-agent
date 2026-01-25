# Multi-stage Dockerfile for LLM Edge Agent (Rust Workspace)

# Stage 1: Build
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release binary - Phase 3 Unified Service
RUN cargo build --release --package llm-edge-agents --bin llm-edge-agent-phase3

# Stage 2: Production
FROM debian:bookworm-slim

# Install runtime dependencies and security updates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && apt-get upgrade -y \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -g 1001 llm-agent && \
    useradd -u 1001 -g llm-agent -s /bin/bash -m llm-agent

WORKDIR /app

# Copy built binary from builder - Phase 3 Unified Service
COPY --from=builder /app/target/release/llm-edge-agent-phase3 /usr/local/bin/llm-edge-agent

# Create necessary directories
RUN mkdir -p /var/log/llm-edge-agent /etc/llm-edge-agent /cache && \
    chown -R llm-agent:llm-agent /var/log/llm-edge-agent /etc/llm-edge-agent /cache /app

# Switch to non-root user
USER llm-agent

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose ports
# 8080: Main HTTP proxy port
# 9090: Metrics port
EXPOSE 8080 9090

# Phase 3 Layer 1 Environment variables
ENV AGENT_PHASE=phase3 \
    AGENT_LAYER=layer1 \
    MAX_TOKENS=1500 \
    MAX_LATENCY_MS=3000 \
    MAX_CALLS_PER_RUN=4 \
    RUST_LOG=llm_edge_agents=info,tower_http=info \
    SERVER_ADDRESS=0.0.0.0:8080

# Start application
ENTRYPOINT ["/usr/local/bin/llm-edge-agent"]
CMD []

# Labels
LABEL org.opencontainers.image.title="LLM Edge Agent" \
      org.opencontainers.image.description="High-performance LLM intercepting proxy" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.vendor="Global Business Advisors" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/globalbusinessadvisors/llm-edge-agent"
