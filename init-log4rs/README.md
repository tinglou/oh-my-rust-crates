# init-log4rs

A utility crate that simplifies log4rs initialization for Rust applications. It automatically searches for a log4rs configuration file in multiple directories, and creates one if not found.

## Features

- **Auto-discovery**: Searches for `log4rs.yaml` configuration file in multiple directories:
  - Executable directory
  - Current working directory (in `logs` subdirectory)
  - Parent directory (in `logs` subdirectory)
  - User's home directory (in `logs` subdirectory)
  - System temporary directory

- **Auto-creation**: If no configuration file is found, automatically creates a new one with sensible defaults including:
  - Console output (stdout and stderr)
  - Rolling file appender with size-based rotation (10 MB limit)
  - Fixed window rollover pattern (keeps up to 20 archived logs)
  - Configurable log format: `{date} {level} {module}:{line} - {message}`

- **Hot-reload ready**: Configuration file can be set to refresh every 30 seconds for dynamic log level changes

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
init-log4rs = "1.0"
log = "0.4"
```

## Usage

```rust
use init_log4rs;
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

### Parameters

- `log_cfg_yaml`: The name of the log4rs configuration file (e.g., `"log4rs.yaml"`)
- `log_stem`: The base name for the log file without extension (e.g., `"app_log"` will create `app_log.log`)

## Generated Configuration

When no configuration file is found, the crate creates a YAML configuration with the following structure:

```yaml
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
