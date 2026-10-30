# Development

This document covers building, testing, and releasing the crates in this workspace.

## Prerequisites

- Rust stable with Edition 2024 support
- Cargo
- Git
- Optional native toolchains for `hypervectorscan`: Conan, vcpkg, or a system Hyperscan/Vectorscan installation

## Workspace layout

The Cargo workspace includes:

- `backerror-rs/backerror` and `backerror-rs/backerror-macros`
- `mac-oui/macaddr-ouidb` and `mac-oui/macaddr-ouidb-support`
- `conan2-rs`
- `hypervectorscan-rs/hypervectorscan`, `hypervectorscan-sys`, `hyperscan-src`, and `vectorscan-src`
- `init-log4rs`
- `cargo-undep`

The `samples` and `erfs` directories are excluded from the workspace.

## Build and test

Run commands from the repository root:

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Some crates depend on native libraries or platform toolchains. If a full workspace build is not available on your machine, target the crate you are changing:

```bash
cargo test -p cargo-undep
cargo test -p cargo-undep --features clap-cli
cargo clippy -p cargo-undep --all-targets -- -D warnings
```

## Releasing

Releases use `cargo-release` and `git-cliff`. See the detailed release guide in [release.md](release.md).

## Contributing

Issues and pull requests are welcome. Keep changes focused, run the relevant formatting, test, and lint commands before submitting, and update the matching English and Chinese README files when user-facing behavior changes.
