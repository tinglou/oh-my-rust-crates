# hypervectorscan-sys

[:us: English](README.md) | [:cn: 中文](README-cn.md)

Raw FFI bindings to [Hyperscan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan).

## Build modes

| Feature            | Description                                        | System Dependencies                    |
|--------------------|----------------------------------------------------|----------------------------------------|
| `autobuild`        | Auto-detect: conan2 → vcpkg → system → vendored    | None required                          |
| `conan2`           | Force Conan package manager                        | conan CLI                              |
| `vcpkg`            | Force vcpkg package manager                        | vcpkg                                  |
| `system`           | Force system-installed library (pkg-config)        | pkg-config + libhs-dev / vectorscan-dev |
| `vendored`         | Force bundled source cmake build                   | cmake, C++ compiler, ragel, boost-dev  |
| `avx512`           | Enable AVX512 (vendored x86_64 only)               | —                                      |
| `avxvnni`          | Enable AVX512VBMI (vendored x86_64 only)           | —                                      |

### autobuild priority

```
detect conan  →  found → conan2 build
     ↓ not found
detect vcpkg  →  found → vcpkg build
     ↓ not found
detect system (pkg-config) → found → system build
     ↓ not found
vendored (bundled source cmake build)
```

### system mode support

| OS           | system mode | Installation                     |
|--------------|:-----------:|----------------------------------|
| **Linux**    | ✅          | `apt install libhyperscan-dev`   |
| **macOS**    | ✅          | `brew install hyperscan`         |
| **Windows**  | ❌ (skipped) | Use vcpkg instead               |

## Supported targets

| Architecture        | Hyperscan | Vectorscan | vendored backend  |
|---------------------|:--:|:--:|--------------------|
| x86_64              | ✅ | ✅ | hyperscan-src      |
| aarch64/arm64       | ❌ | ✅ | vectorscan-src     |
| ppc64/riscv/others  | ❌ | ❌ | vectorscan-src (SIMDe) |

## Backend selection

`build.rs` auto-selects based on the `TARGET` environment variable:

- **x86_64**: Hyperscan (all modes)
  - Prebuilt packages for Linux x86_64 include AVX-512 instructions, which may SIGILL on unsupported CPUs → force FAT_RUNTIME source build
- **Non-x86_64**: Vectorscan (Hyperscan only supports x86_64)

## Conanfiles

- `conanfile-hyperscan.txt` — Hyperscan 5.4.2
- `conanfile-vectorscan.txt` — Vectorscan 5.4.11

## publish

`cargo publish --dry-run  --features autobuild`
