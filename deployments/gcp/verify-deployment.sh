#!/bin/bash
# =============================================================================
# LLM-Edge-Agent Post-Deployment Verification
# =============================================================================
#
# This script verifies all agent endpoints are responding correctly.
#
# Usage:
#   ./verify-deployment.sh [SERVICE_URL]
#
# Example:
#   ./verify-deployment.sh https://llm-edge-agent-xxx-uc.a.run.app

set -euo pipefail

SERVICE_URL="${1:-}"
if [[ -z "${SERVICE_URL}" ]]; then
    echo "Usage: ./verify-deployment.sh <SERVICE_URL>"
    echo "Example: ./verify-deployment.sh https://llm-edge-agent-xxx-uc.a.run.app"
    exit 1
fi

# Remove trailing slash
SERVICE_URL="${SERVICE_URL%/}"

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║           LLM-Edge-Agent Post-Deployment Verification                       ║"
echo "╠════════════════════════════════════════════════════════════════════════════╣"
echo "║  Service URL: ${SERVICE_URL}"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Get auth token
TOKEN=$(gcloud auth print-identity-token 2>/dev/null || echo "")
AUTH_HEADER=""
if [[ -n "${TOKEN}" ]]; then
    AUTH_HEADER="Authorization: Bearer ${TOKEN}"
fi

# Track results
PASSED=0
FAILED=0
TOTAL=0

# Function to test endpoint
test_endpoint() {
    local METHOD="$1"
    local PATH="$2"
    local DESCRIPTION="$3"
    local EXPECTED_STATUS="${4:-200}"
    local BODY="${5:-}"

    TOTAL=$((TOTAL + 1))

    local CURL_ARGS=("-s" "-o" "/tmp/response.json" "-w" "%{http_code}")
    if [[ -n "${AUTH_HEADER}" ]]; then
        CURL_ARGS+=("-H" "${AUTH_HEADER}")
    fi
    CURL_ARGS+=("-H" "Content-Type: application/json")

    if [[ "${METHOD}" == "POST" && -n "${BODY}" ]]; then
        CURL_ARGS+=("-X" "POST" "-d" "${BODY}")
    elif [[ "${METHOD}" == "GET" ]]; then
        CURL_ARGS+=("-X" "GET")
    fi

    local STATUS=$(curl "${CURL_ARGS[@]}" "${SERVICE_URL}${PATH}" 2>/dev/null || echo "000")

    if [[ "${STATUS}" == "${EXPECTED_STATUS}" ]]; then
        echo "  ✓ [${STATUS}] ${METHOD} ${PATH} - ${DESCRIPTION}"
        PASSED=$((PASSED + 1))
    else
        echo "  ✗ [${STATUS}] ${METHOD} ${PATH} - ${DESCRIPTION} (expected ${EXPECTED_STATUS})"
        FAILED=$((FAILED + 1))
    fi
}

echo "──────────────────────────────────────────────────────────────────────────────"
echo "1. SERVICE HEALTH ENDPOINTS"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/health" "Main health check"
test_endpoint "GET" "/health/ready" "Readiness probe"
test_endpoint "GET" "/health/live" "Liveness probe"
test_endpoint "GET" "/metrics" "Prometheus metrics"
test_endpoint "GET" "/" "Service info"

echo ""
echo "──────────────────────────────────────────────────────────────────────────────"
echo "2. TOOL INVOCATION AGENT"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/agents/tool-invocation/health" "Agent health"
test_endpoint "GET" "/agents/tool-invocation/inspect" "Agent inspect"
test_endpoint "POST" "/agents/tool-invocation/simulate" "Agent simulate" "200" \
    '{"tool_name":"test_tool","params":{},"blocked":false}'

echo ""
echo "──────────────────────────────────────────────────────────────────────────────"
echo "3. CIRCUIT BREAKER AGENT"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/agents/circuit-breaker/health" "Agent health"
test_endpoint "GET" "/agents/circuit-breaker/metrics" "Agent metrics"
test_endpoint "GET" "/agents/circuit-breaker/inspect/openai" "Agent inspect"
test_endpoint "POST" "/agents/circuit-breaker/simulate" "Agent simulate" "200" \
    '{"provider_id":"test-provider","initial_state":"Closed","initial_failure_count":0,"initial_success_count":0}'

echo ""
echo "──────────────────────────────────────────────────────────────────────────────"
echo "4. FAILOVER AGENT"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/agents/failover/failover/health" "Agent health"
test_endpoint "GET" "/agents/failover/failover/info" "Agent info"
test_endpoint "GET" "/agents/failover/failover/inspect" "Agent inspect"
test_endpoint "POST" "/agents/failover/failover/simulate" "Agent simulate" "200" \
    '{"scenario":"primary-down"}'

echo ""
echo "──────────────────────────────────────────────────────────────────────────────"
echo "5. EXECUTION GUARD AGENT"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/agents/execution-guard/health" "Agent health"
test_endpoint "GET" "/agents/execution-guard/inspect" "Agent inspect"
test_endpoint "POST" "/agents/execution-guard/simulate" "Agent simulate" "200" \
    '{"model":"gpt-4","input_tokens":1000,"max_output_tokens":2000}'

echo ""
echo "──────────────────────────────────────────────────────────────────────────────"
echo "6. CACHING STRATEGY AGENT"
echo "──────────────────────────────────────────────────────────────────────────────"
test_endpoint "GET" "/agents/caching-strategy/cache-strategy/inspect" "Agent inspect"
test_endpoint "POST" "/agents/caching-strategy/cache-strategy/simulate" "Agent simulate" "200" \
    '{"request_type":"chat_completion","provider_id":"openai","model_id":"gpt-4","simulate_cache_hit":false}'

echo ""
echo "══════════════════════════════════════════════════════════════════════════════"
echo "                           VERIFICATION SUMMARY"
echo "══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "  Total Tests:  ${TOTAL}"
echo "  Passed:       ${PASSED}"
echo "  Failed:       ${FAILED}"
echo ""

if [[ ${FAILED} -eq 0 ]]; then
    echo "  ╔═══════════════════════════════════════════════╗"
    echo "  ║  ✓ ALL VERIFICATION TESTS PASSED!             ║"
    echo "  ╚═══════════════════════════════════════════════╝"
    exit 0
else
    echo "  ╔═══════════════════════════════════════════════╗"
    echo "  ║  ✗ SOME VERIFICATION TESTS FAILED             ║"
    echo "  ╚═══════════════════════════════════════════════╝"
    echo ""
    echo "  Troubleshooting:"
    echo "    1. Check Cloud Run logs: gcloud run logs tail llm-edge-agent --region=us-central1"
    echo "    2. Verify service account permissions"
    echo "    3. Check Secret Manager for required secrets"
    echo "    4. Verify ruvector-service is healthy"
    exit 1
fi
