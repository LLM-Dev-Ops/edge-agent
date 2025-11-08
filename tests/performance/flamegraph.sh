#!/bin/bash
# Performance Profiling with Flamegraphs
# Generates CPU flamegraphs for performance analysis

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DURATION="${DURATION:-60}"  # Profiling duration in seconds
OUTPUT_DIR="${OUTPUT_DIR:-./performance-reports}"
BINARY="${BINARY:-./target/release/llm-edge-agent}"

echo "========================================="
echo "Performance Profiling (Flamegraph)"
echo "========================================="
echo "Binary: $BINARY"
echo "Duration: ${DURATION}s"
echo "Output: $OUTPUT_DIR"
echo "========================================="

mkdir -p "$OUTPUT_DIR"

# Check for required tools
if ! command -v cargo-flamegraph &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-flamegraph...${NC}"
    cargo install flamegraph
fi

# Build in release mode with debug symbols
echo -e "${YELLOW}Building with debug symbols...${NC}"
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release

# Generate flamegraph
echo -e "${YELLOW}Profiling for ${DURATION} seconds...${NC}"
echo "Starting application..."

# Run with flamegraph
sudo CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph \
    --output="$OUTPUT_DIR/flamegraph.svg" \
    --root \
    -- --duration="$DURATION"

echo -e "${GREEN}✅ Flamegraph generated: $OUTPUT_DIR/flamegraph.svg${NC}"
echo ""
echo "View the flamegraph:"
echo "  open $OUTPUT_DIR/flamegraph.svg"
echo "  # or"
echo "  firefox $OUTPUT_DIR/flamegraph.svg"
