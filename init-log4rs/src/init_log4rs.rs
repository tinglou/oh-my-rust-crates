use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use log4rs;
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

/// Try to load configuration file from following directories or create new.
///
/// * executable dir
/// * current_dir
/// * homedir
/// * temp dir
pub fn init_log4rs(log_cfg_yaml: &str, log_stem: &str) -> Result<(), InitLog4rsError> {
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

    // Search log4rs.yaml first
    for path in &log_path {
        let mut log_cfg_file = path.clone();
        log_cfg_file.push(log_cfg_yaml);
        match log4rs::init_file(log_cfg_file.clone(), Default::default()) {
            Ok(_) => {
                eprintln!("Log dir {}", path.display());
                return Ok(());
            }
            Err(_err) => {
                // eprintln!("Failed to load {}: {}", path.display(), _err);
            }
        }
    }

    // Create if not found
    for path in log_path.iter() {
        match logger_yaml_file_create(path, log_cfg_yaml, log_stem) {
            Ok(_) => {
                eprintln!("Log dir {}", path.display());
                return Ok(());
            }
            Err(_err) => {
                // eprintln!("Failed to load {}: {}", path.display(), _err);
            }
        }
    }
    return Err(InitLog4rsError::Other(
        "Failed to initialize log4rs.".to_string(),
    ));
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
