//! Command-line argument parsing with clap.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Selected operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Mode {
    /// Report or remove unused workspace dependencies
    Clean,
    /// Report or move compatible member dependency sources
    Hoist,
    /// Report or sort dependency tables, merging duplicate workspace dependencies
    Sort,
}

/// Parsed command-line options.
#[derive(Debug, Default, Parser)]
#[command(
    name = "cargo-undep",
    bin_name = "cargo undep",
    version,
    about = "Find unused workspace dependencies, hoist member dependencies, or sort dependency tables",
    after_help = "Without COMMAND, the tool reports unused and hoistable dependencies. --fix requires `clean`, `hoist`, or `sort`. When run from a member directory, `hoist` only processes crates under the current directory."
)]
pub struct Args {
    /// Command to run
    #[command(subcommand)]
    pub mode: Option<Mode>,
    /// Apply changes for the selected command
    #[arg(long, global = true)]
    pub fix: bool,
    /// After --fix with changes, run `cargo build --all-targets` to verify
    #[arg(long, global = true)]
    pub build: bool,
    /// Allow --fix with uncommitted git changes or no git
    #[arg(long, global = true)]
    pub allow_dirty: bool,
    /// Path to the workspace root Cargo.toml
    #[arg(long, global = true, value_name = "PATH")]
    pub manifest_path: Option<PathBuf>,
}

impl Args {
    /// Parse the process arguments.
    ///
    /// `--help` and `--version` print their output and terminate the process.
    ///
    /// When invoked as `cargo undep`, Cargo passes the subcommand name as the
    /// first argument (`argv[1] == "undep"`), so it is stripped here.
    pub fn parse() -> Result<Self, String> {
        let mut argv: Vec<OsString> = std::env::args_os().collect();
        if argv
            .get(1)
            .is_some_and(|arg| arg.to_string_lossy() == "undep")
        {
            argv.remove(1);
        }

        Self::parse_from(argv)
    }

    fn parse_from<I, T>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        match Self::try_parse_from(args) {
            Ok(args) => {
                if args.fix && args.mode.is_none() {
                    Err("--fix requires the `clean`, `hoist`, or `sort` command".to_string())
                } else {
                    Ok(args)
                }
            }
            Err(error) if error.use_stderr() => Err(error.to_string()),
            Err(error) => {
                print!("{error}");
                std::process::exit(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(items: &[&str]) -> Result<Args, String> {
        let mut args = vec!["cargo-undep"];
        args.extend_from_slice(items);
        Args::parse_from(args)
    }

    #[test]
    fn parses_switches() {
        let args = parse(&["clean", "--fix", "--allow-dirty"]).unwrap();
        assert_eq!(args.mode, Some(Mode::Clean));
        assert!(args.fix);
        assert!(args.allow_dirty);
        assert!(args.manifest_path.is_none());
    }

    #[test]
    fn parses_no_command() {
        let args = parse(&[]).unwrap();
        assert_eq!(args.mode, None);
        assert!(!args.fix);
    }

    #[test]
    fn parses_hoist() {
        let args = parse(&["hoist", "--fix"]).unwrap();
        assert_eq!(args.mode, Some(Mode::Hoist));
        assert!(args.fix);
    }

    #[test]
    fn parses_sort() {
        let args = parse(&["sort", "--fix"]).unwrap();
        assert_eq!(args.mode, Some(Mode::Sort));
        assert!(args.fix);
    }

    #[test]
    fn rejects_fix_without_command() {
        assert!(parse(&["--fix"]).is_err());
    }

    #[test]
    fn parses_manifest_path() {
        let args = parse(&["clean", "--manifest-path", "some/Cargo.toml"]).unwrap();
        assert_eq!(args.manifest_path, Some(PathBuf::from("some/Cargo.toml")));

        let args = parse(&["--manifest-path=other/Cargo.toml", "hoist"]).unwrap();
        assert_eq!(args.manifest_path, Some(PathBuf::from("other/Cargo.toml")));
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(parse(&["--nope"]).is_err());
    }

    #[test]
    fn rejects_multiple_commands() {
        assert!(parse(&["clean", "hoist"]).is_err());
    }
}
