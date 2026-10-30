# backerror

[:us: English](README.md) | [:cn: 中文](README-cn.md)

`backerror` 是一个 Rust 错误处理库，会在错误创建时自动捕获位置信息与调用栈，并与 [thiserror](https://github.com/dtolnay/thiserror) 无缝集成，通过追踪错误来源位置来提升调试体验。

错误抛出时，错误链和调用栈会被捕获，并以易于阅读的格式展示。

## 特性

- **自动位置追踪**：自动捕获错误来源的源码位置（文件、行、列）
- **调用栈捕获**：可选捕获完整调用栈，便于调试
- **无缝集成**：兼容现有基于 `thiserror` 的错误类型
- **零成本抽象**：仅在启用时产生运行时开销，可在 release 构建中关闭
- **透明包装**：在保留原始错误类型的同时附加位置元数据
- **条件编译**：可在 release 构建中禁用以减少开销

## 安装

执行以下命令将 `backerror` 加入项目：

```bash
cargo add backerror
```

或在 `Cargo.toml` 中添加：

```toml
[dependencies]
backerror = "0.1"
thiserror = "2.0"
```

## 用法

### 示例代码

```rust
use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
#[error(transparent)]
pub struct MyError1(#[from] std::io::Error);

#[backerror]
#[derive(Debug, Error)]
pub enum MyError2 {
    #[error("By MyError2: {0}")]
    My1(#[from] MyError1),
}

#[backerror]
#[derive(Debug, Error)]
pub enum MyError3 {
    #[error("By MyError3: {0}")]
    My2(#[from] MyError2),
}

fn throw_error1() -> Result<(), MyError1> {
    std::fs::File::open("blurb.txt")?;
    Ok(())
}

fn throw_error2() -> Result<(), MyError2> {
    Ok(throw_error1()?)
}

fn throw_error3() -> Result<(), MyError3> {
    Ok(throw_error2()?)
}

#[test]
fn test_display() {
    if let Err(err) = throw_error3() {
        println!("{}", err);
    }
}

#[test]
fn test_debug() {
    if let Err(e) = throw_error3() {
        println!("{:?}", e);
    }
}
```

### 输出示例

#### Display 输出（`to_string`）

```text
By MyError3: By MyError2: The system cannot find the file specified. (os error 2); Caused by example::MyError2 (backerror\tests\example.rs:32:8); Caused by example::MyError1 (backerror\tests\example.rs:29:8); Caused by std::io::error::Error(backerror\tests\example.rs:24:5);
```

#### Debug 输出

Debug 输出会包含错误链、捕获位置以及完整调用栈，例如：

```text
My2(My1(MyError1(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }
Caused by: example::MyError2: By MyError2: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:32:8)
        at example::impl$10::from (.\tests\example.rs:16)
        at example::throw_error3 (.\tests\example.rs:32)
        ...
Caused by: example::MyError1: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:29:8)
        ...
Caused by: std::io::error::Error: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:24:5)
        ...
)
```

## Rust features

crate 提供以下可选特性：

- `backtrace`：仅在设置 `RUST_BACKTRACE` 环境变量时捕获调用栈
- `force_backtrace`：无论环境变量如何都强制捕获调用栈（默认启用）
- `release_off`：在 release 构建中禁用 backerror 转换（默认启用）

自定义特性：

```toml
[dependencies]
backerror = { version = "...", default-features = false, features = ["force_backtrace"] }
```

## 工作原理

`backerror` 的工作方式：

1. 提供 `#[backerror]` 属性宏来转换错误类型
2. 将 `#[from] T` 转换为 `#[from] LocatedError<T>`
3. 使用 Rust 的 `#[track_caller]` 捕获错误转换发生的位置
4. 在创建错误时可选捕获完整调用栈

错误发生时，它会被包装进 `LocatedError<T>`，在保留原始错误的同时附加位置元数据。
