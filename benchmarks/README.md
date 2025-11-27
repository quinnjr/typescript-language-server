# TypeScript Language Server Benchmarks

This directory contains benchmarks to compare the performance of the Rust TypeScript Language Server against the official TypeScript Language Server (tsserver).

## Benchmark Types

### 1. Rust Micro-benchmarks (Criterion)

Low-level performance tests for parsing and analysis:

```bash
# Run all Criterion benchmarks
cargo bench

# Run specific benchmark
cargo bench parsing
cargo bench analysis

# Generate HTML reports
cargo bench -- --verbose
# Reports are in target/criterion/
```

### 2. LSP Protocol Benchmarks

End-to-end benchmarks comparing both servers via the LSP protocol:

```bash
cd benchmarks/scripts

# Install dependencies
pnpm install

# Generate additional fixture files
node generate-fixtures.mjs

# Run benchmarks (requires both servers to be available)
node benchmark-lsp.mjs

# Benchmark only Rust server
node benchmark-lsp.mjs --server rust

# Benchmark only tsserver
node benchmark-lsp.mjs --server tsserver
```

## Prerequisites

1. **Build the Rust server in release mode:**
   ```bash
   cargo build --release
   ```

2. **Install the TypeScript Language Server:**
   ```bash
   npm install -g typescript-language-server typescript
   ```

## Fixture Files

The `fixtures/` directory contains TypeScript files of various sizes:

| File | Description | Lines |
|------|-------------|-------|
| `small.ts` | Simple file with basic types | ~50 |
| `medium.ts` | Moderate complexity with generics | ~300 |
| `large.ts` | Many interfaces, classes, functions | ~800 |
| `xlarge.ts` | Large file for stress testing | ~2000 |
| `huge.ts` | Very large file | ~4000 |

Generate larger fixtures:
```bash
node benchmarks/scripts/generate-fixtures.mjs
```

## Benchmark Metrics

### Parsing Benchmarks
- **Parse time**: Time to parse TypeScript source to AST
- **Incremental parse**: Time to re-parse after small edit
- **Tree traversal**: Time to walk the entire AST

### LSP Benchmarks
- **Initialize**: Server startup and initialization
- **Document open**: Time to open and process a document
- **Document symbols**: Extract outline/symbol information
- **Semantic tokens**: Generate syntax highlighting data
- **Hover**: Get hover information at a position
- **Go to definition**: Find definition of symbol
- **Find references**: Find all references to symbol

## Interpreting Results

Results show:
- **Mean**: Average time across all iterations
- **Median**: Middle value (less affected by outliers)
- **Min/Max**: Best and worst case times
- **Speedup**: How many times faster Rust is vs tsserver

Example output:
```
================================================================================
Benchmark                                   Mean       Median         Min         Max
--------------------------------------------------------------------------------
[Rust LSP] Open: small.ts (50 lines)      2.34ms     2.12ms      1.98ms      3.45ms
[tsserver] Open: small.ts (50 lines)     45.67ms    44.23ms     42.11ms     52.34ms
================================================================================

Comparison Summary:
------------------------------------------------------------
Open: small.ts (50 lines)                🚀 19.52x faster
```

## Expected Results

Based on typical benchmarks, you can expect:

| Operation | Rust vs tsserver |
|-----------|------------------|
| Parsing | 5-20x faster |
| Document symbols | 3-10x faster |
| Semantic tokens | 5-15x faster |
| Hover | 2-5x faster |
| Memory usage | 3-10x less |

Note: Actual results depend on hardware, file size, and complexity.

## Profiling

For detailed profiling:

```bash
# CPU profiling with perf (Linux)
perf record --call-graph dwarf cargo bench parsing
perf report

# Memory profiling with heaptrack
heaptrack cargo bench parsing

# Flamegraph
cargo install flamegraph
cargo flamegraph --bench parsing
```

## Contributing

When adding new benchmarks:

1. Add Criterion benchmarks in `benches/`
2. Add LSP test cases in `benchmarks/scripts/benchmark-lsp.mjs`
3. Update fixture files if needed
4. Document expected performance characteristics

