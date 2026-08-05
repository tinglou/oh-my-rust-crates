# hypervectorscan

[:us: English](README.md) | :cn: 中文

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

| Feature        | 说明                                              |
|----------------|---------------------------------------------------|
| `autobuild`    | 自动检测 conan → vcpkg → system → vendored        |
| `conan2`       | 强制 Conan 包管理器                                |
| `vcpkg`        | 强制 vcpkg 包管理器                                |
| `system`       | 强制系统已安装库 (`apt install libhyperscan-dev`)  |
| `vendored`     | 强制 bundled 源码 cmake 编译                       |
| `avx512`       | 启用 AVX512（需 vendored x86_64）                  |
| `avxvnni`      | 启用 AVX512VBMI（需 vendored x86_64）              |

All features are passed through to `hypervectorscan-sys`.

## API

| Module    | Key Types                                                             | Description          |
|-----------|-----------------------------------------------------------------------|----------------------|
| `native`  | `BlockDatabase`, `BlockScanner`, `StreamingDatabase`, `StreamScanner` | High-level scanning API |
| `wrapper` | `Database`, `Scratch`, `Pattern`, `SerializedDatabase`               | FFI wrapper types    |
| `error`   | `Error`, `HyperscanErrorCode`, `AsResult`                             | Error handling       |
