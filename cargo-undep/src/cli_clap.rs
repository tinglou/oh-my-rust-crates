//! Optional clap-based command-line parser.
//!
//! This module is only compiled with the `clap-cli` feature. The hand-written
//! parser in `cli.rs` remains the default while the binary-size cost is
//! evaluated.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

/// Parsed command-line options.
#[derive(Debug, Default, Parser)]
#[command(
    name = "cargo-undep",
    bin_name = "cargo undep",
    version,
    about = "Find unused [workspace.dependencies]",
    after_help = "Without --fix the tool only prints the dependencies that would be removed."
)]
pub struct Args {
    /// Remove unused workspace dependencies
    #[arg(long)]
    pub fix: bool,
    /// Allow --fix with uncommitted git changes or no git
    #[arg(long)]
    pub allow_dirty: bool,
    /// Path to the workspace root Cargo.toml
    #[arg(long, value_name = "PATH")]
    pub manifest_path: Option<PathBuf>,
}

impl Args {
    /// Parse the process arguments.
    ///
    /// Cargo passes the subcommand name as `argv[1]` when invoked as
    /// `cargo undep`, so it is stripped before clap sees the arguments.
    pub fn parse() -> Result<Self, String> {
        let mut argv: Vec<OsString> = std::env::args_os().collect();
        if argv
            .get(1)
            .is_some_and(|arg| arg.to_string_lossy() == "undep")
        {
            argv.remove(1);
        }

        match Self::try_parse_from(argv) {
            Ok(args) => Ok(args),
            Err(error) if error.use_stderr() => Err(error.to_string()),
            Err(error) => {
                print!("{error}");
                std::process::exit(0);
            }
        }
    }
}
