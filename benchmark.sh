#!/bin/bash

# Real-world benchmark script for string-escape-simd
# Uses actual JavaScript/TypeScript files from AFFiNE v0.23.2 as test data

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/target/release/affine_bench"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}String Escape SIMD - Real-World Benchmark Suite${NC}"
echo -e "${BLUE}=================================================${NC}"
echo ""

# Check if benchmark data exists
if [ ! -d "$SCRIPT_DIR/benchmark_data" ]; then
    echo -e "${RED}Error: Benchmark data not found!${NC}"
    echo ""
    echo "To set up the benchmark data, run:"
    echo ""
    echo -e "${YELLOW}  # Download AFFiNE v0.23.2 source code${NC}"
    echo "  mkdir -p /tmp/affine && cd /tmp/affine"
    echo "  curl -L 'https://github.com/toeverything/AFFiNE/archive/refs/tags/v0.23.2.tar.gz' -o affine-v0.23.2.tar.gz"
    echo "  tar -xzf affine-v0.23.2.tar.gz"
    echo ""
    echo -e "${YELLOW}  # Collect JavaScript/TypeScript files${NC}"
    echo "  mkdir -p '$SCRIPT_DIR/benchmark_data'"
    echo "  find /tmp/affine/AFFiNE-0.23.2 -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' -type f | \\"
    echo "    while IFS= read -r file; do"
    echo "      echo \"// File: \$file\" >> '$SCRIPT_DIR/benchmark_data/all_files.js'"
    echo "      cat \"\$file\" >> '$SCRIPT_DIR/benchmark_data/all_files.js'"
    echo "      echo -e \"\\n\\n\" >> '$SCRIPT_DIR/benchmark_data/all_files.js'"
    echo "    done"
    echo ""
    exit 1
fi

# Build the benchmark binary if it doesn't exist
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${YELLOW}Building benchmark binary...${NC}"
    cd "$SCRIPT_DIR"
    cargo build --release --bin affine_bench
    echo ""
fi

# Get dataset info
DATASET_SIZE=$(wc -c < "$SCRIPT_DIR/benchmark_data/all_files.js")
DATASET_MB=$(echo "scale=1; $DATASET_SIZE / 1000000" | bc -l)

echo -e "${GREEN}Dataset Information:${NC}"
echo "  Source: AFFiNE v0.23.2 JavaScript/TypeScript files"
echo "  Size: $DATASET_SIZE bytes ($DATASET_MB MB)"
echo "  Files: $(wc -l < "$SCRIPT_DIR/benchmark_data/file_list.txt" 2>/dev/null || echo "N/A")"
echo ""

# Parse command line arguments
MODE="all"
if [ $# -gt 0 ]; then
    MODE="$1"
fi

case "$MODE" in
    "all")
        echo -e "${GREEN}Running all benchmarks...${NC}"
        echo ""
        
        echo -e "${BLUE}1. Quick comparison (internal timing):${NC}"
        "$BINARY_PATH" compare
        echo ""
        
        echo -e "${BLUE}2. Hyperfine benchmark:${NC}"
        if command -v hyperfine >/dev/null 2>&1; then
            hyperfine --warmup 3 --runs 10 \
                --command-name "SIMD implementation" "$BINARY_PATH hyperfine simd" \
                --command-name "Fallback implementation" "$BINARY_PATH hyperfine fallback"
        else
            echo -e "${YELLOW}hyperfine not found. Install it with:${NC}"
            echo "  cargo install hyperfine"
            echo "  # or download from https://github.com/sharkdp/hyperfine/releases"
        fi
        ;;
        
    "compare")
        echo -e "${BLUE}Running comparison benchmark:${NC}"
        "$BINARY_PATH" compare
        ;;
        
    "hyperfine")
        echo -e "${BLUE}Running hyperfine benchmark:${NC}"
        if command -v hyperfine >/dev/null 2>&1; then
            hyperfine --warmup 3 --runs 10 \
                --command-name "SIMD implementation" "$BINARY_PATH hyperfine simd" \
                --command-name "Fallback implementation" "$BINARY_PATH hyperfine fallback"
        else
            echo -e "${RED}Error: hyperfine not found!${NC}"
            exit 1
        fi
        ;;
        
    "individual")
        echo -e "${BLUE}Running individual files benchmark:${NC}"
        "$BINARY_PATH" individual
        ;;
        
    "simd")
        echo -e "${BLUE}Benchmarking SIMD implementation only:${NC}"
        "$BINARY_PATH" simd
        ;;
        
    "fallback")
        echo -e "${BLUE}Benchmarking fallback implementation only:${NC}"
        "$BINARY_PATH" fallback
        ;;
        
    "help"|"-h"|"--help")
        echo "Usage: $0 [MODE]"
        echo ""
        echo "Modes:"
        echo "  all        - Run all benchmarks (default)"
        echo "  compare    - Compare SIMD vs fallback implementations"
        echo "  hyperfine  - Run hyperfine benchmark"
        echo "  individual - Process individual files"
        echo "  simd       - Benchmark SIMD implementation only"
        echo "  fallback   - Benchmark fallback implementation only"
        echo "  help       - Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0               # Run all benchmarks"
        echo "  $0 compare       # Quick comparison"
        echo "  $0 hyperfine     # Precise hyperfine benchmark"
        ;;
        
    *)
        echo -e "${RED}Error: Unknown mode '$MODE'${NC}"
        echo "Run '$0 help' for usage information."
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Benchmark complete!${NC}"