#!/bin/bash
# Dependency Vulnerability Scanning
# Uses cargo-audit to scan Rust dependencies for known vulnerabilities

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPORT_FILE="${REPORT_FILE:-./security-reports/dependency-scan-results.txt}"
mkdir -p "$(dirname "$REPORT_FILE")"

echo "========================================="
echo "Dependency Vulnerability Scan"
echo "========================================="

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-audit...${NC}"
    cargo install cargo-audit --features=fix
fi

# Update advisory database
echo -e "${YELLOW}Updating advisory database...${NC}"
cargo audit fetch

# Run audit
echo -e "${YELLOW}Scanning dependencies...${NC}"
if cargo audit --json > "$REPORT_FILE" 2>&1; then
    echo -e "${GREEN}✅ No vulnerabilities found!${NC}"

    # Pretty print summary
    echo ""
    echo "Dependency Summary:"
    cargo tree --depth 1 | head -20

    exit 0
else
    AUDIT_EXIT_CODE=$?

    echo -e "${RED}❌ Vulnerabilities detected!${NC}"
    echo ""
    echo "Vulnerability Report:"
    cargo audit

    echo ""
    echo -e "${YELLOW}Full report saved to: $REPORT_FILE${NC}"
    echo ""
    echo "To fix vulnerabilities automatically:"
    echo "  cargo audit fix"

    exit $AUDIT_EXIT_CODE
fi
