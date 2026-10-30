//! Command-line argument parsing.
//!
//! The tool intentionally uses a tiny hand-written parser instead of pulling in
//! a CLI framework: there are only two commands and three switches, and keeping
//! the dependency graph small is a design goal.

use std::ffi::OsString;
use std::path::PathBuf;

/// Selected operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Report or remove unused workspace dependencies.
    Clean,
    /// Report or hoist compatible member dependency sources.
    Hoist,
}

/// Parsed command-line options.
#[derive(Debug, Default)]
pub struct Args {
    /// Command selected on the command line. `None` reports both operations.
    pub mode: Option<Mode>,
    /// Apply changes instead of only reporting them.
    pub fix: bool,
    /// After a successful --fix with changes, run `cargo build --all-targets` to verify.
    pub build: bool,
    /// Allow `--fix` to run with uncommitted git changes or without git.
    pub allow_dirty: bool,
    /// Optional path to a workspace root `Cargo.toml`.
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
        Self::parse_from(std::env::args_os().skip(1))
    }

    fn parse_from<I>(iter: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = Self::default();
        let mut iter = iter.into_iter().peekable();

        if iter
            .peek()
            .is_some_and(|arg| arg.to_string_lossy() == "undep")
        {
            iter.next();
        }

        while let Some(raw) = iter.next() {
            let arg = raw.to_string_lossy();

            match arg.as_ref() {
                "clean" => set_mode(&mut args, Mode::Clean)?,
                "hoist" => set_mode(&mut args, Mode::Hoist)?,
                "--fix" => args.fix = true,
                "--build" => args.build = true,
                "--allow-dirty" => args.allow_dirty = true,
                "--manifest-path" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--manifest-path requires a path argument".to_string())?;
                    args.manifest_path = Some(PathBuf::from(value));
                }
                "-h" | "--help" => {
                    print!("{HELP}");
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("cargo-undep {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                other => {
                    if let Some(value) = other.strip_prefix("--manifest-path=") {
                        if value.is_empty() {
                            return Err("--manifest-path requires a path argument".to_string());
                        }
                        args.manifest_path = Some(PathBuf::from(value));
                    } else {
                        return Err(format!("unknown argument or command: {other}\n\n{HELP}"));
                    }
                }
            }
        }

        if args.fix && args.mode.is_none() {
            return Err(format!(
                "--fix requires the `clean` or `hoist` command\n\n{HELP}"
            ));
        }

        Ok(args)
    }
}

fn set_mode(args: &mut Args, mode: Mode) -> Result<(), String> {
    if args.mode.is_some() {
        return Err("only one of `clean` or `hoist` may be specified".to_string());
    }
    args.mode = Some(mode);
    Ok(())
}

/// Help text shown by `--help`.
pub const HELP: &str = "\
cargo-undep - find unused workspace dependencies and hoist member dependencies

USAGE:
    cargo undep [COMMAND] [OPTIONS]

COMMANDS:
    clean                  Report or remove unused [workspace.dependencies]
    hoist                  Report or move compatible member dependency sources
                           into [workspace.dependencies]

OPTIONS:
    --fix                  Apply changes for the selected command
    --build                After --fix with changes, run `cargo build --all-targets` to verify
    --allow-dirty          Allow --fix with uncommitted git changes or no git
    --manifest-path PATH   Path to the workspace root Cargo.toml
    -h, --help             Print help
    -V, --version          Print version

Without COMMAND, the tool reports both unused workspace dependencies and
hoistable member dependencies. --fix requires `clean` or `hoist`.
When run from a member directory, `hoist` only processes crates under the
current directory.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(items: &[&str]) -> Result<Args, String> {
        Args::parse_from(items.iter().map(OsString::from))
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
    fn rejects_fix_without_command() {
        assert!(parse(&["--fix"]).is_err());
    }

    #[test]
    fn strips_cargo_subcommand_name() {
        let args = parse(&["undep", "clean", "--fix"]).unwrap();
        assert_eq!(args.mode, Some(Mode::Clean));
        assert!(args.fix);
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
