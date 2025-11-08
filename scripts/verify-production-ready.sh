#!/bin/bash
#
# LLM Edge Agent - Production Readiness Verification Script
#
# This script verifies that the system is production-ready by running
# comprehensive checks on code quality, compilation, tests, and security.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASS=0
FAIL=0
WARN=0

echo "========================================="
echo "LLM Edge Agent - Production Readiness Check"
echo "========================================="
echo ""

# Function to print status
print_status() {
    local status=$1
    local message=$2

    case $status in
        "PASS")
            echo -e "${GREEN}✓ PASS${NC}: $message"
            ((PASS++))
            ;;
        "FAIL")
            echo -e "${RED}✗ FAIL${NC}: $message"
            ((FAIL++))
            ;;
        "WARN")
            echo -e "${YELLOW}⚠ WARN${NC}: $message"
            ((WARN++))
            ;;
    esac
}

# Function to run a check
run_check() {
    local name=$1
    shift
    echo ""
    echo "Running: $name"
    echo "Command: $@"
    if "$@" > /tmp/check_output.txt 2>&1; then
        print_status "PASS" "$name"
        return 0
    else
        print_status "FAIL" "$name - See /tmp/check_output.txt for details"
        cat /tmp/check_output.txt
        return 1
    fi
}

# Set PATH for cargo
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Check Rust toolchain
echo ""
echo "=== 1. Toolchain Verification ==="
if command -v rustc >/dev/null 2>&1; then
    RUST_VERSION=$(rustc --version)
    print_status "PASS" "Rust toolchain installed: $RUST_VERSION"
else
    print_status "FAIL" "Rust toolchain not found"
fi

if command -v cargo >/dev/null 2>&1; then
    CARGO_VERSION=$(cargo --version)
    print_status "PASS" "Cargo installed: $CARGO_VERSION"
else
    print_status "FAIL" "Cargo not found"
fi

# 2. Check project structure
echo ""
echo "=== 2. Project Structure Verification ==="
required_dirs=(
    "crates/llm-edge-agent"
    "crates/llm-edge-proxy"
    "crates/llm-edge-cache"
    "crates/llm-edge-routing"
    "crates/llm-edge-providers"
    "crates/llm-edge-security"
    "crates/llm-edge-monitoring"
)

for dir in "${required_dirs[@]}"; do
    if [ -d "$dir" ]; then
        print_status "PASS" "Directory exists: $dir"
    else
        print_status "FAIL" "Missing directory: $dir"
    fi
done

# 3. Compilation checks
echo ""
echo "=== 3. Compilation Verification ==="
run_check "Workspace compilation (debug)" cargo build --workspace || true
run_check "Workspace compilation (release)" cargo build --workspace --release || true

# 4. Code quality checks
echo ""
echo "=== 4. Code Quality Checks ==="
run_check "Formatting check" cargo fmt --all -- --check || true
run_check "Clippy lints" cargo clippy --workspace -- -D warnings || true

# 5. Test suite
echo ""
echo "=== 5. Test Suite Verification ==="
run_check "Unit tests" cargo test --workspace --lib || true
run_check "Integration tests" cargo test --workspace --test '*' || true

# 6. Security audit
echo ""
echo "=== 6. Security Audit ==="
if command -v cargo-audit >/dev/null 2>&1; then
    run_check "Dependency audit" cargo audit || true
else
    print_status "WARN" "cargo-audit not installed (install with: cargo install cargo-audit)"
fi

# 7. Documentation
echo ""
echo "=== 7. Documentation Verification ==="
run_check "Doc generation" cargo doc --workspace --no-deps || true

required_docs=(
    "README.md"
    "QUICKSTART.md"
    "DEVELOPMENT.md"
    "CONTRIBUTING.md"
    "CHANGELOG.md"
)

for doc in "${required_docs[@]}"; do
    if [ -f "$doc" ]; then
        print_status "PASS" "Documentation exists: $doc"
    else
        print_status "FAIL" "Missing documentation: $doc"
    fi
done

# 8. Binary size check (release mode)
echo ""
echo "=== 8. Binary Size Verification ==="
BINARY_PATH="target/release/llm-edge-agent"
if [ -f "$BINARY_PATH" ]; then
    SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    print_status "PASS" "Binary size: $SIZE"

    # Check if stripped
    if file "$BINARY_PATH" | grep -q "not stripped"; then
        print_status "WARN" "Binary not stripped (check profile.release.strip in Cargo.toml)"
    else
        print_status "PASS" "Binary is stripped"
    fi
else
    print_status "WARN" "Release binary not built"
fi

# 9. Dependency tree check
echo ""
echo "=== 9. Dependency Analysis ==="
echo "Total dependencies:"
cargo tree --workspace | wc -l
echo ""
echo "Direct dependencies:"
cargo tree --workspace --depth 1

# 10. Performance checks (if criterion installed)
echo ""
echo "=== 10. Benchmarks ==="
if cargo bench --help 2>/dev/null | grep -q "Run the benchmarks"; then
    print_status "PASS" "Benchmark infrastructure available"
else
    print_status "WARN" "No benchmarks configured"
fi

# 11. Environment validation
echo ""
echo "=== 11. Environment Configuration ==="
if [ -f ".env.example" ]; then
    print_status "PASS" ".env.example exists"
else
    print_status "WARN" ".env.example not found"
fi

# 12. CI/CD validation
echo ""
echo "=== 12. CI/CD Configuration ==="
if [ -d ".github/workflows" ]; then
    WORKFLOW_COUNT=$(find .github/workflows -name '*.yml' -o -name '*.yaml' | wc -l)
    print_status "PASS" "GitHub Actions configured ($WORKFLOW_COUNT workflows)"
else
    print_status "WARN" "No CI/CD workflows found"
fi

# Final summary
echo ""
echo "========================================="
echo "Production Readiness Summary"
echo "========================================="
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${YELLOW}Warnings: $WARN${NC}"
echo -e "${RED}Failed: $FAIL${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
    if [ $WARN -eq 0 ]; then
        echo -e "${GREEN}✓ PRODUCTION READY${NC}: All checks passed!"
        exit 0
    else
        echo -e "${YELLOW}⚠ MOSTLY READY${NC}: All critical checks passed, but there are warnings"
        exit 0
    fi
else
    echo -e "${RED}✗ NOT READY${NC}: $FAIL critical check(s) failed"
    exit 1
fi
