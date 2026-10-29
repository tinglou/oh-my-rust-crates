//! Command-line argument parsing.
//!
//! The tool intentionally uses a tiny hand-written parser instead of pulling in
//! a CLI framework: there are only three switches, and keeping the dependency
//! graph small is a design goal.

use std::ffi::OsString;
use std::path::PathBuf;

/// Parsed command-line options.
#[derive(Debug, Default)]
pub struct Args {
    /// Remove unused dependencies instead of only reporting them.
    pub fix: bool,
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
                "--fix" => args.fix = true,
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
                        return Err(format!("unknown argument: {other}\n\n{HELP}"));
                    }
                }
            }
        }

        Ok(args)
    }
}

/// Help text shown by `--help`.
pub const HELP: &str = "\
cargo-undep - find unused [workspace.dependencies]

USAGE:
    cargo undep [OPTIONS]

OPTIONS:
    --fix                  Remove unused workspace dependencies
    --allow-dirty          Allow --fix with uncommitted git changes or no git
    --manifest-path PATH   Path to the workspace root Cargo.toml
    -h, --help             Print help
    -V, --version          Print version

Without --fix the tool only prints the dependencies that would be removed.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(items: &[&str]) -> Result<Args, String> {
        Args::parse_from(items.iter().map(OsString::from))
    }

    #[test]
    fn parses_switches() {
        let args = parse(&["--fix", "--allow-dirty"]).unwrap();
        assert!(args.fix);
        assert!(args.allow_dirty);
        assert!(args.manifest_path.is_none());
    }

    #[test]
    fn strips_cargo_subcommand_name() {
        let args = parse(&["undep", "--fix"]).unwrap();
        assert!(args.fix);
    }

    #[test]
    fn parses_manifest_path() {
        let args = parse(&["--manifest-path", "some/Cargo.toml"]).unwrap();
        assert_eq!(args.manifest_path, Some(PathBuf::from("some/Cargo.toml")));

        let args = parse(&["--manifest-path=other/Cargo.toml"]).unwrap();
        assert_eq!(args.manifest_path, Some(PathBuf::from("other/Cargo.toml")));
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(parse(&["--nope"]).is_err());
    }
}
