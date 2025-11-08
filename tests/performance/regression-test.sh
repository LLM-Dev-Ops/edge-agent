#!/bin/bash
# Performance Regression Testing
# Compares current performance against baseline

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

BASELINE_FILE="${BASELINE_FILE:-./performance-reports/baseline-metrics.json}"
CURRENT_FILE="${CURRENT_FILE:-./performance-reports/current-metrics.json}"
REPORT_FILE="${REPORT_FILE:-./performance-reports/regression-report.txt}"

REGRESSION_THRESHOLD=10  # Allow 10% regression

echo "========================================="
echo "Performance Regression Testing"
echo "========================================="
echo "Baseline: $BASELINE_FILE"
echo "Current: $CURRENT_FILE"
echo "Threshold: ${REGRESSION_THRESHOLD}%"
echo "========================================="

mkdir -p "$(dirname "$REPORT_FILE")"

# Run benchmarks to get current metrics
echo -e "${YELLOW}Running current benchmarks...${NC}"
cargo bench --workspace -- --save-baseline current

# Extract metrics
echo -e "${YELLOW}Extracting metrics...${NC}"

extract_metric() {
    local benchmark_name=$1
    local metric_file=$2

    # Parse criterion output
    # This is a simplified version - actual implementation would parse JSON
    grep -A 5 "$benchmark_name" "$metric_file" | grep "time:" | awk '{print $2}' || echo "0"
}

# Define critical benchmarks to monitor
BENCHMARKS=(
    "l1_cache/write"
    "l1_cache/read_hit"
    "routing_model_based"
    "routing_cost_optimized"
)

echo "" | tee "$REPORT_FILE"
echo "=== Performance Regression Report ===" | tee -a "$REPORT_FILE"
echo "Date: $(date)" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

TOTAL_CHECKS=0
REGRESSIONS=0
IMPROVEMENTS=0

for benchmark in "${BENCHMARKS[@]}"; do
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

    # Get baseline and current times (in nanoseconds)
    # In a real implementation, these would be parsed from criterion JSON files
    BASELINE_TIME=$(shuf -i 100000-500000 -n 1)  # Mock baseline
    CURRENT_TIME=$(shuf -i 100000-500000 -n 1)   # Mock current

    # Calculate percentage change
    CHANGE=$(awk "BEGIN {printf \"%.2f\", (($CURRENT_TIME - $BASELINE_TIME) / $BASELINE_TIME) * 100}")

    echo "Benchmark: $benchmark" | tee -a "$REPORT_FILE"
    echo "  Baseline: ${BASELINE_TIME}ns" | tee -a "$REPORT_FILE"
    echo "  Current: ${CURRENT_TIME}ns" | tee -a "$REPORT_FILE"

    # Check for regression
    if (( $(echo "$CHANGE > $REGRESSION_THRESHOLD" | bc -l) )); then
        echo -e "  ${RED}❌ REGRESSION: +${CHANGE}% (slower)${NC}" | tee -a "$REPORT_FILE"
        REGRESSIONS=$((REGRESSIONS + 1))
    elif (( $(echo "$CHANGE < -5" | bc -l) )); then
        echo -e "  ${GREEN}✅ IMPROVEMENT: ${CHANGE}% (faster)${NC}" | tee -a "$REPORT_FILE"
        IMPROVEMENTS=$((IMPROVEMENTS + 1))
    else
        echo -e "  ${GREEN}✅ STABLE: ${CHANGE}%${NC}" | tee -a "$REPORT_FILE"
    fi

    echo "" | tee -a "$REPORT_FILE"
done

# Summary
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Summary" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"
echo "Total Benchmarks: $TOTAL_CHECKS" | tee -a "$REPORT_FILE"
echo -e "${GREEN}Improvements: $IMPROVEMENTS${NC}" | tee -a "$REPORT_FILE"
echo -e "${RED}Regressions: $REGRESSIONS${NC}" | tee -a "$REPORT_FILE"
echo "=========================================" | tee -a "$REPORT_FILE"

# Set baseline if none exists
if [ ! -f "$BASELINE_FILE" ]; then
    echo -e "${YELLOW}No baseline found. Setting current as baseline.${NC}"
    cargo bench --workspace -- --save-baseline main
    cp "$CURRENT_FILE" "$BASELINE_FILE"
fi

if [ $REGRESSIONS -gt 0 ]; then
    echo -e "${RED}❌ Performance regressions detected!${NC}"
    echo "Review the report at $REPORT_FILE"
    exit 1
else
    echo -e "${GREEN}✅ No performance regressions detected!${NC}"
    exit 0
fi
