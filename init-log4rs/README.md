# init-log4rs

A utility crate that simplifies log4rs initialization for Rust applications. It automatically searches for a log4rs configuration file in multiple directories, and creates one if not found.

## Features

- **Auto-discovery**: Searches for `log4rs.yaml` configuration file in multiple directories (in order):
  - `RUST_LOG_DIR` environment variable path (if set)
  - Executable directory (in `logs` subdirectory)
  - Current working directory (in `logs` subdirectory)
  - Parent directory (in `logs` subdirectory)
  - User's home directory (in `logs` subdirectory)
  - System temporary directory (in `logs` subdirectory)

- **Auto-creation**: If no configuration file is found, automatically creates a new one with sensible defaults including:
  - Console output (stdout and stderr)
  - Rolling file appender with size-based rotation (10 MB limit)
  - Fixed window rollover pattern (keeps up to 20 archived logs)
  - Configurable log format: `{date} {level} {module}:{line} - {message}`

- **Hot-reload ready**: Configuration file refreshes every 60 seconds for dynamic log level changes

- **Tracing support** (optional): When the `tracing` feature is enabled, `tracing` spans and events are automatically bridged to `log4rs` and written to the same log files.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
init-log4rs = "1.0"
log = "0.4"
```

Or with tracing support:

```toml
[dependencies]
init-log4rs = { version = "1.0", features = ["tracing"] }
tracing = "0.1"
```

## Usage

### Basic (log crate)

```rust
use log::{info, debug, error};

fn main() -> anyhow::Result<()> {
    // Initialize log4rs with configuration file name and log file base name
    init_log4rs::init_log4rs("log4rs.yaml", "app_log")?;

    // Now you can use the log macros
    info!("Application started");
    debug!("Debug information");
    error!("Error occurred");

    Ok(())
}
```

### Tracing support (optional)

Enable the `tracing` feature to bridge `tracing` events into the same log4rs pipeline:

```rust,no_run
use tracing::{info, debug, error};

fn main() -> anyhow::Result<()> {
    // Same init call — tracing subscriber is set up automatically
    init_log4rs::init_log4rs("log4rs.yaml", "app_log")?;

    // Use tracing macros — output goes to log4rs files
    info!("This goes to the log file");
    debug!("Debug level trace");

    Ok(())
}
```

Control the tracing log level via the `RUST_LOG` environment variable:

```sh
RUST_LOG=debug ./your_app
```

### Parameters

- `log_cfg_yaml`: The name of the log4rs configuration file (e.g., `"log4rs.yaml"`)
- `log_stem`: The base name for the log file without extension (e.g., `"app_log"` will create `app_log.log`)

## Generated Configuration

When no configuration file is found, the crate creates a YAML configuration with the following structure:

```yaml
# Scan this file for changes every 30 seconds
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
