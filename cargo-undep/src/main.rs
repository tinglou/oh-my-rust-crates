//! `cargo-undep` finds and removes unused `[workspace.dependencies]`.
//!
//! The tool scans every workspace member for dependency inheritance
//! (`workspace = true`) in normal, development, build, and target-specific
//! dependency tables. With no command it reports both unused workspace
//! dependencies and dependencies that can be hoisted. The `clean` command
//! removes unused entries; the `hoist` command moves compatible member
//! dependency sources into `[workspace.dependencies]`; the `sort` command
//! groups dependency tables by path and crate category.

mod cleaner;
mod cli;
mod git;
mod hoist;
mod sort;
mod workspace;

use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

use cli::{Args, Mode};

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

    let current_dir = std::env::current_dir()
        .map_err(|error| format!("failed to get current directory: {error}"))?;
    let start = match &args.manifest_path {
        Some(path) => path.clone(),
        None => current_dir.clone(),
    };

    let manifest_path = workspace::find_workspace_manifest(&start)?;
    let workspace = workspace::Workspace::load(&manifest_path)?;

    match args.mode {
        None => report_all(&workspace),
        Some(Mode::Clean) => run_clean(
            &workspace,
            args.fix,
            args.build,
            args.allow_dirty,
            &manifest_path,
        ),
        Some(Mode::Hoist) => {
            let scope = hoist_scope(&workspace, &current_dir);
            run_hoist(
                &workspace,
                args.fix,
                args.build,
                args.allow_dirty,
                &manifest_path,
                scope,
            )
        }
        Some(Mode::Sort) => run_sort(
            &workspace,
            args.fix,
            args.build,
            args.allow_dirty,
            &manifest_path,
        ),
    }
}

/// Limit `hoist` to the current directory when it is not the workspace root.
fn hoist_scope<'a>(workspace: &workspace::Workspace, current_dir: &'a Path) -> Option<&'a Path> {
    (current_dir != workspace.root_dir && current_dir.starts_with(&workspace.root_dir))
        .then_some(current_dir)
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

fn run_clean(
    workspace: &workspace::Workspace,
    fix: bool,
    build: bool,
    allow_dirty: bool,
    manifest_path: &Path,
) -> Result<(), String> {
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

    if build {
        run_cargo_build(manifest_path)?;
    }

    Ok(())
}

fn run_hoist(
    workspace: &workspace::Workspace,
    fix: bool,
    build: bool,
    allow_dirty: bool,
    manifest_path: &Path,
    scope: Option<&Path>,
) -> Result<(), String> {
    let plan = hoist::plan_scoped(workspace, scope)?;
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

    if build && !plan.changes.is_empty() {
        run_cargo_build(manifest_path)?;
    }

    Ok(())
}

fn run_sort(
    workspace: &workspace::Workspace,
    fix: bool,
    build: bool,
    allow_dirty: bool,
    manifest_path: &Path,
) -> Result<(), String> {
    let plan = sort::plan(workspace)?;

    if plan.changes.is_empty() {
        println!("No dependency tables need sorting.");
        return Ok(());
    }

    println!("Cargo.toml files to update ({}):", plan.changes.len());
    for change in &plan.changes {
        println!("  - {}", change.path.display());
    }

    if !fix {
        println!();
        println!("Run `cargo undep sort --fix` to apply these changes.");
        return Ok(());
    }

    ensure_manifest_git_safety(workspace, allow_dirty)?;

    for change in &plan.changes {
        fs::write(&change.path, &change.contents)
            .map_err(|error| format!("failed to write {}: {error}", change.path.display()))?;
    }

    println!();
    println!("Updated {} Cargo.toml files.", plan.changes.len());

    if build {
        run_cargo_build(manifest_path)?;
    }

    Ok(())
}

fn run_cargo_build(manifest_path: &Path) -> Result<(), String> {
    let workspace_root = manifest_path.parent().ok_or("manifest has no parent")?;
    println!();
    println!("Running `cargo build --all-targets` to verify...");
    let status = Command::new("cargo")
        .args(["build", "--all-targets"])
        .current_dir(workspace_root)
        .status()
        .map_err(|e| format!("failed to run cargo build: {e}"))?;
    if !status.success() {
        return Err("cargo build failed".to_string());
    }
    println!("Build successful.");
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
    if !plan.hoisted.is_empty() {
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

    if !plan.enforced.is_empty() {
        if !plan.hoisted.is_empty() {
            println!();
        }
        println!("Enforced inheritance ({}):", plan.enforced.len());
        for dependency in &plan.enforced {
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
            let category = match dependency.conflict_type {
                hoist::ConflictType::VersionConflict => "[version conflict] ",
                hoist::ConflictType::DefaultFeaturesConflict => "[default-features conflict] ",
                hoist::ConflictType::AlreadyInWorkspace => "",
                hoist::ConflictType::Other => "",
            };
            println!("  - {}{}: {}", category, dependency.name, dependency.reason);
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

/// Enforce git safety for `sort --fix`.
///
/// Sorting only rewrites Cargo.toml files, so only uncommitted changes to a
/// Cargo.toml block the operation. Other dirty files are unrelated.
fn ensure_manifest_git_safety(
    workspace: &workspace::Workspace,
    allow_dirty: bool,
) -> Result<(), String> {
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

    let dirty_manifests = git::modified_manifests(&workspace.root_dir)?;
    if dirty_manifests.is_empty() {
        return Ok(());
    }

    if allow_dirty {
        return Ok(());
    }

    Err(format!(
        "these Cargo.toml files have uncommitted changes: {}. Commit or stash them, or re-run with --allow-dirty.",
        dirty_manifests.join(", ")
    ))
}
