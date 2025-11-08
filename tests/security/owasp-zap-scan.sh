#!/bin/bash
# OWASP ZAP Security Scan Script
# Performs automated security testing using OWASP ZAP

set -e

# Configuration
TARGET_URL="${TARGET_URL:-http://localhost:8080}"
ZAP_PORT="${ZAP_PORT:-8090}"
REPORT_DIR="${REPORT_DIR:-./security-reports}"
SCAN_TYPE="${SCAN_TYPE:-baseline}"  # baseline, full, or api

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "OWASP ZAP Security Scan"
echo "========================================="
echo "Target: $TARGET_URL"
echo "Scan Type: $SCAN_TYPE"
echo "Report Directory: $REPORT_DIR"
echo "========================================="

# Create report directory
mkdir -p "$REPORT_DIR"

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is required but not installed${NC}"
    exit 1
fi

# Pull ZAP Docker image
echo -e "${YELLOW}Pulling OWASP ZAP Docker image...${NC}"
docker pull owasp/zap2docker-stable

# Run appropriate scan based on type
case $SCAN_TYPE in
    baseline)
        echo -e "${GREEN}Running Baseline Scan (passive scan only)...${NC}"
        docker run --rm \
            --network host \
            -v "$PWD/$REPORT_DIR:/zap/wrk/:rw" \
            owasp/zap2docker-stable zap-baseline.py \
            -t "$TARGET_URL" \
            -r baseline-report.html \
            -J baseline-report.json \
            -w baseline-report.md \
            -c zap-rules.conf \
            -I
        ;;

    full)
        echo -e "${GREEN}Running Full Scan (active + passive)...${NC}"
        docker run --rm \
            --network host \
            -v "$PWD/$REPORT_DIR:/zap/wrk/:rw" \
            owasp/zap2docker-stable zap-full-scan.py \
            -t "$TARGET_URL" \
            -r full-scan-report.html \
            -J full-scan-report.json \
            -w full-scan-report.md \
            -I
        ;;

    api)
        echo -e "${GREEN}Running API Scan...${NC}"
        # Requires OpenAPI spec
        if [ -z "$OPENAPI_SPEC" ]; then
            echo -e "${YELLOW}Warning: OPENAPI_SPEC not set, using default${NC}"
            OPENAPI_SPEC="docs/openapi.yaml"
        fi

        docker run --rm \
            --network host \
            -v "$PWD/$REPORT_DIR:/zap/wrk/:rw" \
            -v "$PWD/$OPENAPI_SPEC:/zap/wrk/openapi.yaml:ro" \
            owasp/zap2docker-stable zap-api-scan.py \
            -t /zap/wrk/openapi.yaml \
            -f openapi \
            -r api-scan-report.html \
            -J api-scan-report.json \
            -w api-scan-report.md \
            -I
        ;;

    *)
        echo -e "${RED}Error: Invalid scan type. Use: baseline, full, or api${NC}"
        exit 1
        ;;
esac

# Check scan results
SCAN_EXIT_CODE=$?

echo ""
echo "========================================="
echo "Scan Complete!"
echo "========================================="
echo "Reports available in: $REPORT_DIR"
ls -lh "$REPORT_DIR"

if [ $SCAN_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✅ No high-risk vulnerabilities found${NC}"
    exit 0
elif [ $SCAN_EXIT_CODE -eq 1 ]; then
    echo -e "${YELLOW}⚠️  Warnings found (review reports)${NC}"
    exit 1
elif [ $SCAN_EXIT_CODE -eq 2 ]; then
    echo -e "${RED}❌ High-risk vulnerabilities found!${NC}"
    exit 2
else
    echo -e "${RED}❌ Scan failed with error${NC}"
    exit $SCAN_EXIT_CODE
fi
