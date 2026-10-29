//! Minimal git integration used to guard destructive `--fix` runs.

use std::path::Path;
use std::process::Command;

/// Returns whether a working `git` executable is available.
pub fn is_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Returns whether `dir` is inside a git work tree.
pub fn is_work_tree(dir: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|err| format!("failed to run git: {err}"))?;

    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
}

/// Returns whether the work tree containing `dir` has uncommitted changes.
///
/// Untracked files count as changes, matching the safety expectations of a
/// destructive command.
pub fn is_dirty(dir: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|err| format!("failed to run git status: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(!output.stdout.is_empty())
}
