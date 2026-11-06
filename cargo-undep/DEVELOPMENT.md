# Development Guide

This document contains information for contributors and maintainers of `cargo-undep`.

## Building and testing

```bash
# Format check
cargo fmt -p cargo-undep -- --check

# Linting (strict)
cargo clippy -p cargo-undep --all-targets -- -D warnings

# Tests
cargo test -p cargo-undep

# Documentation (no warnings)
RUSTDOCFLAGS='-D warnings' cargo doc -p cargo-undep --no-deps
```

## Project structure

```
cargo-undep/
├── Cargo.toml
├── README.md
├── README-cn.md
├── src/
│   ├── main.rs           # Entry point, command dispatch
│   ├── cli.rs            # Clap-based CLI parser
│   ├── workspace.rs      # Workspace discovery, member detection, reference checking
│   ├── hoist.rs          # Dependency hoisting logic
│   ├── sort.rs           # Category-based dependency sorting
│   ├── cleaner.rs        # Comment-aware line removal
│   └── git.rs            # Git safety checks
```

## Key design decisions

### CLI
- Three subcommands: `clean` (unused deps), `hoist` (member → workspace), and `sort` (dependency table ordering)
- Default (no subcommand) reports both unused and hoistable dependencies
- `--fix` is the "apply" flag, requires a subcommand
- Global options: `--allow-dirty`, `--manifest-path`
- Clap owns command-line parsing

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

### Sorting logic (sort.rs)
- Targets `[workspace.dependencies]` and each member's `[dependencies]` only
- Tables with fewer than 10 entries are left untouched
- A category written in a dependency's trailing comment (`# category = ...`) overrides every other rule
- Path dependencies form the first `path` group
- Other dependencies are grouped by the first crate category from `cargo metadata`
- Categories are normalized to the segment before `::` (for example, `development-tools::profiling` becomes `development-tools`)
- Missing categories go into the final `misc` group
- Entries within a group are sorted by dependency key
- Each group starts with a `# --- category ---` comment
- `toml_edit` moves each entry together with contiguous comments above it and any inline comment after it
- The operation is idempotent; previously generated group comments are replaced

### Version merging
Simple caret versions only (e.g., `1.0`, `1.2.5`, `^1.2`). Merges by taking the maximum lower bound within same major (or same minor for 0.x). Non-simple versions (exact, tilde, ranges) require exact match.

### Git safety
`clean --fix` and `hoist --fix` require a clean git work tree (git available, inside repo, `git status --porcelain` empty). `sort --fix` only refuses when a `Cargo.toml` has uncommitted changes. `--allow-dirty` bypasses these checks.

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