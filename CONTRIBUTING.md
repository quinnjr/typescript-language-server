# Contributing to TypeScript Language Server (Rust)

Thank you for your interest in contributing! This document explains our development process.

## Git Flow Workflow

This project uses **git-flow** for branch management. All contributions must follow this workflow.

### Branch Structure

```
main          ← Production releases only
  ↑
develop       ← Integration branch
  ↑
feature/*     ← New features
bugfix/*      ← Bug fixes
```

### Branch Types

| Branch | Base | Merges Into | Purpose |
|--------|------|-------------|---------|
| `feature/*` | `develop` | `develop` | New features |
| `bugfix/*` | `develop` | `develop` | Bug fixes |
| `release/*` | `develop` | `main` + `develop` | Release preparation |
| `hotfix/*` | `main` | `main` + `develop` | Production fixes |

### Creating a Feature

```bash
# Start from develop
git checkout develop
git pull origin develop

# Create feature branch
git checkout -b feature/my-feature

# Make changes, commit
git add .
git commit -m "feat(scope): description"

# Push and create PR to develop
git push -u origin feature/my-feature
```

### Creating a Bug Fix

```bash
git checkout develop
git pull origin develop
git checkout -b bugfix/issue-123-description

# Fix the bug, commit
git commit -m "fix(scope): description"

# Push and create PR to develop
git push -u origin bugfix/issue-123-description
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `test` | Adding tests |
| `chore` | Maintenance tasks |
| `ci` | CI/CD changes |
| `build` | Build system changes |

### Examples

```
feat(parser): add support for JSX fragments
fix(completions): handle undefined symbol table
docs(readme): update installation instructions
refactor(types): simplify assignability checks
perf(binder): cache scope lookups
test(hover): add tests for JSDoc extraction
chore(deps): update tree-sitter to v0.22
ci(release): add arm64 linux build
```

## Pull Request Process

1. **Create a branch** following git-flow naming
2. **Make your changes** with conventional commits
3. **Run tests** locally: `cargo test`
4. **Run linter**: `cargo clippy`
5. **Format code**: `cargo fmt`
6. **Create PR** to the correct target branch
7. **Wait for CI** to pass
8. **Address review feedback**
9. **Squash and merge** (maintainers)

## Development Setup

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for VSCode extension)
- pnpm (for VSCode extension)

### Building

```bash
# Build the language server
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### VSCode Extension

```bash
cd editors/vscode
pnpm install
pnpm run build

# Launch extension host (press F5 in VSCode)
```

## Code Style

### Rust

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document public APIs with doc comments

### TypeScript

- Use ESLint configuration provided
- Use Prettier for formatting (via ESLint)

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### TypeScript Tests

```bash
cd editors/vscode
pnpm test
pnpm test:coverage
```

## Reporting Issues

- Use GitHub Issues
- Include reproduction steps
- Include error messages
- Include environment info (OS, Rust version, etc.)

## Feature Requests

- Open a GitHub Issue with `[Feature]` prefix
- Describe the use case
- Provide examples if possible

## Questions?

Open a Discussion on GitHub or reach out to maintainers.

---

Thank you for contributing! 🎉

