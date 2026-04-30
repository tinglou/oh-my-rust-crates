# Oh My Rust Crates

A collection of Rust crates for common utilities and tools.

## Crates

This workspace contains the following crates:

### 1. backerror

[![crates.io](https://img.shields.io/crates/v/backerror.svg)](https://crates.io/crates/backerror)
[![docs.rs](https://docs.rs/backerror/badge.svg)](https://docs.rs/backerror)

**Java-style backtrace for Rust**

`backerror` enhances error handling by automatically capturing location information and stack traces for errors. It provides seamless integration with [thiserror](https://github.com/dtolnay/thiserror) to improve debugging capabilities by tracking where errors originate in your code.

#### Features

- **Automatic Location Tracking**: Captures source location (file, line, column) where errors originate
- **Stack Trace Capture**: Optionally captures full stack traces for debugging
- **Seamless Integration**: Works with existing `thiserror`-based error types
- **Zero-cost Abstraction**: Only incurs runtime cost when enabled (can be disabled in release builds)
- **Transparent Wrapping**: Preserves original error type while adding location metadata

#### Quick Example

```rust
use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
#[error(transparent)]
pub struct MyError(#[from] std::io::Error);

fn read_file() -> Result<(), MyError> {
    std::fs::File::open("blurb.txt")?;
    Ok(())
}
```

#### Example Output

```text
MyError: The system cannot find the file specified. (os error 2)
    at example::read_file (./src/main.rs:10:5)
    at example::main (./src/main.rs:15:12)
```

[Learn more →](backerror-rs/backerror/README.md)

---

### 2. macaddr-ouidb

[![crates.io](https://img.shields.io/crates/v/macaddr-ouidb.svg)](https://crates.io/crates/macaddr-ouidb)
[![docs.rs](https://docs.rs/macaddr-ouidb/badge.svg)](https://docs.rs/macaddr-ouidb)

**High-performance MAC Address OUI Lookup**

`macaddr-ouidb` provides fast MAC address to manufacturer (OUI - Organizationally Unique Identifier) lookup with a pre-compiled database.

#### Features

- **Zero-Cost Abstraction**: Compile-time generated lookup tables
- **Memory Efficient**: Columnar storage + compact encoding, full database ~900KB
- **Fast Lookup**: O(log n) time complexity, supports 24/28/36-bit OUI
- **Comprehensive Coverage**: 52,000+ OUI entries (from Nmap project)
- **Virtual NIC Detection**: Built-in virtualization platform NIC recognition
- **Flexible Integration**: Supports `serde` serialization, optional `pnet` interoperability
- **no_std Support**: Works in embedded environments

#### Quick Example

```rust
use macaddr_ouidb::{MacAddress, OUI_DB};

let mac = MacAddress::from_str("00:55:DA:0A:BB:CC")?;

match OUI_DB.lookup(mac) {
    Some(org_name) => println!("Manufacturer: {}", org_name),
    None => println!("OUI information not found"),
}
```

[Learn more →](macaddr-ouidb/README.md)

---

## Workspace Structure

```
oh-my-rust-crates/
├── backerror-rs/
│   ├── backerror/          # Main backerror crate
│   └── backerror-macros/   # Procedural macros for backerror
├── macaddr-ouidb/          # MAC address OUI lookup crate
└── samples/                # Example projects
    ├── backerror-demo/
    ├── backerror-nostd-demo/
    ├── mac-oui-demo/
    └── mac-oui-nostd-demo/
```

## License

All crates in this workspace are licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Issues and Pull Requests are welcome!