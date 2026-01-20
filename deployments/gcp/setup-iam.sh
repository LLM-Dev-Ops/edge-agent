#!/bin/bash
# =============================================================================
# LLM-Edge-Agent IAM Setup Script
# =============================================================================
#
# This script creates and configures the service account with least-privilege
# permissions required for LLM-Edge-Agent on Google Cloud Run.
#
# Usage:
#   ./setup-iam.sh [PROJECT_ID] [REGION]
#
# Prerequisites:
#   - gcloud CLI authenticated
#   - Project owner or IAM admin permissions

set -euo pipefail

# Configuration
PROJECT_ID="${1:-agentics-dev}"
REGION="${2:-us-central1}"
SERVICE_ACCOUNT_NAME="llm-edge-agent-sa"
SERVICE_ACCOUNT_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"

echo "========================================"
echo "LLM-Edge-Agent IAM Setup"
echo "========================================"
echo "Project: ${PROJECT_ID}"
echo "Region: ${REGION}"
echo "Service Account: ${SERVICE_ACCOUNT_EMAIL}"
echo "========================================"

# Step 1: Create Service Account
echo ""
echo "[1/5] Creating service account..."
if gcloud iam service-accounts describe "${SERVICE_ACCOUNT_EMAIL}" --project="${PROJECT_ID}" &>/dev/null; then
    echo "  Service account already exists"
else
    gcloud iam service-accounts create "${SERVICE_ACCOUNT_NAME}" \
        --project="${PROJECT_ID}" \
        --display-name="LLM Edge Agent Service Account" \
        --description="Service account for LLM-Edge-Agent Cloud Run service. Least-privilege access."
    echo "  Created service account: ${SERVICE_ACCOUNT_EMAIL}"
fi

# Step 2: Grant Cloud Run Invoker (to call other internal services)
echo ""
echo "[2/5] Granting Cloud Run Invoker role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/run.invoker" \
    --condition=None \
    --quiet
echo "  Granted roles/run.invoker"

# Step 3: Grant Secret Manager Accessor (for secrets like RUVECTOR_API_KEY)
echo ""
echo "[3/5] Granting Secret Manager Accessor role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None \
    --quiet
echo "  Granted roles/secretmanager.secretAccessor"

# Step 4: Grant Logging Writer (for Cloud Logging)
echo ""
echo "[4/5] Granting Logging Writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/logging.logWriter" \
    --condition=None \
    --quiet
echo "  Granted roles/logging.logWriter"

# Step 5: Grant Monitoring Writer (for Cloud Monitoring metrics)
echo ""
echo "[5/5] Granting Monitoring Writer role..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/monitoring.metricWriter" \
    --condition=None \
    --quiet
echo "  Granted roles/monitoring.metricWriter"

# Create secrets if they don't exist
echo ""
echo "[Optional] Creating placeholder secrets..."
for SECRET_NAME in "ruvector-config" "telemetry-config"; do
    if gcloud secrets describe "${SECRET_NAME}" --project="${PROJECT_ID}" &>/dev/null; then
        echo "  Secret '${SECRET_NAME}' already exists"
    else
        echo "  Creating secret '${SECRET_NAME}' (placeholder)"
        echo '{"placeholder": "update-me"}' | gcloud secrets create "${SECRET_NAME}" \
            --project="${PROJECT_ID}" \
            --data-file=- \
            --labels="service=llm-edge-agent" \
            2>/dev/null || true
    fi
done

echo ""
echo "========================================"
echo "IAM Setup Complete!"
echo "========================================"
echo ""
echo "Service Account: ${SERVICE_ACCOUNT_EMAIL}"
echo ""
echo "Granted Roles (Least Privilege):"
echo "  - roles/run.invoker          (call internal services)"
echo "  - roles/secretmanager.secretAccessor (read secrets)"
echo "  - roles/logging.logWriter    (write logs)"
echo "  - roles/monitoring.metricWriter (write metrics)"
echo ""
echo "NOT Granted (by design):"
echo "  - roles/cloudsql.client      (NO direct SQL access)"
echo "  - roles/storage.objectAdmin  (NO direct storage access)"
echo "  - roles/pubsub.publisher     (NO direct Pub/Sub access)"
echo ""
echo "Next Steps:"
echo "  1. Update secrets in Secret Manager with actual values"
echo "  2. Deploy the service: gcloud builds submit --config=deployments/gcp/cloudbuild.yaml ."
echo ""
