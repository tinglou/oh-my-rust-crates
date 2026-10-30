# Oh My Rust Crates

[:us: English](README.md) | [:cn: 中文](README-cn.md)

一组面向诊断、网络、原生构建集成与工作区工具场景的 Rust crate 集合。

## 主要功能

| Crate | 提供的核心能力 |
|-------|----------------|
| [backerror](backerror-rs/backerror/README-cn.md) | 基于 `thiserror` 的 Java 风格错误位置与调用栈 |
| [macaddr-ouidb](mac-oui/macaddr-ouidb/README-cn.md) | 内嵌数据的高速 MAC OUI 查询，支持地区信息、虚拟网卡识别与 `no_std` |
| [conan2-rs](conan2-rs/README-cn.md) | 在 Cargo `build.rs` 与 FFI 项目中集成 Conan 2 |
| [hypervectorscan](hypervectorscan-rs/hypervectorscan/README-cn.md) | Hyperscan / Vectorscan 多模式匹配的安全高层 Rust 封装 |
| [init-log4rs](init-log4rs/README-cn.md) | 零配置发现、创建并初始化 `log4rs` |
| [cargo-undep](cargo-undep/README-cn.md) | 查找无用工作区依赖，并安全提升成员依赖来源 |

## Crates

### backerror

**Java 风格 Rust 错误回溯**

`backerror` 在错误创建时捕获源码位置，并可选捕获调用栈。它与 [thiserror](https://github.com/dtolnay/thiserror) 集成，在保留原始错误类型的同时，让错误传播过程更容易调试。

详见 [backerror-rs/backerror/README-cn.md](backerror-rs/backerror/README-cn.md)。

### macaddr-ouidb

**高性能 MAC 地址 OUI 查询**

`macaddr-ouidb` 基于预编译数据库提供 MAC 地址到制造商的快速查询，支持地区信息、虚拟网卡识别、多种 MAC 地址格式与 `no_std`。

详见 [mac-oui/macaddr-ouidb/README-cn.md](mac-oui/macaddr-ouidb/README-cn.md)。

### conan2-rs

**Cargo build.rs 集成 Conan 2**

`conan2-rs` 为 [Conan 2](https://docs.conan.io/2/) 包管理器提供 Rust 接口，使 FFI crate 可以直接在 `build.rs` 中安装原生 C/C++ 依赖。

详见 [conan2-rs/README-cn.md](conan2-rs/README-cn.md)。

### hypervectorscan

**Hyperscan / Vectorscan 安全 Rust 封装**

`hypervectorscan` 为 [Hyperscan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan) 提供安全、高层的 Rust API。支持 block、streaming 和 vectored 扫描，并可通过 conan、vcpkg、系统库或 vendored 构建自动检测 C 库。

详见 [hypervectorscan-rs/hypervectorscan/README-cn.md](hypervectorscan-rs/hypervectorscan/README-cn.md)。

### init-log4rs

**零配置 log4rs 初始化**

`init-log4rs` 会自动发现已有的 `log4rs` 配置，或在缺失时创建包含控制台输出与滚动文件日志的默认配置。启用 `tracing` 特性后，spans 和 events 也会写入同一套日志管线。

详见 [init-log4rs/README-cn.md](init-log4rs/README-cn.md)。

### cargo-undep

**工作区依赖清理与提升**

`cargo-undep` 可以报告并删除无用的 `[workspace.dependencies]`，也能把可兼容的成员依赖来源提升到工作区，同时保留 `features`、`optional`、`default-features` 等成员级属性。

详见 [cargo-undep/README-cn.md](cargo-undep/README-cn.md)。

## 开发

构建、测试与发布说明见 [DEVELOPMENT.md](DEVELOPMENT.md)。欢迎提交 Issue 与 Pull Request。

## 许可证

本 workspace 中所有 crate 均采用 Apache License, Version 2.0。详见 [LICENSE](LICENSE)。
