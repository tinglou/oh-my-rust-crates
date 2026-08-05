# Oh My Rust Crates

[:us: English](README.md) | [:cn: 中文](README-cn.md)

Rust 公共工具库集合。

## Crates

本 workspace 包含以下 crate：

### 1. backerror

**Java 风格 Rust 错误回溯**

`backerror` 为 Rust 错误增强自动捕获位置信息和调用栈的能力。它无缝集成 [thiserror](https://github.com/dtolnay/thiserror)，通过追踪错误在代码中的来源位置来提升调试能力。

详见 [backerror-rs/backerror/README-cn.md](backerror-rs/backerror/README-cn.md)。（暂未提供中文文档）

### 2. macaddr-ouidb

**高性能 MAC 地址 OUI 查询**

`macaddr-ouidb` 提供基于预编译数据库的 MAC 地址到制造商（OUI - 组织唯一标识符）的快速查询。

详见 [macaddr-ouidb/README-cn.md](macaddr-ouidb/README-cn.md)。（暂未提供中文文档）

### 3. conan2-rs

**Cargo build.rs 集成 Conan 2**

`conan2-rs` 为 [Conan 2](https://docs.conan.io/2/) C/C++ 包管理器提供 Rust 接口，使得在 FFI 项目中可以直接从 `build.rs` 管理原生 C/C++ 依赖。

详见 [conan2-rs/README-cn.md](conan2-rs/README-cn.md)。（暂未提供中文文档）

### 4. hypervectorscan

**Hyperscan / Vectorscan 安全 Rust 封装**

`hypervectorscan` 为 [Hypersan](https://github.com/intel/hyperscan) / [Vectorscan](https://github.com/VectorCamp/vectorscan) 高性能多模式正则匹配引擎提供安全、高层 Rust API。支持 block、streaming 和 vectored 扫描模式，可通过 conan、vcpkg、system 或 vendored 方式自动检测 C 库。

详见 [hypervectorscan-rs/hypervectorscan/README-cn.md](hypervectorscan-rs/hypervectorscan/README-cn.md)。

### 5. init-log4rs

**零配置 log4rs 初始化**

`init-log4rs` 自动搜索并初始化 log4rs 配置，无需手动配置即可为 Rust 应用启用日志功能。

详见 [init-log4rs/README-cn.md](init-log4rs/README-cn.md)。（暂未提供中文文档）

## License

本 workspace 中所有 crate 均采用 Apache License, Version 2.0。详见 [LICENSE](LICENSE)。

## Contributing

欢迎提交 Issues 和 Pull Requests！
