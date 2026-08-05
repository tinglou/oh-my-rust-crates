# hypervectorscan

[:us: English](README.md) | [:cn: 中文](README-cn.md)

Safe Rust wrapper for [Hyperscan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan) — high-performance multi-pattern regex matching.

## Usage

By default (`autobuild` feature), the build system auto-detects the best way to get the C library — no manual configuration needed.

```rust
use hypervectorscan::{BlockDatabase, Flag, Pattern, Scan};

let patterns = vec![Pattern::new("hello", Flag::CASELESS, 0)];
let db = BlockDatabase::new(patterns)?;
let mut scanner = db.create_scanner();

scanner.scan(b"Hello, world!", |id, from, to, _flags| {
    println!("matched pattern {id} at [{from}, {to})");
    Scan::Continue
})?;
```

## Features

| Feature        | Description                                        |
|----------------|----------------------------------------------------|
| `autobuild`    | Auto-detect: conan → vcpkg → system → vendored     |
| `conan2`       | Force Conan package manager                        |
| `vcpkg`        | Force vcpkg package manager                        |
| `system`       | Force system-installed library (`apt install libhyperscan-dev`) |
| `vendored`     | Force bundled source cmake build                   |
| `avx512`       | Enable AVX512 (vendored x86_64 only)               |
| `avxvnni`      | Enable AVX512VBMI (vendored x86_64 only)           |

All features are passed through to `hypervectorscan-sys`.

## API

| Module    | Key Types                                                             | Description            |
|-----------|-----------------------------------------------------------------------|------------------------|
| `native`  | `BlockDatabase`, `BlockScanner`, `StreamingDatabase`, `StreamScanner` | High-level scanning API |
| `wrapper` | `Database`, `Scratch`, `Pattern`, `SerializedDatabase`               | FFI wrapper types      |
| `error`   | `Error`, `HyperscanErrorCode`, `AsResult`                             | Error handling         |
