# conan2-rs

[:us: English](README.md) | [:cn: 中文](README-cn.md)

用于在 Cargo build script 中集成 [Conan 2](https://docs.conan.io/2/) 的 Rust 库。

## 用法

```rust
use conan2_rs::command::install::ConanInstall;
use conan2_rs::command::BuildPolicy;

// 1. 构建 conan install 命令
let command = ConanInstall::builder()?
    .profile("default")
    .build(BuildPolicy::Missing(None))
    .path("conanfile.txt")
    .into_command();

// 2. 运行 conan install，并解析 JSON 输出
let root = command.run()?;

// 3. 提取指定 component 的 C++ 构建信息
let cpp_info = root.get_cpp_info(&["hyperscan::hs", "hyperscan::hs_runtime"])?;

// 4. 输出 cargo link 指令
for libdir in &cpp_info.lib_dirs {
    println!("cargo:rustc-link-search={}", libdir);
}
for lib in &cpp_info.libs {
    println!("cargo:rustc-link-lib=static={}", lib);
}

// 5. 通过 bindgen 从 C 头文件生成 Rust FFI 绑定
let bindings = bindgen::builder()
    .header("wrapper.h")
    .clang_arg(format!("-I{}", cpp_info.include_dirs[0]))
    .allowlist_function("my_.*")
    .allowlist_type("my_.*")
    .generate()?;
bindings.write_to_file("src/bindings.rs")?;
```

## API

| 模块 | 说明 |
|------|------|
| `command::install` | `ConanInstall` builder → 运行 conan → 解析 JSON |
| `command::types` | `BuildPolicy`、`ConanVerbosity` 等类型 |
| `model` | `ConanRoot`、`Graph`、`Recipe`、`CppInfo` — Conan JSON schema 类型 |
| `model::visit` | `get_cpp_info()`、`get_recipe_by_name()` — 依赖图遍历 |

## 工作原理

1. **查找 conan** — 通过 `$CONAN` 环境变量或 `which conan`
2. **运行** `conan install <conanfile> --format json --out-file ...`
3. **解析** JSON 输出为 `ConanRoot`（匹配 [Conan graph JSON schema](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)）
4. **遍历**依赖图，提取每个 component 的 `CppInfo`
5. 调用方使用 `include_dirs` / `lib_dirs` / `libs` 配置 Rust 构建

## Conan JSON schema（部分）

```
ConanRoot
  └── graph: Graph
        ├── nodes: { id → Recipe }
        │     └── cpp_info: { component → CppInfo }
        │           ├── includedirs
        │           ├── libdirs
        │           └── libs
        └── root: { context → node_id }
```

## 参考

- [Conan Reference](https://docs.conan.io/2/reference.html)
- [Graph Info JSON Formatter](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)
