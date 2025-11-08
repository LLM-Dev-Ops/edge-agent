#!/bin/bash
# Penetration Testing Script
# Tests for common vulnerabilities: SQLi, XSS, CSRF, injection attacks

set -e

TARGET_URL="${TARGET_URL:-http://localhost:8080}"
REPORT_FILE="${REPORT_FILE:-./security-reports/penetration-test-results.txt}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

mkdir -p "$(dirname "$REPORT_FILE")"

echo "=========================================" | tee "$REPORT_FILE"
echo "Penetration Testing Suite" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Target: $TARGET_URL" | tee -a "$REPORT_FILE"
echo "Date: $(date)" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test() {
    local test_name=$1
    local test_command=$2
    local expected_behavior=$3

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "${YELLOW}Running: $test_name${NC}" | tee -a "$REPORT_FILE"

    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASS: $expected_behavior${NC}" | tee -a "$REPORT_FILE"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL: $expected_behavior${NC}" | tee -a "$REPORT_FILE"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    echo "" | tee -a "$REPORT_FILE"
}

# Test 1: SQL Injection Prevention
echo "=== 1. SQL Injection Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "SQL Injection in JSON payload" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"' OR '1'='1\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '400\|422'" \
    "Should reject malformed SQL injection attempts"

# Test 2: XSS Prevention
echo "=== 2. Cross-Site Scripting (XSS) Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "XSS in message content" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"<script>alert(1)</script>\"}]}' \
        | grep -qv '<script>'" \
    "Should sanitize or escape XSS payloads"

# Test 3: Command Injection
echo "=== 3. Command Injection Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Command injection in model field" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"; rm -rf /\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '400\|422'" \
    "Should reject command injection attempts"

# Test 4: Path Traversal
echo "=== 4. Path Traversal Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Path traversal attempt" \
    "curl -s $TARGET_URL/../../../etc/passwd \
        -o /dev/null -w '%{http_code}' | grep -q '404'" \
    "Should prevent path traversal attacks"

# Test 5: Authentication Bypass
echo "=== 5. Authentication Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Request without authentication" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '401'" \
    "Should reject unauthenticated requests"

run_test \
    "Invalid API key" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -H 'Authorization: Bearer invalid-key-123' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '401\|403'" \
    "Should reject invalid API keys"

# Test 6: Rate Limiting
echo "=== 6. Rate Limiting Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Rate limit enforcement" \
    "for i in {1..101}; do \
        curl -s -X POST $TARGET_URL/v1/chat/completions \
            -H 'Content-Type: application/json' \
            -H 'Authorization: Bearer test-key' \
            -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
            -o /dev/null -w '%{http_code}\n' \
        done | grep -q '429'" \
    "Should enforce rate limits"

# Test 7: Input Validation
echo "=== 7. Input Validation Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Extremely large payload" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"'$(python3 -c 'print(\"A\" * 1000000)')'\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '400\|413\|422'" \
    "Should reject oversized payloads"

run_test \
    "Invalid JSON" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{invalid json' \
        -o /dev/null -w '%{http_code}' | grep -q '400'" \
    "Should reject malformed JSON"

# Test 8: HTTP Headers Security
echo "=== 8. Security Headers Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "X-Content-Type-Options header" \
    "curl -s -I $TARGET_URL/health | grep -q 'X-Content-Type-Options: nosniff'" \
    "Should set X-Content-Type-Options header"

run_test \
    "X-Frame-Options header" \
    "curl -s -I $TARGET_URL/health | grep -q 'X-Frame-Options: DENY'" \
    "Should set X-Frame-Options header"

# Test 9: SSRF Prevention
echo "=== 9. Server-Side Request Forgery (SSRF) Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "SSRF attempt via model parameter" \
    "curl -s -X POST $TARGET_URL/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d '{\"model\":\"http://169.254.169.254/latest/meta-data/\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}]}' \
        -o /dev/null -w '%{http_code}' | grep -q '400\|422'" \
    "Should prevent SSRF attacks"

# Test 10: Information Disclosure
echo "=== 10. Information Disclosure Tests ===" | tee -a "$REPORT_FILE"
run_test \
    "Error messages don't leak sensitive info" \
    "curl -s -X POST $TARGET_URL/v1/invalid/endpoint \
        | grep -qv 'stack trace\|internal error\|database'" \
    "Error messages should not expose internals"

# Summary
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Test Summary" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Total Tests: $TOTAL_TESTS" | tee -a "$REPORT_FILE"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}" | tee -a "$REPORT_FILE"
echo -e "${RED}Failed: $FAILED_TESTS${NC}" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ All security tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some security tests failed. Review the report at $REPORT_FILE${NC}"
    exit 1
fi
