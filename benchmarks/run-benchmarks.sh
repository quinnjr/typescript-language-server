#!/bin/bash

# Run all benchmarks for the TypeScript Language Server
# Usage: ./run-benchmarks.sh [--full]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "============================================"
echo "TypeScript Language Server Benchmark Suite"
echo "============================================"
echo ""

# Build release version
echo "Building release version..."
cargo build --release

echo ""
echo "============================================"
echo "Criterion Micro-benchmarks"
echo "============================================"
echo ""

# Run Criterion benchmarks
if [ "$1" == "--full" ]; then
    echo "Running full benchmark suite..."
    cargo bench
else
    echo "Running quick benchmarks (use --full for complete suite)..."
    cargo bench -- --quick
fi

echo ""
echo "============================================"
echo "LSP Protocol Benchmarks"
echo "============================================"
echo ""

cd "$SCRIPT_DIR/scripts"

# Check if dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "Installing benchmark dependencies..."
    pnpm install
fi

# Generate fixtures if needed
if [ ! -f "../fixtures/large.ts" ]; then
    echo "Generating fixture files..."
    node generate-fixtures.mjs
fi

# Run LSP benchmarks
echo ""
echo "Benchmarking Rust TypeScript Language Server..."
node benchmark-lsp.mjs --server rust

# Check if tsserver is available
if command -v npx &> /dev/null && npx typescript-language-server --version &> /dev/null; then
    echo ""
    echo "Benchmarking TypeScript Language Server (tsserver)..."
    node benchmark-lsp.mjs --server tsserver

    echo ""
    echo "Running comparison benchmark..."
    node benchmark-lsp.mjs
else
    echo ""
    echo "⚠️  typescript-language-server not found."
    echo "   Install with: npm install -g typescript-language-server typescript"
    echo "   Skipping tsserver benchmarks."
fi

echo ""
echo "============================================"
echo "Benchmark Complete!"
echo "============================================"
echo ""
echo "Results:"
echo "  - Criterion reports: target/criterion/"
echo "  - LSP benchmark output: see above"
echo ""

