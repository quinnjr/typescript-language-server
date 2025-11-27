# TypeScript Language Server (Rust)

A high-performance Language Server Protocol (LSP) implementation for TypeScript and JavaScript, written in Rust.

[![CI](https://github.com/quinnjr/typescript-language-server/actions/workflows/ci.yml/badge.svg)](https://github.com/quinnjr/typescript-language-server/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/quinnjr/typescript-language-server/graph/badge.svg?token=CFjVJL5yQv)](https://codecov.io/gh/quinnjr/typescript-language-server)

## Features

### Language Support
- **TypeScript** (`.ts`)
- **TypeScript React** (`.tsx`)
- **JavaScript** (`.js`, `.mjs`, `.cjs`)
- **JavaScript React** (`.jsx`)

### LSP Capabilities

| Feature | Status | Description |
|---------|--------|-------------|
| Document Symbols | ✅ | Outline view with functions, classes, variables |
| Semantic Tokens | ✅ | Rich syntax highlighting |
| Diagnostics | ✅ | Syntax errors and type diagnostics |
| Hover | ✅ | Type information and JSDoc comments |
| Folding Ranges | ✅ | Collapse functions, classes, imports |
| Selection Range | ✅ | Smart expand/shrink selection |
| Go to Definition | ✅ | Navigate to symbol definitions |
| Find References | ✅ | Find all usages of a symbol |
| Rename Symbol | ✅ | Rename across scope |
| Completions | ✅ | IntelliSense with built-ins and scope |
| Signature Help | ✅ | Function parameter hints |
| Inlay Hints | ✅ | Type and parameter annotations |
| Code Actions | ✅ | Quick fixes and refactorings |

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-username/typescript-language-server.git
cd typescript-language-server

# Build the language server
cargo build --release

# The binary will be at target/release/typescript-language-server
```

### VSCode Extension

```bash
cd editors/vscode
pnpm install
pnpm exec tsc
```

Then press `F5` in VSCode to launch the extension development host.

## Usage

### Command Line

```bash
# Start the language server (communicates via stdio)
./target/release/typescript-language-server
```

### With VSCode

1. Open the `editors/vscode` folder in VSCode
2. Run `pnpm install` to install dependencies
3. Press `F5` to launch the Extension Development Host
4. Open any TypeScript or JavaScript file

### With Other Editors

The language server uses standard LSP over stdio. Configure your editor's LSP client to run:

```bash
/path/to/typescript-language-server
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    LSP Server (tower-lsp)               │
└─────────────────────────────┬───────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────┐
│                    Project System                        │
│              (tsconfig, file graph, workspace)           │
└─────────────────────────────┬───────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────┐
│                    Type Checker                          │
│         (inference, narrowing, assignability)            │
└─────────────────────────────┬───────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────┐
│                    Binder / Symbol Table                 │
│              (scopes, symbols, references)               │
└─────────────────────────────┬───────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────┐
│                    Module Resolver                       │
│         (imports, node_modules, path mapping)            │
└─────────────────────────────┬───────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────┐
│                    Parser (tree-sitter)                  │
│                    (AST, syntax tree)                    │
└─────────────────────────────────────────────────────────┘
```

### Project Structure

```
typescript-language-server/
├── src/
│   ├── main.rs              # Entry point
│   ├── server.rs            # LSP server implementation
│   ├── document.rs          # Document management
│   ├── parser.rs            # tree-sitter parsing
│   ├── analysis/            # Symbol table & binder
│   │   ├── scope.rs         # Scope tree
│   │   ├── symbol.rs        # Symbol definitions
│   │   ├── symbol_table.rs  # Per-file symbol storage
│   │   └── binder.rs        # AST → symbols
│   ├── capabilities/        # LSP features
│   │   ├── completions.rs
│   │   ├── diagnostics.rs
│   │   ├── hover.rs
│   │   ├── definition.rs
│   │   └── ...
│   ├── resolution/          # Module resolution
│   │   ├── resolver.rs
│   │   ├── tsconfig.rs
│   │   └── node_modules.rs
│   ├── project/             # Project system
│   │   ├── project.rs
│   │   ├── file_graph.rs
│   │   └── workspace.rs
│   └── types/               # Type system
│       ├── types.rs         # Type representations
│       ├── checker.rs       # Type checker
│       └── printer.rs       # Type display
├── editors/
│   └── vscode/              # VSCode extension
├── benches/                 # Criterion benchmarks
└── benchmarks/              # LSP comparison benchmarks
```

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+
- pnpm (for VSCode extension)

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Testing

```bash
# Run all Rust tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run VSCode extension tests
cd editors/vscode
pnpm test
```

### Code Coverage

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --html

# View report
open target/llvm-cov/html/index.html
```

### Benchmarks

```bash
# Run Rust benchmarks
cargo bench

# Run LSP comparison benchmarks
cd benchmarks
./run-benchmarks.sh
```

## Configuration

The language server respects `tsconfig.json` / `jsconfig.json` files:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}
```

### Supported Options

- `compilerOptions.target`
- `compilerOptions.module`
- `compilerOptions.moduleResolution` (node, node16, nodenext, bundler)
- `compilerOptions.baseUrl`
- `compilerOptions.paths`
- `compilerOptions.strict` and related flags
- `include` / `exclude` patterns
- `extends` for configuration inheritance

## Performance

This implementation uses:
- **tree-sitter** for fast, incremental parsing
- **DashMap** for concurrent document storage
- **tower-lsp** for async LSP handling

Benchmarks show significant improvements over the official TypeScript language server for parsing and symbol resolution operations.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes
- Add tests for new features
- Update documentation as needed

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [tower-lsp](https://github.com/ebkalderon/tower-lsp) - LSP server framework
- [tree-sitter](https://tree-sitter.github.io/) - Incremental parsing
- [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) - TypeScript grammar
- [TypeScript](https://www.typescriptlang.org/) - Language specification reference

