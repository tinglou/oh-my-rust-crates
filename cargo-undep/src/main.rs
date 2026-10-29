//! `cargo-undep` finds and removes unused `[workspace.dependencies]`.
//!
//! The tool scans every workspace member for dependency inheritance
//! (`workspace = true`) in normal, development, build, and target-specific
//! dependency tables. With no command it reports both unused workspace
//! dependencies and dependencies that can be hoisted. The `clean` command
//! removes unused entries; the `hoist` command moves compatible member
//! dependency sources into `[workspace.dependencies]`.

mod cleaner;
#[cfg(not(feature = "clap-cli"))]
mod cli;
#[cfg(feature = "clap-cli")]
mod cli_clap;
mod git;
mod hoist;
mod workspace;

use std::fs;
use std::process::ExitCode;

#[cfg(not(feature = "clap-cli"))]
use cli::{Args, Mode};
#[cfg(feature = "clap-cli")]
use cli_clap::{Args, Mode};

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

    match args.mode {
        None => report_all(&workspace),
        Some(Mode::Clean) => run_clean(&workspace, args.fix, args.allow_dirty),
        Some(Mode::Hoist) => run_hoist(&workspace, args.fix, args.allow_dirty),
    }
}

fn report_all(workspace: &workspace::Workspace) -> Result<(), String> {
    let unused = workspace.unused_dependencies()?;
    print_unused(workspace, &unused);

    let plan = hoist::plan(workspace)?;
    print_hoist_plan(&plan);

    if !unused.is_empty() || !plan.hoisted.is_empty() || !plan.skipped.is_empty() {
        println!();
        println!(
            "Run `cargo undep clean --fix` to remove unused dependencies or `cargo undep hoist --fix` to hoist dependencies."
        );
    }

    Ok(())
}

fn run_clean(workspace: &workspace::Workspace, fix: bool, allow_dirty: bool) -> Result<(), String> {
    let unused = workspace.unused_dependencies()?;
    print_unused(workspace, &unused);

    if unused.is_empty() {
        return Ok(());
    }

    if !fix {
        println!();
        println!("Run `cargo undep clean --fix` to remove them.");
        return Ok(());
    }

    ensure_git_safety(workspace, allow_dirty)?;

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

fn run_hoist(workspace: &workspace::Workspace, fix: bool, allow_dirty: bool) -> Result<(), String> {
    let plan = hoist::plan(workspace)?;
    print_hoist_plan(&plan);

    if !fix {
        if !plan.changes.is_empty() {
            println!();
            println!("Run `cargo undep hoist --fix` to apply these changes.");
        }
        return Ok(());
    }

    if plan.is_empty() {
        return Ok(());
    }

    ensure_git_safety(workspace, allow_dirty)?;

    for change in &plan.changes {
        fs::write(&change.path, &change.contents)
            .map_err(|error| format!("failed to write {}: {error}", change.path.display()))?;
    }

    println!();
    println!(
        "Updated {} files and hoisted {} workspace dependencies.",
        plan.changes.len(),
        plan.hoisted.len()
    );

    Ok(())
}

fn print_unused(workspace: &workspace::Workspace, unused: &[&workspace::WorkspaceDependency]) {
    if unused.is_empty() {
        println!("No unused workspace dependencies found.");
        return;
    }

    println!("Unused workspace dependencies ({}):", unused.len());
    for dependency in unused {
        println!(
            "  - {} ({}:{})",
            dependency.name,
            workspace.manifest_path.display(),
            dependency.start_line + 1
        );
    }
}

fn print_hoist_plan(plan: &hoist::HoistPlan) {
    if plan.hoisted.is_empty() {
        println!("No hoistable workspace dependencies found.");
    } else {
        println!("Hoistable workspace dependencies ({}):", plan.hoisted.len());
        for dependency in &plan.hoisted {
            println!(
                "  - {} ({}; {} declarations)",
                dependency.name, dependency.source, dependency.declarations
            );
            for manifest in &dependency.manifests {
                println!("      {}", manifest.display());
            }
        }
    }

    if !plan.skipped.is_empty() {
        println!();
        println!("Skipped dependencies ({}):", plan.skipped.len());
        for dependency in &plan.skipped {
            println!("  - {}: {}", dependency.name, dependency.reason);
        }
    }
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
