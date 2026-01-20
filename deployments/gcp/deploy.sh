#!/bin/bash
# =============================================================================
# LLM-Edge-Agent Production Deployment Script
# =============================================================================
#
# This script performs the complete deployment of LLM-Edge-Agent to Google Cloud Run.
#
# Usage:
#   ./deploy.sh [PROJECT_ID] [REGION] [PLATFORM_ENV]
#
# Example:
#   ./deploy.sh agentics-dev us-central1 prod

set -euo pipefail

# Configuration
PROJECT_ID="${1:-agentics-dev}"
REGION="${2:-us-central1}"
PLATFORM_ENV="${3:-prod}"
SERVICE_NAME="llm-edge-agent"
IMAGE_TAG="${GITHUB_SHA:-$(git rev-parse --short HEAD 2>/dev/null || echo 'latest')}"

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║           LLM-Edge-Agent Production Deployment                              ║"
echo "╠════════════════════════════════════════════════════════════════════════════╣"
echo "║  Project:     ${PROJECT_ID}"
echo "║  Region:      ${REGION}"
echo "║  Environment: ${PLATFORM_ENV}"
echo "║  Service:     ${SERVICE_NAME}"
echo "║  Image Tag:   ${IMAGE_TAG}"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Step 0: Validate gcloud configuration
echo "[0/7] Validating gcloud configuration..."
CURRENT_PROJECT=$(gcloud config get-value project 2>/dev/null)
if [[ "${CURRENT_PROJECT}" != "${PROJECT_ID}" ]]; then
    echo "  Setting project to ${PROJECT_ID}..."
    gcloud config set project "${PROJECT_ID}"
fi
echo "  Project: ${PROJECT_ID} ✓"

# Step 1: Run IAM setup (idempotent)
echo ""
echo "[1/7] Setting up IAM (idempotent)..."
if [[ -f "$(dirname "$0")/setup-iam.sh" ]]; then
    bash "$(dirname "$0")/setup-iam.sh" "${PROJECT_ID}" "${REGION}" || {
        echo "  Warning: IAM setup had issues, continuing..."
    }
else
    echo "  IAM setup script not found, assuming already configured"
fi

# Step 2: Build Docker image
echo ""
echo "[2/7] Building Docker image..."
DOCKERFILE_PATH="deployments/gcp/Dockerfile"
if [[ ! -f "${DOCKERFILE_PATH}" ]]; then
    DOCKERFILE_PATH="Dockerfile"
fi

docker build \
    -t "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" \
    -t "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest" \
    -f "${DOCKERFILE_PATH}" \
    . || {
    echo "  Docker build failed. Trying Cloud Build..."
    gcloud builds submit \
        --tag "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" \
        --timeout=1800s \
        .
}
echo "  Image built: gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG} ✓"

# Step 3: Push to Container Registry
echo ""
echo "[3/7] Pushing image to Container Registry..."
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" 2>/dev/null || \
gcloud auth configure-docker --quiet && \
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}"
docker push "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:latest" || true
echo "  Image pushed ✓"

# Step 4: Get RuVector and Telemetry URLs (if available)
echo ""
echo "[4/7] Resolving service dependencies..."
RUVECTOR_URL=$(gcloud run services describe ruvector-service --region="${REGION}" --format='value(status.url)' 2>/dev/null || echo "https://ruvector-service-placeholder.run.app")
TELEMETRY_URL=$(gcloud run services describe observatory --region="${REGION}" --format='value(status.url)' 2>/dev/null || echo "https://observatory-placeholder.run.app")
echo "  RuVector URL: ${RUVECTOR_URL}"
echo "  Telemetry URL: ${TELEMETRY_URL}"

# Step 5: Deploy to Cloud Run
echo ""
echo "[5/7] Deploying to Cloud Run..."
gcloud run deploy "${SERVICE_NAME}" \
    --image "gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}" \
    --region "${REGION}" \
    --platform managed \
    --port 8080 \
    --memory 512Mi \
    --cpu 1 \
    --min-instances 0 \
    --max-instances 10 \
    --timeout 60s \
    --concurrency 80 \
    --cpu-throttling \
    --set-env-vars "PLATFORM_ENV=${PLATFORM_ENV}" \
    --set-env-vars "SERVICE_NAME=${SERVICE_NAME}" \
    --set-env-vars "SERVICE_VERSION=${IMAGE_TAG}" \
    --set-env-vars "RUVECTOR_SERVICE_URL=${RUVECTOR_URL}" \
    --set-env-vars "TELEMETRY_ENDPOINT=${TELEMETRY_URL}" \
    --set-env-vars "RUST_LOG=llm_edge_agents=info,tower_http=info" \
    --service-account "llm-edge-agent-sa@${PROJECT_ID}.iam.gserviceaccount.com" \
    --ingress internal-and-cloud-load-balancing \
    --no-allow-unauthenticated \
    --quiet
echo "  Deployed ✓"

# Step 6: Get service URL
echo ""
echo "[6/7] Getting service URL..."
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" --region="${REGION}" --format='value(status.url)')
echo "  Service URL: ${SERVICE_URL}"

# Step 7: Verify deployment
echo ""
echo "[7/7] Verifying deployment..."
echo "  Waiting for service to stabilize..."
sleep 10

# Get ID token for authenticated request
TOKEN=$(gcloud auth print-identity-token 2>/dev/null || echo "")
if [[ -n "${TOKEN}" ]]; then
    HEALTH_RESPONSE=$(curl -s -H "Authorization: Bearer ${TOKEN}" "${SERVICE_URL}/health" 2>/dev/null || echo '{"status":"unknown"}')
    echo "  Health check response: ${HEALTH_RESPONSE}"
fi

echo ""
echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║                    Deployment Complete!                                     ║"
echo "╠════════════════════════════════════════════════════════════════════════════╣"
echo "║  Service URL: ${SERVICE_URL}"
echo "║"
echo "║  Agent Endpoints:"
echo "║    - ${SERVICE_URL}/agents/tool-invocation/*"
echo "║    - ${SERVICE_URL}/agents/circuit-breaker/*"
echo "║    - ${SERVICE_URL}/agents/failover/*"
echo "║    - ${SERVICE_URL}/agents/execution-guard/*"
echo "║    - ${SERVICE_URL}/agents/caching-strategy/*"
echo "║"
echo "║  Health Endpoints:"
echo "║    - ${SERVICE_URL}/health"
echo "║    - ${SERVICE_URL}/health/ready"
echo "║    - ${SERVICE_URL}/health/live"
echo "║    - ${SERVICE_URL}/metrics"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Verify all agent endpoints: ./verify-deployment.sh ${SERVICE_URL}"
echo "  2. Update DNS/Load Balancer if needed"
echo "  3. Configure alerting in Cloud Monitoring"
echo ""
