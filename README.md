# Oh My Rust Crates

[:us: English](README.md) | [:cn: 中文](README-cn.md)

A collection of focused Rust crates for diagnostics, networking, native build integration, and workspace tooling.

## Key capabilities

| Crate | What it gives you |
|-------|-------------------|
| [backerror](backerror-rs/backerror/README.md) | Java-style error locations and stack traces built on `thiserror` |
| [macaddr-ouidb](mac-oui/macaddr-ouidb/README.md) | Fast, embedded MAC OUI lookup with region data, virtual NIC detection, and `no_std` support |
| [conan2-rs](conan2-rs/README.md) | Conan 2 integration for Cargo build scripts and FFI projects |
| [hypervectorscan](hypervectorscan-rs/hypervectorscan/README.md) | Safe, high-level Rust bindings for Hyperscan / Vectorscan multi-pattern matching |
| [init-log4rs](init-log4rs/README.md) | Zero-config `log4rs` discovery, creation, and initialization |
| [cargo-undep](cargo-undep/README.md) | Find unused workspace dependencies and hoist member dependency sources safely |

## Crates

### backerror

**Java-style backtraces for Rust errors**

`backerror` captures source locations and optional stack traces when errors are created. It integrates with [thiserror](https://github.com/dtolnay/thiserror), preserving the original error type while making propagation easier to debug.

See [backerror-rs/backerror/README.md](backerror-rs/backerror/README.md) for usage, feature flags, and output examples.

### macaddr-ouidb

**High-performance MAC address OUI lookup**

`macaddr-ouidb` performs fast MAC-to-manufacturer lookups from a precompiled database. It includes region information, virtual NIC detection, multiple MAC formats, and `no_std` support.

See [mac-oui/macaddr-ouidb/README.md](mac-oui/macaddr-ouidb/README.md) for the API, feature flags, and data coverage.

### conan2-rs

**Conan 2 integration for Cargo build scripts**

`conan2-rs` provides a Rust interface to the [Conan 2](https://docs.conan.io/2/) package manager, so FFI crates can install native C/C++ dependencies directly from `build.rs`.

See [conan2-rs/README.md](conan2-rs/README.md) for a complete `build.rs` example and API overview.

### hypervectorscan

**Safe Rust wrapper for Hyperscan / Vectorscan**

`hypervectorscan` exposes a safe, high-level API for [Hyperscan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan). It supports block, streaming, and vectored scanning, with automatic C library detection through conan, vcpkg, the system, or vendored builds.

See [hypervectorscan-rs/hypervectorscan/README.md](hypervectorscan-rs/hypervectorscan/README.md) for examples, features, and linking notes.

### init-log4rs

**Zero-config log4rs initialization**

`init-log4rs` discovers an existing `log4rs` configuration or creates a sensible default, including console output and rotating file logs. Optional `tracing` support bridges spans and events into the same pipeline.

See [init-log4rs/README.md](init-log4rs/README.md) for setup and configuration details.

### cargo-undep

**Workspace dependency cleanup and hoisting**

`cargo-undep` reports and removes unused `[workspace.dependencies]`, and can hoist compatible member dependency sources into the workspace while preserving member-level attributes such as `features`, `optional`, and `default-features`.

See [cargo-undep/README.md](cargo-undep/README.md) for commands, examples, and safety rules.

## Development

Build, test, and release instructions live in [DEVELOPMENT.md](DEVELOPMENT.md). Contributions are welcome.

## License

All crates in this workspace are licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
