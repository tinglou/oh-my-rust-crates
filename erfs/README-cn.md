[English](README.md) | 简体中文

# 嵌入式资源文件系统（ERFS）

ERFS 将目录树打包为紧凑的只读镜像，使应用可以直接从自身二进制中读取资源，无需文件系统，也无需拷贝数据。

本工作区包含纯 Rust 实现：

| Crate                          | 说明                                                                |
| ------------------------------ | ------------------------------------------------------------------- |
| [`erfs-rt`](erfs-rt)           | `no_std` 运行时，支持借用（`&[u8]`）与拥有（`Vec<u8>`）两种模式。    |
| [`erfs-gen`](erfs-gen)         | 镜像生成器：供 `build.rs` 使用的库，以及 `erfs-gen` 命令行工具。    |
| [`erfs-tool`](erfs-tool)       | 镜像查看工具：`ls`、`cat`、`tree` 三个子命令。                     |
| [`samples/erfs-demo`](../samples/erfs-demo) | 使用默认（`std` + `alloc`）特性的完整示例。          |
| [`samples/erfs-nostd-demo`](../samples/erfs-nostd-demo) | 使用 `default-features = false` 的示例。 |

[`erfs-c`](erfs-c) 中的 C 实现保留为二进制格式的参考；本仓库生成的镜像与
`erfs-c` 逐字节一致（见 [`erfs-c/layout.md`](erfs-c/layout.md)）。

## 工作原理

```
 资源目录/ ──erfs-gen──▶ erfs_le_app.img ──include_bytes!──▶ erfs-rt::Fs
```

1. `erfs-gen` 扫描资源目录并写出 ERFS 镜像。
2. 通过 `include_bytes!`（直接使用，或经生成的 Rust 绑定）把镜像嵌入二进制。
3. `erfs-rt` 对镜像校验一次，之后提供零拷贝访问。

同时支持小端与大端镜像；字节序由 magic 自动识别，因此同一镜像可在任意目标上使用。

## 命令行

```text
用法: erfs-gen [选项] <源目录> <id> <输出目录>

参数:
  <源目录>   要嵌入的目录（或单个文件）
  <id>       资源 id，需匹配 [a-z][a-z0-9_]*
  <输出目录> 已存在、用于接收生成文件的目录

选项:
  --image        生成二进制镜像（默认）
  --rust         生成 Rust 绑定源码（默认）
  --no-rust      不生成 Rust 绑定源码
  --le           生成小端镜像
  --be           生成大端镜像
  --both         同时生成小端和大端镜像
  -h, --help     打印帮助
  -V, --version  打印版本
```

生成的文件名与 `erfs-c` 保持一致：

* `erfs_le_<id>.img`、`erfs_be_<id>.img` —— 二进制镜像。
* `erfs_<id>.rs` —— Rust 绑定，通过 `include_bytes!` 提供 `IMAGE`、
  `IMAGE_LE`、`IMAGE_BE` 和 `ID` 常量。

### 镜像查看工具

`erfs-tool` 提供与 `erfs-c` 中 `erfstool` 相同的三个子命令：

```text
erfs-tool ls <dir> <img>             - 列出目录条目
erfs-tool cat [-c | -x] <path> <img> - 输出文件内容
erfs-tool tree [-d] <dir> <img>      - 打印目录树
```

`cat` 默认原样输出文件字节；`-c` 使用 C 风格转储（每行 16 字节，包含偏移量、
转义控制字符和十六进制表示）；`-x`（或 `--hex`）输出十六进制转储。`tree -d` 只显示目录，
忽略文件。

`ls` 使用无边框、带表头的表格对齐名称、类型和大小；`tree` 使用标准的
`├──` / `└──` / `│` 连接线展示层级，同时对齐各列。

## 在 `build.rs` 中使用库

```rust,no_run
// build.rs
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let options = erfs_gen::Options::new()
        .both_endians()
        .rust_binding(true);

    erfs_gen::generate("resources", "app", &out_dir, &options).unwrap();
    println!("cargo:rerun-if-changed=resources");
}
```

然后嵌入生成的绑定：

```rust,ignore
pub mod erfs_gensrc {
    include!(concat!(env!("OUT_DIR"), "/erfs_app.rs"));
}
```

## 运行时 API

```rust,ignore
use erfs_rt::{Fs, Visit};

const IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));

let fs = Fs::parse(IMAGE)?;

// 零拷贝读取文件内容
let data: &[u8] = fs.read("/dir/file.txt")?;

// 打开并查看条目
let entry = fs.open("/dir")?;
for child in entry.children()? {
    println!("{} ({} 字节)", child.name_str()?, child.size());
}

// 深度优先遍历
fs.walk(|visit| {
    if let Visit::File(entry) = visit {
        println!("{}", entry.name_str()?);
    }
    Ok(())
})?;
```

`erfs_rt::fs!` 宏把嵌入与解析两步合而为一：

```rust,ignore
let fs = erfs_rt::fs!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));
```

### 拥有模式

当镜像在运行时加载而非编译期嵌入时，`OwnedFs` 会将其存入 `Vec<u8>` 并只校验一次；
它提供相同的查找 API，并通过 `OwnedFs::as_fs` 与 `Fs` 共享实现：

```rust,no_run
use erfs_rt::OwnedFs;

let fs = OwnedFs::from_file("resources.img")?;
let data: &[u8] = fs.read("/dir/file.txt")?;
```

`OwnedFs::from_bytes` 接收 `Vec<u8>`，`from_slice` 复制切片，`into_bytes`
返回拥有的镜像。

`erfs-rt` 始终为 `#![no_std]`。借用模式 `Fs` 不分配内存；拥有模式需要 `alloc`
特性，`from_file` 另外需要 `std`（两者默认开启）。裸机目标请使用
`default-features = false`。

## 示例与测试

```sh
cargo test --workspace
cargo run -p erfs-gen -- --both resources myres /tmp/out
cargo run --manifest-path samples/erfs-demo/Cargo.toml
cargo run --manifest-path samples/erfs-nostd-demo/Cargo.toml
```

`erfs-rt` 与 `erfs-gen` 的测试套件包含黄金测试：当存在 `erfs-c` 参考镜像时，
会把生成的镜像与之逐字节比较。

## 待办

* 符号链接/硬链接支持

## 许可证

Apache-2.0
