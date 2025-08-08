# Real-World Benchmarking with AFFiNE Dataset

This directory contains a comprehensive benchmark suite that uses real JavaScript/TypeScript code from the [AFFiNE v0.23.2 release](https://github.com/toeverything/AFFiNE/releases/tag/v0.23.2) to evaluate JSON string escaping performance.

## Why AFFiNE?

AFFiNE is a modern, production TypeScript/JavaScript codebase that provides:

- **Real-world complexity**: 6,448 source files totaling ~22MB
- **Diverse content**: Mix of TypeScript, React JSX, configuration files
- **Realistic escaping scenarios**: Actual strings, comments, and code patterns found in production
- **Large scale**: Sufficient data volume to trigger SIMD optimizations

## Dataset Characteristics

- **Source**: AFFiNE v0.23.2 JavaScript/TypeScript files
- **File count**: 6,448 files (.js, .jsx, .ts, .tsx)
- **Total size**: ~22MB of source code
- **Content types**: 
  - React components with JSX
  - TypeScript interfaces and types
  - Configuration files
  - Test files
  - Documentation

## Quick Start

### 1. Automatic Setup
```bash
# Run the benchmark script - it will guide you through setup
./benchmark.sh
```

### 2. Manual Setup
```bash
# Download AFFiNE v0.23.2
mkdir -p /tmp/affine && cd /tmp/affine
curl -L "https://github.com/toeverything/AFFiNE/archive/refs/tags/v0.23.2.tar.gz" -o affine-v0.23.2.tar.gz
tar -xzf affine-v0.23.2.tar.gz

# Collect JavaScript/TypeScript files
mkdir -p benchmark_data
find /tmp/affine/AFFiNE-0.23.2 -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" -type f | \
  while IFS= read -r file; do
    echo "// File: $file" >> benchmark_data/all_files.js
    cat "$file" >> benchmark_data/all_files.js
    echo -e "\n\n" >> benchmark_data/all_files.js
  done

# Create file list for individual processing
find /tmp/affine/AFFiNE-0.23.2 -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" -type f > benchmark_data/file_list.txt
```

### 3. Run Benchmarks
```bash
# Quick comparison
./benchmark.sh compare

# Hyperfine benchmark (requires hyperfine)
./benchmark.sh hyperfine

# All benchmarks
./benchmark.sh all
```

## Benchmark Modes

### 1. Quick Comparison (`compare`)
Uses internal timing to compare SIMD vs fallback implementations:
```bash
cargo run --release --bin affine_bench -- compare
# or
./benchmark.sh compare
```

### 2. Hyperfine Benchmark (`hyperfine`)
Uses the `hyperfine` tool for precise, statistical benchmarking:
```bash
hyperfine --warmup 3 --runs 10 \
  './target/release/affine_bench hyperfine simd' \
  './target/release/affine_bench hyperfine fallback'
# or
./benchmark.sh hyperfine
```

### 3. Individual Files (`individual`)
Processes each file separately to measure cumulative performance:
```bash
cargo run --release --bin affine_bench -- individual
# or
./benchmark.sh individual
```

### 4. Single Implementation Testing
Test specific implementations in isolation:
```bash
# SIMD only
./benchmark.sh simd

# Fallback only  
./benchmark.sh fallback
```

## Binary Usage

The `affine_bench` binary provides several modes:

```bash
# Build the binary
cargo build --release --bin affine_bench

# Usage
./target/release/affine_bench <mode> [options]

# Modes:
#   simd           - Benchmark optimized SIMD implementation
#   fallback       - Benchmark fallback implementation  
#   compare        - Compare both implementations
#   individual     - Process individual files from AFFiNE
#   hyperfine      - Silent mode for hyperfine benchmarking
```

## Installing Hyperfine

### Option 1: Package Manager
```bash
# Debian/Ubuntu
sudo apt install hyperfine

# macOS
brew install hyperfine

# Arch Linux
pacman -S hyperfine
```

### Option 2: Cargo
```bash
cargo install hyperfine
```

### Option 3: Direct Download
```bash
# Linux x86_64
curl -L https://github.com/sharkdp/hyperfine/releases/download/v1.18.0/hyperfine-v1.18.0-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv hyperfine-v1.18.0-x86_64-unknown-linux-gnu/hyperfine /usr/local/bin/
```

## Expected Results

### On x86_64
Both implementations should perform similarly since the SIMD optimizations are aarch64-specific:

```
SIMD implementation:      38.5 ms ± 0.5 ms
Fallback implementation:  38.6 ms ± 0.2 ms
Result: Equivalent performance (expected)
```

### On aarch64 (Apple Silicon, AWS Graviton, etc.)
The SIMD implementation should show significant improvements:

```
SIMD implementation:      25.2 ms ± 0.3 ms  
Fallback implementation:  38.6 ms ± 0.2 ms
Result: SIMD is 53% faster
```

## Data File Structure

```
benchmark_data/
├── all_files.js      # All JS/TS files concatenated (22MB)
└── file_list.txt     # List of original file paths (6,448 lines)
```

The `all_files.js` contains all source files with headers indicating the original file path:

```javascript
// File: /tmp/affine/AFFiNE-0.23.2/vitest.config.ts
import { resolve } from 'node:path';
// ... file content ...


// File: /tmp/affine/AFFiNE-0.23.2/packages/common/infra/src/index.ts
export * from './framework';
// ... file content ...
```

## Performance Insights

This real-world benchmark reveals:

1. **Large file handling**: How the library performs with production-scale codebases
2. **Mixed content patterns**: Performance across different JavaScript/TypeScript constructs  
3. **Memory efficiency**: Behavior with substantial string processing workloads
4. **SIMD effectiveness**: Real-world impact of vectorized processing

The AFFiNE dataset is ideal because it contains the complex, nested string patterns found in modern web applications, making it a much more realistic test than synthetic benchmarks.