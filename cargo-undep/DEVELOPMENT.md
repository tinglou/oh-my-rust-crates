# Development Guide

This document contains information for contributors and maintainers of `cargo-undep`.

## Building and testing

```bash
# Format check
cargo fmt --all -- --check

# Linting (strict)
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features clap-cli -- -D warnings

# Tests
cargo test
cargo test --features clap-cli

# Documentation (no warnings)
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --features clap-cli
```

## Project structure

```
cargo-undep/
├── Cargo.toml
├── README.md
├── README-cn.md
├── src/
│   ├── main.rs           # Entry point, command dispatch
│   ├── cli.rs            # Hand-written CLI parser (default)
│   ├── cli_clap.rs       # Clap-based CLI parser (optional feature)
│   ├── workspace.rs      # Workspace discovery, member detection, reference checking
│   ├── hoist.rs          # Dependency hoisting logic
│   ├── cleaner.rs        # Comment-aware line removal
│   └── git.rs            # Git safety checks
```

## Key design decisions

### CLI
- Two subcommands: `clean` (unused deps) and `hoist` (member → workspace)
- Default (no subcommand) reports both
- `--fix` is the "apply" flag, requires a subcommand
- Global options: `--allow-dirty`, `--manifest-path`
- Hand-written parser is default; clap is behind `clap-cli` feature for binary size comparison

### Workspace discovery
- Reads `[workspace]` from root `Cargo.toml`
- Supports `members` (globs), `exclude`, and auto-discovery of path dependencies
- Root package (if present) is also a member

### Reference detection
A dependency is "used" if any member declares `workspace = true` in any dependency table (normal, dev, build, target-specific). Both inline and sub-table forms supported.

### Comment handling (cleaner.rs)
- Removes comments directly above a deleted dependency if contiguous (no blank line)
- Preserves commented-out dependency lines (`# name = value`) as boundaries
- Preserves CRLF line endings

### Hoisting logic (hoist.rs)
- Collects all member dependency declarations
- Groups by dependency key
- Merges compatible version requirements (caret semantics, max lower bound)
- Skips on: version conflict, default-features mismatch, path/git mismatch, package alias mismatch, custom registry
- Preserves member-level `features`, `optional`, `default-features`
- `--hoist --fix` also enforces inheritance for deps already in `[workspace.dependencies]`

### Version merging
Simple caret versions only (e.g., `1.0`, `1.2.5`, `^1.2`). Merges by taking the maximum lower bound within same major (or same minor for 0.x). Non-simple versions (exact, tilde, ranges) require exact match.

### Git safety
`--fix` requires clean git work tree (git available, inside repo, `git status --porcelain` empty). `--allow-dirty` bypasses.

## Adding a new test

Tests are in the same file as the module (inline `#[cfg(test)]` modules). Use the existing fixtures in `workspace.rs` as patterns.

## Releasing

```bash
# Update version in Cargo.toml
cargo publish --dry-run
cargo publish
```

## License

Apache-2.0