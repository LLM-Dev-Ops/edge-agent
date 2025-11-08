#!/bin/bash
# Chaos Engineering Tests
# Tests system resilience under various failure scenarios

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

TARGET_URL="${TARGET_URL:-http://localhost:8080}"
REPORT_FILE="${REPORT_FILE:-./chaos-reports/chaos-test-results.txt}"

mkdir -p "$(dirname "$REPORT_FILE")"

echo "=========================================" | tee "$REPORT_FILE"
echo "Chaos Engineering Tests" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Target: $TARGET_URL" | tee -a "$REPORT_FILE"
echo "Date: $(date)" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_chaos_test() {
    local test_name=$1
    local chaos_command=$2
    local recovery_command=$3
    local validation_command=$4

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "${YELLOW}Running Chaos Test: $test_name${NC}" | tee -a "$REPORT_FILE"

    # Inject chaos
    echo "  Injecting chaos..." | tee -a "$REPORT_FILE"
    eval "$chaos_command" || true

    sleep 5  # Let system experience the chaos

    # Validate system behavior
    echo "  Validating system response..." | tee -a "$REPORT_FILE"
    if eval "$validation_command"; then
        echo -e "${GREEN}✅ PASS: System handled failure gracefully${NC}" | tee -a "$REPORT_FILE"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL: System did not handle failure properly${NC}" | tee -a "$REPORT_FILE"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    # Recover from chaos
    echo "  Recovering system..." | tee -a "$REPORT_FILE"
    eval "$recovery_command" || true

    sleep 5  # Let system recover

    echo "" | tee -a "$REPORT_FILE"
}

# Test 1: Redis Node Failure
echo "=== 1. Cache Layer Failure ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Redis cache node failure" \
    "docker-compose -f docker-compose.production.yml stop redis-1" \
    "docker-compose -f docker-compose.production.yml start redis-1" \
    "curl -s -f $TARGET_URL/health && curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '200'"

# Test 2: All Redis Nodes Failure
echo "=== 2. Complete Cache Cluster Failure ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Complete cache cluster failure" \
    "docker-compose -f docker-compose.production.yml stop redis-1 redis-2 redis-3" \
    "docker-compose -f docker-compose.production.yml start redis-1 redis-2 redis-3" \
    "curl -s -f $TARGET_URL/health"

# Test 3: Network Latency Injection
echo "=== 3. Network Latency Injection ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "High network latency (500ms)" \
    "docker run --rm --network=host gaiadocker/iproute2 tc qdisc add dev lo root netem delay 500ms || true" \
    "docker run --rm --network=host gaiadocker/iproute2 tc qdisc del dev lo root || true" \
    "curl -s -f -m 10 $TARGET_URL/health"

# Test 4: Prometheus Monitoring Failure
echo "=== 4. Monitoring System Failure ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Prometheus monitoring failure" \
    "docker-compose -f docker-compose.production.yml stop prometheus" \
    "docker-compose -f docker-compose.production.yml start prometheus" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '200'"

# Test 5: Jaeger Tracing Failure
echo "=== 5. Distributed Tracing Failure ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Jaeger tracing system failure" \
    "docker-compose -f docker-compose.production.yml stop jaeger" \
    "docker-compose -f docker-compose.production.yml start jaeger" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '200'"

# Test 6: High CPU Load
echo "=== 6. Resource Exhaustion (CPU) ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "CPU stress test" \
    "docker run --rm -d --name cpu-stress --cpus=4 progrium/stress --cpu 4 --timeout 10s" \
    "docker stop cpu-stress 2>/dev/null || true; docker rm cpu-stress 2>/dev/null || true" \
    "curl -s -f -m 5 $TARGET_URL/health"

# Test 7: Memory Pressure
echo "=== 7. Resource Exhaustion (Memory) ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Memory stress test" \
    "docker run --rm -d --name mem-stress --memory=2g progrium/stress --vm 1 --vm-bytes 1G --timeout 10s" \
    "docker stop mem-stress 2>/dev/null || true; docker rm mem-stress 2>/dev/null || true" \
    "curl -s -f -m 5 $TARGET_URL/health"

# Test 8: Container Restart
echo "=== 8. Application Restart ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Application container restart" \
    "docker-compose -f docker-compose.production.yml restart llm-edge-agent" \
    "sleep 30"  # Wait for restart \
    "curl -s -f $TARGET_URL/health && curl -s -f $TARGET_URL/health/ready"

# Test 9: Partial Network Partition
echo "=== 9. Network Partition ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "Partial network partition" \
    "docker network disconnect llm-edge-network redis-2 || true" \
    "docker network connect llm-edge-network redis-2 || true" \
    "curl -s -f $TARGET_URL/health"

# Test 10: Clock Skew
echo "=== 10. Clock Skew ===" | tee -a "$REPORT_FILE"
run_chaos_test \
    "System clock skew" \
    "docker exec llm-edge-agent date -s '+1 hour' 2>/dev/null || true" \
    "docker-compose -f docker-compose.production.yml restart llm-edge-agent && sleep 30" \
    "curl -s -f $TARGET_URL/health"

# Summary
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Chaos Engineering Test Summary" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Total Tests: $TOTAL_TESTS" | tee -a "$REPORT_FILE"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}" | tee -a "$REPORT_FILE"
echo -e "${RED}Failed: $FAILED_TESTS${NC}" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ All chaos tests passed! System is resilient.${NC}"
    exit 0
else
    echo -e "${RED}❌ Some chaos tests failed. System needs resilience improvements.${NC}"
    exit 1
fi
