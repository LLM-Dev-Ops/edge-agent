#!/bin/bash
# Comprehensive Test Runner for LLM Edge Agent
# Runs all test suites with proper orchestration

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
TEST_TYPE="${1:-all}"  # all, unit, integration, load, security, performance, chaos
VERBOSE="${VERBOSE:-false}"
PARALLEL="${PARALLEL:-true}"

# Report directories
REPORT_DIR="./test-reports"
mkdir -p "$REPORT_DIR"/{unit,integration,load,security,performance,chaos}

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}LLM Edge Agent - Test Suite Runner${NC}"
echo -e "${BLUE}=========================================${NC}"
echo "Test Type: $TEST_TYPE"
echo "Parallel: $PARALLEL"
echo "Report Directory: $REPORT_DIR"
echo -e "${BLUE}=========================================${NC}"
echo ""

TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0

run_test_suite() {
    local suite_name=$1
    local command=$2
    local required=$3  # true/false

    TOTAL_SUITES=$((TOTAL_SUITES + 1))

    echo -e "${YELLOW}Running: $suite_name${NC}"
    echo "Command: $command"
    echo ""

    if eval "$command"; then
        echo -e "${GREEN}✅ $suite_name PASSED${NC}"
        PASSED_SUITES=$((PASSED_SUITES + 1))
        echo ""
        return 0
    else
        EXIT_CODE=$?
        echo -e "${RED}❌ $suite_name FAILED (exit code: $EXIT_CODE)${NC}"
        FAILED_SUITES=$((FAILED_SUITES + 1))
        echo ""

        if [ "$required" = "true" ]; then
            echo -e "${RED}Required test suite failed. Stopping.${NC}"
            exit $EXIT_CODE
        fi
        return $EXIT_CODE
    fi
}

# Unit Tests
run_unit_tests() {
    echo -e "${BLUE}=== Unit Tests ===${NC}"
    run_test_suite \
        "Rust Unit Tests" \
        "cargo test --workspace --lib 2>&1 | tee $REPORT_DIR/unit/test-output.txt" \
        true
}

# Integration Tests
run_integration_tests() {
    echo -e "${BLUE}=== Integration Tests ===${NC}"
    run_test_suite \
        "Rust Integration Tests" \
        "cargo test --workspace --test '*' 2>&1 | tee $REPORT_DIR/integration/test-output.txt" \
        true
}

# Load Tests
run_load_tests() {
    echo -e "${BLUE}=== Load Tests ===${NC}"

    # Check if k6 is installed
    if ! command -v k6 &> /dev/null; then
        echo -e "${YELLOW}k6 not found. Installing...${NC}"
        # Installation instructions
        echo "Please install k6: https://k6.io/docs/getting-started/installation/"
        return 1
    fi

    # Start infrastructure if not running
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${YELLOW}Starting infrastructure...${NC}"
        docker-compose -f docker-compose.production.yml up -d
        echo "Waiting for services to be ready..."
        sleep 30
    fi

    run_test_suite \
        "Baseline Load Test" \
        "k6 run tests/load/baseline-load-test.js --out json=$REPORT_DIR/load/baseline-results.json" \
        false

    run_test_suite \
        "Spike Test" \
        "k6 run tests/load/spike-test.js --out json=$REPORT_DIR/load/spike-results.json" \
        false

    run_test_suite \
        "Cache Effectiveness Test" \
        "k6 run tests/load/cache-effectiveness-test.js --out json=$REPORT_DIR/load/cache-results.json" \
        false
}

# Security Tests
run_security_tests() {
    echo -e "${BLUE}=== Security Tests ===${NC}"

    # Dependency scan
    run_test_suite \
        "Dependency Vulnerability Scan" \
        "REPORT_FILE=$REPORT_DIR/security/dependency-scan.txt ./tests/security/dependency-scan.sh" \
        true

    # Penetration tests
    run_test_suite \
        "Penetration Tests" \
        "REPORT_FILE=$REPORT_DIR/security/penetration-test.txt ./tests/security/penetration-test.sh" \
        false

    # OWASP ZAP (optional - requires Docker)
    if command -v docker &> /dev/null; then
        run_test_suite \
            "OWASP ZAP Baseline Scan" \
            "SCAN_TYPE=baseline REPORT_DIR=$REPORT_DIR/security ./tests/security/owasp-zap-scan.sh" \
            false
    fi
}

# Performance Tests
run_performance_tests() {
    echo -e "${BLUE}=== Performance Tests ===${NC}"

    # Benchmarks
    run_test_suite \
        "Rust Benchmarks" \
        "cargo bench --workspace 2>&1 | tee $REPORT_DIR/performance/benchmark-output.txt" \
        false

    # Regression testing
    run_test_suite \
        "Performance Regression Test" \
        "REPORT_FILE=$REPORT_DIR/performance/regression-report.txt ./tests/performance/regression-test.sh" \
        false
}

# Chaos Tests
run_chaos_tests() {
    echo -e "${BLUE}=== Chaos Engineering Tests ===${NC}"

    # Ensure infrastructure is running
    if ! docker-compose -f docker-compose.production.yml ps | grep -q "Up"; then
        echo -e "${YELLOW}Starting infrastructure for chaos tests...${NC}"
        docker-compose -f docker-compose.production.yml up -d
        sleep 30
    fi

    run_test_suite \
        "Chaos Engineering Tests" \
        "REPORT_FILE=$REPORT_DIR/chaos/chaos-results.txt ./tests/chaos/chaos-engineering.sh" \
        false
}

# Code Quality
run_code_quality() {
    echo -e "${BLUE}=== Code Quality Checks ===${NC}"

    run_test_suite \
        "Rustfmt Check" \
        "cargo fmt --all -- --check" \
        true

    run_test_suite \
        "Clippy Lints" \
        "cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee $REPORT_DIR/clippy-output.txt" \
        true
}

# Main test execution
main() {
    START_TIME=$(date +%s)

    # Make scripts executable
    chmod +x tests/security/*.sh tests/performance/*.sh tests/chaos/*.sh 2>/dev/null || true

    case "$TEST_TYPE" in
        all)
            run_code_quality
            run_unit_tests
            run_integration_tests
            run_security_tests
            run_performance_tests
            run_load_tests
            run_chaos_tests
            ;;
        unit)
            run_unit_tests
            ;;
        integration)
            run_integration_tests
            ;;
        load)
            run_load_tests
            ;;
        security)
            run_security_tests
            ;;
        performance)
            run_performance_tests
            ;;
        chaos)
            run_chaos_tests
            ;;
        quality)
            run_code_quality
            ;;
        *)
            echo -e "${RED}Unknown test type: $TEST_TYPE${NC}"
            echo "Valid options: all, unit, integration, load, security, performance, chaos, quality"
            exit 1
            ;;
    esac

    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    # Summary
    echo ""
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}Test Suite Summary${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo "Duration: ${DURATION}s"
    echo "Total Suites: $TOTAL_SUITES"
    echo -e "${GREEN}Passed: $PASSED_SUITES${NC}"
    echo -e "${RED}Failed: $FAILED_SUITES${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo "Reports available in: $REPORT_DIR"

    if [ $FAILED_SUITES -eq 0 ]; then
        echo -e "${GREEN}✅ All test suites passed!${NC}"
        exit 0
    else
        echo -e "${RED}❌ Some test suites failed!${NC}"
        exit 1
    fi
}

# Run main
main
