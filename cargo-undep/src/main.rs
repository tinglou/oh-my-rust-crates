//! `cargo-undep` finds and removes unused `[workspace.dependencies]`.
//!
//! The tool scans every workspace member for dependency inheritance
//! (`workspace = true`) in normal, development, build, and target-specific
//! dependency tables. Without `--fix` it only reports the unused entries.

mod cleaner;
#[cfg(not(feature = "clap-cli"))]
mod cli;
#[cfg(feature = "clap-cli")]
mod cli_clap;
mod git;
mod workspace;

use std::fs;
use std::process::ExitCode;

#[cfg(not(feature = "clap-cli"))]
use cli::Args;
#[cfg(feature = "clap-cli")]
use cli_clap::Args;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;

    let start = match &args.manifest_path {
        Some(path) => path.clone(),
        None => std::env::current_dir()
            .map_err(|error| format!("failed to get current directory: {error}"))?,
    };

    let manifest_path = workspace::find_workspace_manifest(&start)?;
    let workspace = workspace::Workspace::load(&manifest_path)?;
    let unused = workspace.unused_dependencies()?;

    if unused.is_empty() {
        println!("No unused workspace dependencies found.");
        return Ok(());
    }

    println!("Unused workspace dependencies ({}):", unused.len());
    for dependency in &unused {
        println!(
            "  - {} ({}:{})",
            dependency.name,
            workspace.manifest_path.display(),
            dependency.start_line + 1
        );
    }

    if !args.fix {
        println!();
        println!("Run with --fix to remove them.");
        return Ok(());
    }

    ensure_git_safety(&workspace, args.allow_dirty)?;

    let cleaned = cleaner::remove_dependencies(&workspace.manifest_text, &unused);
    fs::write(&workspace.manifest_path, cleaned).map_err(|error| {
        format!(
            "failed to write {}: {error}",
            workspace.manifest_path.display()
        )
    })?;

    println!();
    println!(
        "Removed {} unused workspace dependencies from {}.",
        unused.len(),
        workspace.manifest_path.display()
    );

    Ok(())
}

/// Enforce the git safety policy for destructive `--fix` runs.
///
/// Unless `--allow-dirty` is supplied, the workspace must be in a git work tree
/// with no uncommitted changes. Missing git is treated as unsafe.
fn ensure_git_safety(workspace: &workspace::Workspace, allow_dirty: bool) -> Result<(), String> {
    if !git::is_available() {
        if allow_dirty {
            eprintln!(
                "warning: git was not found; skipping dirty check because --allow-dirty was supplied."
            );
            return Ok(());
        }
        return Err(
            "git was not found. Re-run with --allow-dirty to modify files without git safety checks."
                .to_string(),
        );
    }

    if !git::is_work_tree(&workspace.root_dir)? {
        if allow_dirty {
            eprintln!(
                "warning: the workspace is not inside a git repository; skipping dirty check because --allow-dirty was supplied."
            );
            return Ok(());
        }
        return Err(
            "the workspace is not inside a git repository. Re-run with --allow-dirty to modify files without git safety checks."
                .to_string(),
        );
    }

    if git::is_dirty(&workspace.root_dir)? {
        if allow_dirty {
            return Ok(());
        }
        return Err(
            "the git working tree has uncommitted changes. Commit or stash them, or re-run with --allow-dirty."
                .to_string(),
        );
    }

    Ok(())
}
