# cargo-undep

[English](README.md)

查找并清理 Cargo 工作区 `[workspace.dependencies]` 中未被引用的依赖，并将成员 crate 中可安全共享的依赖来源提升到 `[workspace.dependencies]`。

## 主要功能

### `clean` — 查找并删除无用的工作区依赖

扫描所有工作区成员的 `workspace = true` 继承关系。未被任何成员引用的 `[workspace.dependencies]` 条目视为无用。使用 `--fix` 时，连同紧邻上方的注释一起从根 `Cargo.toml` 删除。

### `hoist` — 将成员依赖来源提升到工作区依赖

查找可以安全地在 `[workspace.dependencies]` 中只声明一次、并由成员 crate 继承的依赖。**只把来源（`version`、`path`、`git`）移到工作区层**；成员级属性（`features`、`optional`、`default-features`）保留在成员 manifest。使用 `--fix` 时，也会对已在 `[workspace.dependencies]` 中的依赖强制把成员改为 `workspace = true`（强制继承）。

### `sort` — 按 path 和 crate 分类整理依赖表

当 `[workspace.dependencies]` 或成员 crate 的 `[dependencies]` 依赖数不少于 10 个时，对这些表进行分类排序。path 依赖排在第一组；其他依赖按 crate 的第一个 category 分组；没有 category 的依赖放入最后的 `misc` 组。如果依赖行尾注释指定了 `# category = ...`，则该类别优先级最高。每组前增加注释行，组内按名称排序。依赖正上方连续的多行注释以及依赖行尾注释会随该依赖一起移动。

## 安装

```text
cargo install cargo-undep
```

安装后可通过 Cargo 子命令调用：

```text
cargo undep
```

## 快速开始

```bash
# 同时报告无用依赖和可提升依赖
cargo undep

# 只检测无用工作区依赖
cargo undep clean

# 删除无用工作区依赖
cargo undep clean --fix

# 只检测可提升的成员依赖
cargo undep hoist

# 提升成员依赖（同时强制继承已声明的依赖）
cargo undep hoist --fix

# 只检测需要排序的依赖表
cargo undep sort

# 对依赖表分类排序
cargo undep sort --fix
```

## 命令行参数

```bash
cargo undep [clean|hoist|sort] [--fix] [--build] [--allow-dirty] [--manifest-path PATH]
```

| 参数 | 说明 |
|------|------|
| `clean` | 检测或删除无用的 `[workspace.dependencies]` |
| `hoist` | 检测或将成员 crate 中可共享的依赖来源提升到 `[workspace.dependencies]` |
| `sort` | 检测或按 path 和 category 对 `[workspace.dependencies]` 与成员 `[dependencies]` 排序 |
| `--fix` | 对选定的命令应用修改 |
| `--build` | 在 `--fix` 有文件修改后，运行 `cargo build --all-targets` 验证 |
| `--allow-dirty` | 允许在 git 工作区有未提交修改或 git 不可用时执行 `--fix` |
| `--manifest-path PATH` | 指定工作区根 `Cargo.toml` |
| `-h`, `--help` | 打印帮助 |
| `-V`, `--version` | 打印版本 |

不指定命令时，工具同时报告无用的工作区依赖和可提升的成员依赖。`--fix` 必须配合 `clean`、`hoist` 或 `sort` 使用。在非工作区根目录运行 `hoist` 时，只处理当前目录下的 crate。

## 用法示例

### 同时报告无用依赖和可提升依赖

以下输出保留工具实际的英文输出。

```text
$ cargo undep
No unused workspace dependencies found.
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - [default-features conflict] macaddr-ouidb-support: mixed default-features
  - [default-features conflict] serde: mixed default-features
  - [version conflict] thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep clean --fix` to remove unused dependencies or `cargo undep hoist --fix` to hoist dependencies.
```

### 只检测无用依赖

```text
$ cargo undep clean
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Run `cargo undep clean --fix` to remove them.
```

### 清理无用依赖

```text
$ cargo undep clean --fix
Unused workspace dependencies (2):
  - snmp-parser (/path/to/Cargo.toml:42)
  - unused-helper (/path/to/Cargo.toml:45)

Removed 2 unused workspace dependencies from /path/to/Cargo.toml.
```

### 只检测可提升依赖

```text
$ cargo undep hoist
Hoistable workspace dependencies (24):
  - anyhow (version = "1.0.102"; 3 declarations)
      /path/to/conan2-rs/Cargo.toml
      /path/to/hypervectorscan-rs/hypervectorscan-sys/Cargo.toml
      /path/to/init-log4rs/Cargo.toml
  ...

Skipped dependencies (3):
  - [default-features conflict] macaddr-ouidb-support: mixed default-features
  - [default-features conflict] serde: mixed default-features
  - [version conflict] thiserror: incompatible version requirements `2.0.18` and `1.0.26`

Run `cargo undep hoist --fix` to apply these changes.
```

### 提升依赖

```text
$ cargo undep hoist --fix
Hoistable workspace dependencies (24):
  ...

Updated 15 files and hoisted 24 workspace dependencies.
```

### 对依赖表排序

```text
$ cargo undep sort
Cargo.toml files to update (2):
  - /path/to/Cargo.toml
  - /path/to/member/Cargo.toml

Run `cargo undep sort --fix` to apply these changes.

$ cargo undep sort --fix
Cargo.toml files to update (2):
  - /path/to/Cargo.toml
  - /path/to/member/Cargo.toml

Updated 2 Cargo.toml files.
```

### 排序前后对比

```toml
# 修改前
[dependencies]
zeta = { package = "serde_json", version = "1" }
# anyhow 上方的注释
anyhow = "1" # 行尾注释
local = { path = "../local" }
clap = "4"
...

# 修改后
[dependencies]
# --- path ---
local = { path = "../local" }
# --- command-line-interface ---
clap = "4"
# --- encoding ---
zeta = { package = "serde_json", version = "1" }
# --- rust-patterns ---
# anyhow 上方的注释
anyhow = "1" # 行尾注释
```

### 提升前后对比

```toml
# 修改前（成员）
serde = { version = "1.0", features = ["derive"], optional = true }

# 修改后（只移动来源，保留成员属性）
serde = { workspace = true, features = ["derive"], optional = true }
```

```toml
# 工作区根 Cargo.toml 获得共享来源
[workspace.dependencies]
serde = "1.0"
```

## 策略

### 引用判定规则

当工作区成员在以下任意表中以 `workspace = true` 声明同名依赖时，该 `[workspace.dependencies]` 条目视为被引用：

- `[dependencies]`
- `[dev-dependencies]` / `[dev_dependencies]`
- `[build-dependencies]` / `[build_dependencies]`
- `[target.'cfg(...)'.dependencies]`
- `[target.'cfg(...)'.dev-dependencies]`
- `[target.'cfg(...)'.build-dependencies]`

同时支持 inline table 和 `[dependencies.foo]` 子表写法。工作区内的 path 依赖会自动纳入成员继续扫描。支持 `[workspace].members` glob 和 `[workspace].exclude`。

### 依赖排序规则

`sort` 对 `[workspace.dependencies]` 和成员 `[dependencies]` 中的每个依赖分类：

1. 依赖行尾注释中指定的 category 优先级最高，例如 `snmp2 = { version = "0.5.3" } # category = network-programming`。
2. 带 `path` 来源的依赖排在第一组 `path`。
3. 其他依赖使用 `cargo metadata` 提供的第一个 category，并只保留 `::` 之前的部分。例如 `development-tools::profiling` 归为 `development-tools`。
4. 没有 category 的依赖放入最后一组 `misc`。

组顺序为 `path`、按字母排序的 category、最后的 `misc`。组内按依赖名称排序。依赖数少于 10 个的表不做修改。

每组前会增加一行注释：

```toml
[dependencies]
# --- path ---
local-helper = { path = "../local-helper" }
# --- encoding ---
serde = "1"
# --- misc ---
unknown-crate = "1"
```

排序时会保留依赖原有的声明文本。依赖正上方连续的多行注释以及同一行末尾的注释都会随依赖一起移动。

### 注释处理规则

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

被注释掉的依赖会保留，并作为扫描边界：

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
unused = "1.0"
```

执行 `--fix` 后，被注释的依赖保留：

```toml
[workspace.dependencies]
# snmp-parser = "0.11.0"
```

当一行满足 `# name = value` 形式，其中 key 是合法依赖名，value 看起来像依赖值（字符串、inline table 或裸数字）时，会被视为“被注释的依赖”。这类行同时作为扫描边界：它以及它上方的注释都不会被删除。

### 版本兼容判定

可兼容的简单版本要求会合并，并提高最低版本（caret 语义）：

- `1.0` + `1.2` → `1.2`
- `0.2` + `0.2.5` → `0.2.5`
- `1.0` + `2.0` → **跳过**（主版本不兼容）
- `0.2` + `0.3` → **跳过**（0.x 次版本不兼容）
- `default-features` 不一致 → **跳过**（Cargo 无法在不修改成员属性的情况下同时表达两种值）

被跳过的依赖会列出原因，但不会阻止其他依赖被提升。

### Git 安全策略

`clean --fix` 和 `hoist --fix` 会修改文件，因此默认要求 git 工作区干净：

1. 必须存在 `git`。
2. 工作区必须位于 git 仓库内。
3. `git status --porcelain` 输出必须为空。

`sort --fix` 只修改 `Cargo.toml`，因此只有当某个 `Cargo.toml` 有未提交修改时才会拒绝执行；其他文件的未提交修改不影响排序。

任一检查失败时，工具会拒绝修改文件并报错。如果 git 不可用或相关文件有修改，需显式传入 `--allow-dirty`。未跟踪文件也会被视为 dirty。

## 许可证

Apache-2.0
