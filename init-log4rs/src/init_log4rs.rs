use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InitLog4rsError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Initialize the `log4rs` logging framework by searching a predefined list of
/// directories for an existing configuration file, or by generating a new one
/// if none is found.
///
/// This is the simplest entry point. It automatically builds a prioritized
/// directory list and delegates to [`init_log4rs_dirs`].
///
/// # Search order (first match wins)
///
/// 1. The directory pointed to by the `RUST_LOG_DIR` environment variable (if set).
/// 2. `<executable_dir>/logs` — the directory containing the running binary.
/// 3. `<current_working_dir>/logs` — the process's current working directory.
/// 4. `<parent_of_current_working_dir>/logs` — one level above the CWD.
/// 5. `<home_dir>/logs` — the user's home directory (platform-specific).
/// 6. `<temp_dir>/logs` — the system temporary directory.
///
/// For each directory in the list, the function first tries to load an existing
/// `log_cfg_yaml` file (e.g. `log4rs.yaml`). If loading succeeds, logging is
/// initialized immediately and the function returns. If the file does not exist
/// anywhere, a default configuration is generated and written into the *first
/// writable* directory from the list.
///
/// On success, the path of the chosen log directory is printed to `stderr`.
///
/// # Parameters
///
/// * `log_cfg_yaml` — The filename of the log4rs YAML configuration to search
///   for / create (e.g. `"log4rs.yaml"`).
/// * `log_stem` — The base name (without extension) for log files that will be
///   produced by the auto-generated rolling-file appender (e.g. `"app_log"`
///   produces `app_log.log` and `app_log.log.{}`).
///
/// # Errors
///
/// Returns [`InitLog4rsError::Other`] when no directory in the search list
/// could be read from or written to, or when the generated YAML cannot be
/// parsed by `log4rs`.
///
/// # Example
///
/// ```rust,no_run
/// use init_log4rs::init_log4rs;
///
/// fn main() {
///     init_log4rs("log4rs.yaml", "my_app").expect("failed to init logging");
///     log::info!("Hello, world!");
/// }
/// ```
pub fn init_log4rs(log_cfg_yaml: &str, log_stem: &str) -> Result<(), InitLog4rsError> {
    let dirs = generate_dir_list();
    init_log4rs_dirs(log_cfg_yaml, log_stem, &dirs)
}

/// Initialize the `log4rs` logging framework just like [`init_log4rs`], but
/// with an extra user-specified directory that is tried **first** before all
/// other locations.
///
/// This is useful when the caller wants to control the primary log directory
/// explicitly (e.g. a configurable `--log-dir` CLI argument), while still
/// falling back to the standard search paths if the preferred directory is
/// not suitable.
///
/// # Search order (first match wins)
///
/// 1. `dir` — the caller-provided directory (tried as-is, without appending
///    `"logs"`).
/// 2. The standard [`init_log4rs`] directory list:
///    `RUST_LOG_DIR`, `<exe>/logs`, `<cwd>/logs`, `<cwd_parent>/logs`,
///    `<home>/logs`, `<temp>/logs`.
///
/// # Parameters
///
/// * `log_cfg_yaml` — The filename of the log4rs YAML configuration to search
///   for / create (e.g. `"log4rs.yaml"`).
/// * `log_stem` — The base name for log files produced by the rolling-file
///   appender (e.g. `"my_app"` → `my_app.log`, `my_app.log.{}`).
/// * `dir` — The directory to check first. It is used exactly as given;
///   `"logs"` is **not** appended automatically.
///
/// # Errors
///
/// Returns [`InitLog4rsError::Other`] when no directory (including the
/// user-provided one) could be read from or written to.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use init_log4rs::init_log4rs_with_dir;
///
/// fn main() {
///     let custom_dir = PathBuf::from("/var/log/my_service");
///     init_log4rs_with_dir("log4rs.yaml", "service", custom_dir)
///         .expect("failed to init logging");
/// }
/// ```
pub fn init_log4rs_with_dir(
    log_cfg_yaml: &str,
    log_stem: &str,
    dir: PathBuf,
) -> Result<(), InitLog4rsError> {
    let mut dirs = generate_dir_list();
    dirs.insert(0, dir);
    init_log4rs_dirs(log_cfg_yaml, log_stem, &dirs)
}

/// Initialize the `log4rs` logging framework using an explicit, caller-provided
/// list of directories.
///
/// This is the lowest-level entry point. It gives the caller full control over
/// which directories are searched and in what order. Both [`init_log4rs`] and
/// [`init_log4rs_with_dir`] are thin wrappers around this function.
///
/// # How it works
///
/// The function performs **two passes** over the `dirs` slice:
///
/// **Pass 1 — Look for an existing configuration.**  
/// For each directory, it constructs the path `<dir>/<log_cfg_yaml>` and
/// attempts to load it with `log4rs::init_file`. The first directory that
/// contains a valid, parseable configuration wins — logging is initialized
/// immediately and the function returns.
///
/// **Pass 2 — Create a new default configuration.**  
/// If no existing configuration was found in *any* directory, the function
/// iterates again and tries to **create** the directory (via `create_dir_all`)
/// and write a default `log4rs` YAML file there. The generated configuration
/// includes:
///
/// * A `console` appender writing to **stdout**.
/// * A `console` appender writing to **stderr**.
/// * A `rolling_file` appender with 10 MiB size-based rotation, keeping up to
///   20 archived files. Log files are named `<log_stem>.log` (active) and
///   `<log_stem>.log.{}` (rotated).
/// * A root logger set to `info` level, feeding the `rolling_file` appender.
///
/// On success, the path of the chosen log directory is printed to `stderr`.
///
/// # Parameters
///
/// * `log_cfg_yaml` — The filename of the log4rs YAML configuration file to
///   search for or create (e.g. `"log4rs.yaml"`).
/// * `log_stem` — The base name for log files produced by the rolling-file
///   appender when a default configuration is generated (e.g. `"my_app"` →
///   `my_app.log`, `my_app.log.{}`).
/// * `dirs` — A slice of directories to search, in priority order (index 0 is
///   checked first). Each directory is used as-is; no `"logs"` subdirectory is
///   appended automatically.
///
/// # Errors
///
/// Returns [`InitLog4rsError::Other`] when:
///
/// * No existing configuration was found **and** no directory in `dirs` could
///   have a default configuration created in it (e.g. permission denied on all
///   candidates), or
/// * The generated YAML configuration cannot be parsed by `log4rs`.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use init_log4rs::init_log4rs_dirs;
///
/// fn main() {
///     let dirs: &[PathBuf] = &[
///         PathBuf::from("/etc/my_app"),
///         PathBuf::from("/tmp/my_app_logs"),
///     ];
///     init_log4rs_dirs("log4rs.yaml", "my_app", dirs)
///         .expect("failed to init logging");
/// }
/// ```
pub fn init_log4rs_dirs(
    log_cfg_yaml: &str,
    log_stem: &str,
    dirs: &[PathBuf],
) -> Result<(), InitLog4rsError> {
    // Search log4rs.yaml first
    for path in dirs.iter() {
        let mut log_cfg_file = path.clone();
        log_cfg_file.push(log_cfg_yaml);
        match log4rs::init_file(log_cfg_file.clone(), Default::default()) {
            Ok(_) => {
                eprintln!("Log dir {}", path.display());
                init_tracing_subscriber();
                return Ok(());
            }
            Err(_err) => {
                // eprintln!("Failed to load {}: {}", path.display(), _err);
            }
        }
    }

    // Create if not found
    for path in dirs.iter() {
        match logger_yaml_file_create(path, log_cfg_yaml, log_stem) {
            Ok(_) => {
                eprintln!("Log dir {}", path.display());
                init_tracing_subscriber();
                return Ok(());
            }
            Err(_err) => {
                // eprintln!("Failed to load {}: {}", path.display(), _err);
            }
        }
    }
    Err(InitLog4rsError::Other(
        "Failed to initialize log4rs.".to_string(),
    ))
}

fn generate_dir_list() -> Vec<PathBuf> {
    let mut log_path: Vec<PathBuf> = Vec::new();

    // First, check RUST_LOG_DIR environment variable
    if let Ok(log_dir) = env::var("RUST_LOG_DIR") {
        log_path.push(PathBuf::from(log_dir));
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(p) = exe.parent()
    {
        let mut exe_log_dir = p.to_path_buf();
        exe_log_dir.push("logs");
        log_path.push(exe_log_dir);
    }
    if let Ok(mut pwd) = env::current_dir() {
        pwd.push("logs");
        log_path.push(pwd);
    }
    if let Ok(pwd) = env::current_dir()
        && let Some(parent) = pwd.parent()
    {
        let mut path = parent.to_path_buf();
        path.push("logs");
        log_path.push(path);
    }
    if let Some(mut path) = dirs::home_dir() {
        path.push("logs");
        log_path.push(path);
    }
    let mut temp_log_dir = env::temp_dir();
    temp_log_dir.push("logs");
    log_path.push(temp_log_dir);
    log_path
}

/// Initialize tracing subscriber when the tracing feature is enabled.
/// Does nothing when the feature is not enabled.
fn init_tracing_subscriber() {
    #[cfg(feature = "tracing")]
    {
        use tracing_subscriber::EnvFilter;
        use tracing_subscriber::prelude::*;
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = tracing_subscriber::registry().with(filter).try_init();
    }
    #[cfg(not(feature = "tracing"))]
    {
        // no-op
    }
}

/// create log4rs configuration file
fn logger_yaml_file_create(
    path: &PathBuf,
    log_cfg: &str,
    log_stem: &str,
) -> Result<(), InitLog4rsError> {
    fs::create_dir_all(path)?;

    let mut log_store = path.clone();
    log_store.push(log_stem);
    log_store.set_extension("log");
    let log_store_file = log_store.to_str().ok_or(InitLog4rsError::Other(
        "log store path construct fail".to_string(),
    ))?;

    let mut log_tar = path.clone();
    log_tar.push(log_stem);
    log_tar.set_extension("log.{}");

    let log_tar_file = log_tar.to_str().ok_or(InitLog4rsError::Other(
        "log tar path construct fail".to_string(),
    ))?;

    let mut log_cfg_file = PathBuf::from(path);
    log_cfg_file.push(log_cfg);

    let config_str = format!(
        "
# Scan this file for changes every 30 seconds
refresh_rate: 60 seconds

appenders:
    # stdout
    stdout:
        kind: console

    # stderr
    stderr:
        kind: console
        target: stderr

    # rolling_file
    rolling_file:
        kind: rolling_file
        path: {}
        append: true
        encoder:
            # https://docs.rs/log4rs/1.0.0/log4rs/encode/pattern/index.html#formatters
            pattern: '{{d}} {{l}} {{M}}:{{L}} - {{m}}{{n}}' # Date Level [Module]:[line] - msg \\n
        policy:
            kind: compound
            trigger:
                kind: size
                limit: 10 mb # file size limit 102400 Byte
            roller:
                kind: fixed_window
                pattern: {}
                base: 1
                count: 20

# loggers:
#     map_app::my_mod: # custom logger
#         level: debug
#         appenders:
#             - stderr
#         additive: false

# Set the default logging level to 'info' and attach the 'rolling_file' appender to the root
root:
    level: info
    appenders:
        - rolling_file
",
        log_store_file, log_tar_file
    );

    // This function will create a file if it does not exist, and will truncate it if it does.
    let mut output = File::create(log_cfg_file.clone())?;
    write!(output, "{}", config_str)?;
    drop(output);
    log4rs::init_file(log_cfg_file, Default::default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log4rs() -> anyhow::Result<()> {
        init_log4rs("log4rs.yaml", "app_log")?;
        Ok(())
    }
}
