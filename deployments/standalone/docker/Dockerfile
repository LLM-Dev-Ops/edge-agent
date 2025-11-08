# Multi-stage Dockerfile for LLM-Edge-Agent (Standalone)

# Stage 1: Build
FROM node:18-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    git

# Copy package files
COPY package*.json ./
COPY tsconfig.json ./

# Install dependencies
RUN npm ci --only=production && \
    npm cache clean --force

# Copy source code
COPY src/ ./src/

# Build TypeScript
RUN npm run build

# Stage 2: Production
FROM node:18-alpine

# Install security updates
RUN apk upgrade --no-cache && \
    apk add --no-cache \
    ca-certificates \
    tini \
    curl

# Create non-root user
RUN addgroup -g 1001 -S llm-agent && \
    adduser -u 1001 -S llm-agent -G llm-agent

WORKDIR /app

# Copy built application from builder
COPY --from=builder --chown=llm-agent:llm-agent /app/node_modules ./node_modules
COPY --from=builder --chown=llm-agent:llm-agent /app/dist ./dist
COPY --from=builder --chown=llm-agent:llm-agent /app/package*.json ./

# Create necessary directories
RUN mkdir -p /var/log/llm-edge-agent /etc/llm-edge-agent /cache && \
    chown -R llm-agent:llm-agent /var/log/llm-edge-agent /etc/llm-edge-agent /cache

# Switch to non-root user
USER llm-agent

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose ports
EXPOSE 8080 9090

# Use tini for proper signal handling
ENTRYPOINT ["/sbin/tini", "--"]

# Start application
CMD ["node", "dist/index.js"]

# Labels
LABEL org.opencontainers.image.title="LLM-Edge-Agent" \
      org.opencontainers.image.description="Intelligent proxy for LLM API requests" \
      org.opencontainers.image.version="1.0.0" \
      org.opencontainers.image.vendor="LLM-Edge-Agent" \
      org.opencontainers.image.licenses="MIT"
