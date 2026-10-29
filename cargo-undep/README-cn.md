# cargo-undep

[English](README.md)

查找并清理 Cargo 工作区 `[workspace.dependencies]` 中未被引用的依赖。

`cargo-undep` 会检查工作区所有成员 crate 中通过 `workspace = true` 继承的
依赖。如果某个 `[workspace.dependencies]` 依赖没有被任何成员引用，就会被视为
无用依赖。使用 `--fix` 时，该依赖会连同紧邻上方的注释一起从根 `Cargo.toml`
中删除。

## 安装

```text
cargo install cargo-undep
```

安装后可以通过 Cargo 子命令调用：

```text
cargo undep
```

## 本地开发

本地修改代码时，如果希望达到与 `cargo install cargo-undep` 相同的效果，
从源码目录安装当前工作区成员：

```text
cargo install --path cargo-undep
```

在工作区根目录执行时使用上面的包路径。修改代码后重新安装时加上 `--force`，
确保已安装的二进制被替换：

```text
cargo install --path cargo-undep --force
```

二进制会安装到 Cargo 的 bin 目录（Unix 为 `~/.cargo/bin`，Windows 为
`%USERPROFILE%\.cargo\bin`），该目录通常已经在 `PATH` 中。之后执行
`cargo undep` 就会使用本地构建的版本。

如果只是临时运行、不想安装：

```text
# 在工作区根目录
cargo run -p cargo-undep -- --help

# 在 cargo-undep 目录内
cargo run -- --help
```

注意：`cargo run` 是直接运行二进制文件；`cargo undep` 这种子命令形式只有在
二进制已安装到 `PATH` 后才可用。

## 用法

```text
cargo undep [--fix] [--allow-dirty] [--manifest-path PATH]
```

| 参数 | 说明 |
|------|------|
| `--fix` | 删除无用的工作区依赖 |
| `--allow-dirty` | 允许在 git 工作区有未提交修改或 git 不可用时执行 `--fix` |
| `--manifest-path PATH` | 指定工作区根 `Cargo.toml` |
| `-h`, `--help` | 打印帮助 |
| `-V`, `--version` | 打印版本 |

不加 `--fix` 时只输出需要清理的依赖列表，不修改任何文件。

### 只检测

```text
$ cargo undep
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Run with --fix to remove them.
```

### 清理

```text
$ cargo undep --fix
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Removed 2 unused workspace dependencies from /path/to/Cargo.toml.
```

## 引用判定规则

当工作区成员在以下任意表中以 `workspace = true` 声明同名依赖时，该
`[workspace.dependencies]` 条目视为被引用：

- `[dependencies]`
- `[dev-dependencies]` / `[dev_dependencies]`
- `[build-dependencies]` / `[build_dependencies]`
- `[target.'cfg(...)'.dependencies]`
- `[target.'cfg(...)'.dev-dependencies]`
- `[target.'cfg(...)'.build-dependencies]`

同时支持 inline table 和 `[dependencies.foo]` 子表写法：

```toml
[dependencies]
serde = { workspace = true }
```

```toml
[dependencies.serde]
workspace = true
```

Cargo 会自动把工作区内的 path 依赖纳入成员，本工具也会沿 path 依赖继续扫描。
支持 `[workspace].members` 中的 glob，以及 `[workspace].exclude`。

## 注释处理规则

删除依赖时，紧邻上方的连续注释也会被删除；如果中间存在空行，则不会删除：

```toml
[workspace.dependencies]
# 这个注释会随依赖一起删除
unused = "1.0"

# 下面两行注释都紧邻 also-unused，因此都会删除
# 上一行空行将它们与前一个依赖分隔开
also-unused = "2.0"

used = "1.0"
```

被注释掉的依赖会保留：

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
unused = "1.0"
```

执行 `--fix` 后：

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
```

当一行满足 `# name = value` 形式，其中 key 是合法依赖名，value 看起来像依赖
值（字符串、inline table 或裸数字）时，会被视为“被注释的依赖”。这类行同时
作为扫描边界：它以及它上方的注释都不会被删除。

## Git 安全策略

`--fix` 会修改文件，因此默认要求 git 工作区干净：

1. 必须存在 `git`。
2. 工作区必须位于 git 仓库内。
3. `git status --porcelain` 输出必须为空。

任一检查失败时，工具会拒绝修改文件并报错。如果 git 不可用或工作区有修改，
需要显式传入 `--allow-dirty`。未跟踪文件也会被视为 dirty。

## 实现说明

- 使用 [`toml_edit`](https://crates.io/crates/toml_edit) 解析 Cargo.toml，并利用
  key/value 的字节 span 定位依赖。该库是 `cargo-edit` 使用的格式保留 TOML 库。
- 使用 [`glob`](https://crates.io/crates/glob) 展开 `[workspace].members`。
- 实际删除基于原始文本行区间完成，保留换行符（包括 CRLF）、空行和其他注释。
- git 检查通过 `std::process::Command` 调用，不引入额外 git 库。

## 限制

- 只检查工作区成员 manifest 中的依赖声明。
- `[patch]`、`[replace]` 以及依赖表之外的引用不会被视为使用。
- 被注释依赖的识别规则偏保守：例如 `# note = "text"` 也会被保留。
- 只修改工作区根目录的 `Cargo.toml`。

## 开发

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 许可证

Apache-2.0
