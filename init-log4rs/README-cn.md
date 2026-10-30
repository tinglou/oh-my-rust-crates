# init-log4rs

[:us: English](README.md) | [:cn: 中文](README-cn.md)

一个简化 Rust 应用 log4rs 初始化的工具 crate。它会在多个目录中自动查找 log4rs 配置文件，如果找不到则自动创建。

## 特性

- **自动发现**：按以下顺序在多个目录中查找 `log4rs.yaml`：
  - `RUST_LOG_DIR` 环境变量指定的路径（如果设置）
  - 可执行文件所在目录（的 `logs` 子目录）
  - 当前工作目录（的 `logs` 子目录）
  - 父目录（的 `logs` 子目录）
  - 用户主目录（的 `logs` 子目录）
  - 系统临时目录（的 `logs` 子目录）

- **自动创建**：如果找不到配置文件，会自动创建包含合理默认值的配置：
  - 控制台输出（stdout 和 stderr）
  - 基于大小轮转的滚动文件 appender（10 MB 上限）
  - 固定窗口轮转模式（最多保留 20 个归档日志）
  - 可配置日志格式：`{date} {level} {module}:{line} - {message}`

- **支持热加载**：配置文件每 60 秒刷新一次，可动态调整日志级别

- **tracing 支持**（可选）：启用 `tracing` 特性后，`tracing` spans 和 events 会自动桥接到 `log4rs` 并写入同一日志文件。

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
init-log4rs = "1.0"
log = "0.4"
```

或启用 tracing 支持：

```toml
[dependencies]
init-log4rs = { version = "1.0", features = ["tracing"] }
tracing = "0.1"
```

## 用法

### 基础用法（log crate）

```rust
use log::{info, debug, error};

fn main() -> anyhow::Result<()> {
    // 传入 log4rs 配置文件名和日志文件基础名
    init_log4rs::init_log4rs("log4rs.yaml", "app_log")?;

    // 现在可以使用 log 宏
    info!("Application started");
    debug!("Debug information");
    error!("Error occurred");

    Ok(())
}
```

### tracing 支持（可选）

启用 `tracing` 特性后，可将 `tracing` 事件桥接到同一套 log4rs 管线：

```rust,no_run
use tracing::{info, debug, error};

fn main() -> anyhow::Result<()> {
    // 初始化调用相同，tracing subscriber 会自动设置
    init_log4rs::init_log4rs("log4rs.yaml", "app_log")?;

    // 使用 tracing 宏，输出会写入 log4rs 文件
    info!("This goes to the log file");
    debug!("Debug level trace");

    Ok(())
}
```

可通过 `RUST_LOG` 环境变量控制 tracing 日志级别：

```sh
RUST_LOG=debug ./your_app
```

### 参数

- `log_cfg_yaml`：log4rs 配置文件名（例如 `"log4rs.yaml"`）
- `log_stem`：日志文件基础名，不含扩展名（例如 `"app_log"` 会创建 `app_log.log`）

## 生成的配置

找不到配置文件时，crate 会创建如下结构的 YAML 配置：

```yaml
# 每 30 秒检查一次文件变更
refresh_rate: 60 seconds

appenders:
  stdout:
    kind: console

  stderr:
    kind: console
    target: stderr

  rolling_file:
    kind: rolling_file
    path: <path>/app_log.log
    append: true
    encoder:
      pattern: '{d} {l} {M}:{L} - {m}{n}'
    policy:
      kind: compound
      trigger:
        kind: size
        limit: 10 mb
      roller:
        kind: fixed_window
        pattern: <path>/app_log.log.{}
        base: 1
        count: 20

root:
  level: info
  appenders:
    - rolling_file
```
