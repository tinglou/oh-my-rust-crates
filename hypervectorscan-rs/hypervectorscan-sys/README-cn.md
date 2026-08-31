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

## Linking

`hypervectorscan-sys` **不**链接 `stdc++` — 由调用方（应用程序或有测试的库）负责。

### 推荐方式：`build.rs`

在 crate 中添加 `build.rs`，Debug 动态链接，Release 静态链接。

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
        {
            // 使用 `static=` 前缀 — rustc 会正确处理 `-Bstatic`/`-Bdynamic` 包裹。
            // `cargo:rustc-link-arg` 的 `-Wl,-Bstatic` 方式无效，因为 rustc 自己的
            // `-lstdc++`（动态）会在 build.rs 的 flags 之前出现。
            println!("cargo:rustc-link-lib=static=stdc++");
            println!("cargo:rustc-link-lib=static=gcc_eh");
            println!("cargo:rustc-link-lib=static=gcc");
        }
    }
}
```

### 其他方式

| 方式 | 用法 | 适用场景 |
|------|------|----------|
| `build.rs`（推荐） | 如上代码 | 大多数项目 — 自包含，自动区分 debug/release |
| `.cargo/config.toml` | `[target.'cfg(target_os = "linux")']` `rustflags = ["-C", "link-args=-lstdc++"]` | 工作区级别、库 crate 自身测试 |
| `RUSTFLAGS` 环境变量 | `RUSTFLAGS="-C link-args=-lstdc++" cargo build` | 一次性构建 |

推荐 `build.rs` — 随项目发布，无需用户额外配置，自动区分 debug/release。

### 优先级与时序

多种机制同时存在时，**叠加生效**而非覆盖。链接器按以下顺序看到它们：

```
rustc 自带的系统库            -lstdc++ (动态)          ← 始终存在，最先出现
build.rs 的 native-lib 指令   -Bstatic -lstdc++ -lgcc_eh -lgcc -Bdynamic  ← cargo:rustc-link-lib=static=...
.cargo/config.toml rustflags  -lstdc++ (动态)          ← 追加在末尾
RUSTFLAGS 环境变量             -lstdc++ (动态)          ← 追加在最后
```

`build.rs` 的静态链接能生效，是因为 `cargo:rustc-link-lib=static=stdc++` 出现在 config/rustflags 的 `-lstdc++` **之前** — 链接器先解析静态版本的符号，后面的动态版本就被跳过了。

> **注意**：`.cargo/config.toml` 是**目录层级**作用域，不是 workspace 作用域。Cargo 从 CWD 向上遍历目录，逐层合并；深层覆盖浅层。但 `rustflags` 和 `build.rs` 指令是**不同通道** — 它们共存而非覆盖。

## Conanfiles

- `conanfile-hyperscan.txt` — Hyperscan 5.4.2
- `conanfile-vectorscan.txt` — Vectorscan 5.4.11

## publish

`cargo publish --dry-run  --features autobuild`
