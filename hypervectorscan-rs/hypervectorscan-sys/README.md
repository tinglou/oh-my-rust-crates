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

## Linking

`hypervectorscan-sys` does **not** link `stdc++` — the caller (application or library with tests) is responsible.

### Recommended: `build.rs`

Add a `build.rs` to your crate. Debug builds dynamically link `stdc++`; release builds statically link it.

```rust,no_run
fn main() {
    link_stdcpp();
}

fn link_stdcpp() {
    #[cfg(target_os = "linux")]
    {
        #[cfg(debug_assertions)]
        println!("cargo:rustc-link-lib=stdc++");

        #[cfg(not(debug_assertions))]
        // Use `static=` prefix — rustc handles the `-Bstatic`/`-Bdynamic` wrapping.
        // The `-Wl,-Bstatic` approach via `cargo:rustc-link-arg` does NOT work here
        // because rustc adds its own `-lstdc++` (dynamic) before build.rs flags.
        println!("cargo:rustc-link-lib=static=stdc++");
    }
}
```

### Alternative approaches

| Approach | How | Best for |
|----------|-----|----------|
| `build.rs` (recommended) | Code above | Most projects — self-contained, debug/release aware |
| `.cargo/config.toml` | `[target.'cfg(target_os = "linux")']` `rustflags = ["-C", "link-args=-lstdc++"]` | Workspace-wide or library crate tests |
| `RUSTFLAGS` env var | `RUSTFLAGS="-C link-args=-lstdc++" cargo build` | Quick one-off builds |

Prefer `build.rs` — it ships with the project, needs no user setup, and automatically distinguishes debug from release. Users who need full control can still override via `RUSTFLAGS`.

### Priority & ordering

When multiple mechanisms are active, they **accumulate** (not override). The linker sees them in this order:

```
rustc's own system libs      -lstdc++ (dynamic)        ← always present, comes first
build.rs native-lib dirs     -Bstatic -lstdc++ -Bdynamic  ← cargo:rustc-link-lib=static=stdc++
.cargo/config.toml rustflags  -lstdc++ (dynamic)        ← appended at the end
RUSTFLAGS env var             -lstdc++ (dynamic)        ← appended last
```

`build.rs` wins for static linking because `cargo:rustc-link-lib=static=stdc++` is placed **before** the raw `-lstdc++` from config/rustflags — the linker resolves symbols from the static version first.

> **Note**: `.cargo/config.toml` is **directory-hierarchical**, not workspace-scoped. Cargo walks up from CWD, merging configs; deeper directories override shallower ones. But `rustflags` and `build.rs` directives are different channels entirely — they coexist.

## Conanfiles

- `conanfile-hyperscan.txt` — Hyperscan 5.4.2
- `conanfile-vectorscan.txt` — Vectorscan 5.4.11

## publish

`cargo publish --dry-run  --features autobuild`
