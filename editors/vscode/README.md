# TypeScript/JavaScript LSP (Rust) - VSCode Extension

A VSCode extension that provides TypeScript and JavaScript language support using a high-performance Rust-based language server.

## Features

- **IntelliSense** - Code completions with type information
- **Go to Definition** - Navigate to symbol definitions
- **Find References** - Find all usages of a symbol
- **Rename Symbol** - Rename across your project
- **Hover Information** - Type info and documentation on hover
- **Diagnostics** - Syntax and type error reporting
- **Semantic Highlighting** - Rich syntax highlighting
- **Code Actions** - Quick fixes and refactorings
- **Inlay Hints** - Type and parameter annotations
- **Signature Help** - Function parameter hints

## Requirements

- VSCode 1.75.0 or newer
- The Rust language server binary (built from the parent project)

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `tsLspRust.enable` | `true` | Enable the language server |
| `tsLspRust.serverPath` | `""` | Path to the language server binary |
| `tsLspRust.trace.server` | `"off"` | Trace communication with server |

## Development

### Prerequisites

- Node.js 18+
- pnpm
- Rust toolchain (for building the server)

### Building

```bash
# Install dependencies
pnpm install

# Build the extension
pnpm run build

# Build in watch mode
pnpm run bundle:watch
```

### Running

1. Open this folder in VSCode
2. Press `F5` to launch the Extension Development Host
3. The extension will automatically find the language server in `../../target/debug/`

### Testing

```bash
# Run tests
pnpm test

# Run tests with coverage
pnpm test:coverage
```

### Packaging

```bash
# Create .vsix package
pnpm run package
```

## Commands

| Command | Description |
|---------|-------------|
| `TypeScript LSP (Rust): Restart Language Server` | Restart the language server |

## Troubleshooting

### Server not found

If you see "Could not find language server", either:

1. Build the server: `cargo build` in the project root
2. Set `tsLspRust.serverPath` in settings to point to your server binary

### View logs

1. Open the Output panel (`View > Output`)
2. Select "TypeScript/JavaScript LSP (Rust)" from the dropdown

## License

MIT

