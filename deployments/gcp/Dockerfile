# Multi-stage Dockerfile for LLM-Edge-Agent Unified Service
# Optimized for Google Cloud Run deployment

# =============================================================================
# Stage 1: Build
# =============================================================================
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

# Build release binary for the unified service
RUN cargo build --release --package llm-edge-agents --bin llm-edge-agent-unified

# =============================================================================
# Stage 2: Production Runtime
# =============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies and security updates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && apt-get upgrade -y \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -g 1001 llm-edge && \
    useradd -u 1001 -g llm-edge -s /bin/bash -m llm-edge

WORKDIR /app

# Copy built binary from builder
COPY --from=builder /app/target/release/llm-edge-agent-unified /usr/local/bin/llm-edge-agent-unified

# Create necessary directories
RUN mkdir -p /var/log/llm-edge-agent && \
    chown -R llm-edge:llm-edge /var/log/llm-edge-agent /app

# Switch to non-root user
USER llm-edge

# Health check for Cloud Run
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || exit 1

# Cloud Run sets PORT environment variable
EXPOSE 8080

# Environment variables (can be overridden at deploy time)
ENV PORT=8080 \
    HOST=0.0.0.0 \
    PLATFORM_ENV=prod \
    RUST_LOG=llm_edge_agents=info,tower_http=info \
    RUST_BACKTRACE=0

# Start the unified service
ENTRYPOINT ["/usr/local/bin/llm-edge-agent-unified"]
CMD []

# Labels for container registry
LABEL org.opencontainers.image.title="LLM Edge Agent Unified Service" \
      org.opencontainers.image.description="Runtime interception, routing, and execution control agents for LLM workloads" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.vendor="Global Business Advisors" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/globalbusinessadvisors/llm-edge-agent"
