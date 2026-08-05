# hypervectorscan-sys

[:us: English](README.md) | :cn: 中文

Raw FFI bindings to [Hyperscan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan).

## Build modes

| Feature            | 说明                                              | 系统依赖                               |
|--------------------|---------------------------------------------------|----------------------------------------|
| `autobuild`        | 自动选择：conan2 → vcpkg → system → vendored      | 无需手动指定                           |
| `conan2`           | 强制 Conan 包管理器                                | conan CLI                              |
| `vcpkg`            | 强制 vcpkg 包管理器                                | vcpkg                                  |
| `system`           | 强制系统已安装库（pkg-config）                     | pkg-config + libhs-dev / vectorscan-dev |
| `vendored`         | 强制 bundled 源码 cmake 编译                       | cmake, C++ 编译器, ragel, boost-dev    |
| `avx512`           | 启用 AVX512（仅 vendored x86_64）                  | —                                      |
| `avxvnni`          | 启用 AVX512VBMI（仅 vendored x86_64）              | —                                      |

### autobuild 优先级

```
检测 conan  →  有 → conan2 构建
     ↓ 无
检测 vcpkg  →  有 → vcpkg 构建
     ↓ 无
检测 system (pkg-config) → 有 → system 构建
     ↓ 无
vendored (bundled 源码 cmake 编译)
```

### system 模式支持

| 操作系统      | system 模式 | 安装方式                         |
|---------------|:-----------:|----------------------------------|
| **Linux**     | ✅          | `apt install libhyperscan-dev`   |
| **macOS**     | ✅          | `brew install hyperscan`         |
| **Windows**   | ❌ (自动跳过) | 使用 vcpkg 代替                  |

## Supported targets

| 架构               | Hyperscan | Vectorscan | vendored 后端    |
|--------------------|:--:|:--:|------------------|
| x86_64             | ✅ | ✅ | hyperscan-src    |
| aarch64/arm64      | ❌ | ✅ | vectorscan-src   |
| ppc64/riscv/其他    | ❌ | ❌ | vectorscan-src (SIMDe) |

## Backend selection

`build.rs` 根据 `TARGET` 环境变量自动选择：

- **x86_64**: Hyperscan（所有模式）
  - Linux x86_64 的预编译包包含 AVX-512 指令，可能在不支持的 CPU 上 SIGILL → 强制 FAT_RUNTIME 源码编译
- **非 x86_64**: Vectorscan（Hyperscan 仅支持 x86_64）

## Conanfiles

- `conanfile-hyperscan.txt` — Hyperscan 5.4.2
- `conanfile-vectorscan.txt` — Vectorscan 5.4.11

## publish

`cargo publish --dry-run  --features autobuild`
