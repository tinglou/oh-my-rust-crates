# Oh My Rust Crates

A collection of Rust crates for common utilities and tools.

## Crates

This workspace contains the following crates:

### 1. backerror

**Java-style backtrace for Rust**

`backerror` enhances error handling by automatically capturing location information and stack traces for errors. It provides seamless integration with [thiserror](https://github.com/dtolnay/thiserror) to improve debugging capabilities by tracking where errors originate in your code.

See [backerror-rs/backerror/README.md](backerror-rs/backerror/README.md) for more details.

### 2. macaddr-ouidb

**High-performance MAC Address OUI Lookup**

`macaddr-ouidb` provides fast MAC address to manufacturer (OUI - Organizationally Unique Identifier) lookup with a pre-compiled database.

See [macaddr-ouidb/README.md](macaddr-ouidb/README.md) for more details.

### 3. conan2-rs

**Conan 2 integration for Cargo build scripts**

`conan2-rs` provides a Rust interface to the [Conan 2](https://docs.conan.io/2/) C/C++ package manager, making it easy to manage native C/C++ dependencies directly from `build.rs` in FFI projects.

See [conan2-rs/README.md](conan2-rs/README.md) for more details.

## License

All crates in this workspace are licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Issues and Pull Requests are welcome!