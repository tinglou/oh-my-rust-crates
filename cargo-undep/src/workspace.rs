//! Workspace discovery and unused workspace dependency detection.
//!
//! `toml_edit` is used for parsing because it preserves the original document
//! and exposes byte spans for keys and values. The spans are converted into
//! line ranges, which [`crate::cleaner`] later removes from the raw text.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use toml_edit::{Document, Item, TableLike, Value};

/// Dependency table names that may inherit a workspace dependency.
pub(crate) const DEPENDENCY_SECTIONS: &[&str] = &[
    "dependencies",
    "dev-dependencies",
    "dev_dependencies",
    "build-dependencies",
    "build_dependencies",
];

/// A dependency declared in the root `[workspace.dependencies]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDependency {
    /// The dependency key, including aliases created via `package = "..."`.
    pub name: String,
    /// Zero-based first line of the declaration in the original manifest.
    pub start_line: usize,
    /// Zero-based last line of the declaration in the original manifest.
    pub end_line: usize,
}

/// A loaded Cargo workspace.
pub struct Workspace {
    /// Absolute or user-supplied workspace root directory.
    pub root_dir: PathBuf,
    /// Path to the workspace root `Cargo.toml`.
    pub manifest_path: PathBuf,
    /// Manifest text after any duplicate `[workspace.dependencies]` entries have
    /// been merged. This is the text that sorting operates on.
    pub manifest_text: String,
    /// Manifest text exactly as read from disk, before duplicate merging.
    pub original_manifest_text: String,
    /// Number of duplicate workspace dependency declarations that were merged
    /// while loading. Always zero for [`Workspace::load`].
    pub merged_duplicates: usize,
    /// All entries in `[workspace.dependencies]`.
    pub dependencies: Vec<WorkspaceDependency>,
    /// Manifests of every crate that belongs to the workspace.
    pub member_manifests: Vec<PathBuf>,
}

impl Workspace {
    /// Load a workspace from its root `Cargo.toml`.
    ///
    /// Duplicate keys are rejected by the TOML parser. Use
    /// [`Workspace::load_for_sort`] when duplicate `[workspace.dependencies]`
    /// entries should be merged instead.
    pub fn load(manifest_path: &Path) -> Result<Self, String> {
        Self::load_inner(manifest_path, false)
    }

    /// Load a workspace for the `sort` command.
    ///
    /// Duplicate entries in the root `[workspace.dependencies]` table are merged
    /// before the manifest is parsed, because `toml_edit` rejects duplicate
    /// keys. The original text is kept in
    /// [`Workspace::original_manifest_text`] so callers can detect and write the
    /// merged result.
    pub fn load_for_sort(manifest_path: &Path) -> Result<Self, String> {
        Self::load_inner(manifest_path, true)
    }

    fn load_inner(manifest_path: &Path, merge_duplicates: bool) -> Result<Self, String> {
        let manifest_path = manifest_path.to_path_buf();
        let root_dir = manifest_path
            .parent()
            .ok_or_else(|| {
                format!(
                    "manifest path has no parent directory: {}",
                    manifest_path.display()
                )
            })?
            .to_path_buf();

        let original_manifest_text = fs::read_to_string(&manifest_path)
            .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;

        let (manifest_text, merged_duplicates) = if merge_duplicates {
            let merged = crate::manifest::merge_workspace_dependency_duplicates(
                &original_manifest_text,
                &manifest_path,
            )?;
            (merged.text, merged.merged)
        } else {
            (original_manifest_text.clone(), 0)
        };

        let document: Document<String> = manifest_text
            .parse()
            .map_err(|err| format!("failed to parse {}: {err}", manifest_path.display()))?;

        let workspace = document
            .get("workspace")
            .and_then(Item::as_table_like)
            .ok_or_else(|| {
                format!(
                    "{} does not contain a [workspace] table",
                    manifest_path.display()
                )
            })?;

        let dependencies = if let Some(dependency_table) =
            workspace.get("dependencies").and_then(Item::as_table_like)
        {
            dependency_table
                .iter()
                .map(|(name, item)| {
                    let (start_line, end_line) =
                        dependency_line_range(dependency_table, name, item, &manifest_text);
                    WorkspaceDependency {
                        name: name.to_string(),
                        start_line,
                        end_line,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let excludes = collect_excludes(&document, &root_dir);
        let member_manifests = discover_members(
            &document,
            &manifest_path,
            &root_dir,
            &excludes,
            &manifest_text,
        )?;

        Ok(Self {
            root_dir,
            manifest_path,
            manifest_text,
            original_manifest_text,
            merged_duplicates,
            dependencies,
            member_manifests,
        })
    }

    /// Members to process when the command is scoped to `scope`.
    ///
    /// If `scope` is the workspace root, every member is returned. If `scope`
    /// is inside a member crate, that crate is returned. Otherwise, members
    /// whose directories are under `scope` are returned.
    pub fn members_in_scope(&self, scope: &Path) -> Vec<PathBuf> {
        let scope = normalize_path(scope);
        if scope == normalize_path(&self.root_dir) {
            return self.member_manifests.clone();
        }

        let mut containing: Option<(&PathBuf, usize)> = None;
        for manifest in &self.member_manifests {
            let Some(directory) = manifest.parent() else {
                continue;
            };
            let directory = normalize_path(directory);
            if scope.starts_with(&directory) {
                let depth = directory.components().count();
                let is_deeper = match containing {
                    None => true,
                    Some((_, best_depth)) => depth > best_depth,
                };
                if is_deeper {
                    containing = Some((manifest, depth));
                }
            }
        }

        if let Some((manifest, _)) = containing {
            return vec![manifest.clone()];
        }

        self.member_manifests
            .iter()
            .filter(|manifest| {
                manifest
                    .parent()
                    .map(|directory| normalize_path(directory).starts_with(&scope))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Returns workspace dependencies that no member crate inherits.
    pub fn unused_dependencies(&self) -> Result<Vec<&WorkspaceDependency>, String> {
        let mut used = HashSet::new();

        for manifest in &self.member_manifests {
            let text = fs::read_to_string(manifest)
                .map_err(|err| format!("failed to read {}: {err}", manifest.display()))?;
            let document: Document<String> = text
                .parse()
                .map_err(|err| format!("failed to parse {}: {err}", manifest.display()))?;

            visit_dependency_entries(&document, |name, item, _| {
                if is_workspace_inherited(item) {
                    used.insert(name.to_string());
                }
            });
        }

        Ok(self
            .dependencies
            .iter()
            .filter(|dependency| !used.contains(&dependency.name))
            .collect())
    }
}

/// Walk up from `start` until a `Cargo.toml` containing `[workspace]` is found.
pub fn find_workspace_manifest(start: &Path) -> Result<PathBuf, String> {
    let mut directory = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = directory.join("Cargo.toml");
        if candidate.is_file()
            && let Ok(text) = fs::read_to_string(&candidate)
            && (text
                .parse::<Document<String>>()
                .map(|document| document.get("workspace").is_some())
                .unwrap_or_else(|_| contains_workspace_table(&text)))
        {
            return Ok(candidate);
        }

        if !directory.pop() {
            return Err(format!(
                "could not find a Cargo workspace starting from {}",
                start.display()
            ));
        }
    }
}

/// Detect a `[workspace]` table without parsing the whole manifest.
///
/// This is a fallback for manifests that the TOML parser rejects, for example
/// because `[workspace.dependencies]` contains duplicate keys. `sort` repairs
/// those manifests, but it still has to find the workspace first.
fn contains_workspace_table(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.split('#').next().unwrap_or(line).trim();
        line == "[workspace]" || line.starts_with("[workspace.") || line.starts_with("[workspace ")
    })
}

/// Convert a key/value span into inclusive zero-based line numbers.
fn dependency_line_range(
    table: &dyn TableLike,
    name: &str,
    item: &Item,
    text: &str,
) -> (usize, usize) {
    let key_span = table.get_key_value(name).and_then(|(key, _)| key.span());
    let item_span = item.span();

    let start = key_span
        .as_ref()
        .map(|span| span.start)
        .or_else(|| item_span.as_ref().map(|span| span.start))
        .unwrap_or(0);

    let mut end = item_span
        .as_ref()
        .map(|span| span.end)
        .or_else(|| key_span.as_ref().map(|span| span.end))
        .unwrap_or(start);

    // A `[workspace.dependencies.foo]` sub-table has no value span. In that
    // case remove the whole sub-table rather than only its header line.
    if matches!(item, Item::Table(_)) && item_span.is_none() {
        end = subtable_end_offset(text, start);
    }

    let start_line = offset_to_line(text, start);
    let end_line = end_offset_to_line(text, end).max(start_line);
    (start_line, end_line)
}

/// Find the byte offset at which the next top-level table header begins.
fn subtable_end_offset(text: &str, start: usize) -> usize {
    let after_first_line = text[start..]
        .find('\n')
        .map(|index| start + index + 1)
        .unwrap_or(text.len());

    let mut offset = after_first_line;
    for line in text[after_first_line..].split_inclusive('\n') {
        if line.starts_with('[') {
            return offset;
        }
        offset += line.len();
    }
    text.len()
}

fn offset_to_line(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}

fn end_offset_to_line(text: &str, end: usize) -> usize {
    let mut offset = end.min(text.len());
    while offset > 0 {
        match text.as_bytes()[offset - 1] {
            b'\n' | b'\r' => offset -= 1,
            _ => break,
        }
    }
    offset_to_line(text, offset)
}

fn collect_excludes(document: &Document<String>, root_dir: &Path) -> Vec<PathBuf> {
    let mut excludes = Vec::new();

    if let Some(list) = document
        .get("workspace")
        .and_then(|workspace| workspace.get("exclude"))
        .and_then(Item::as_array)
    {
        for value in list.iter() {
            if let Some(path) = value.as_str() {
                excludes.push(normalize_path(&root_dir.join(path)));
            }
        }
    }

    excludes
}

fn discover_members(
    document: &Document<String>,
    root_manifest: &Path,
    root_dir: &Path,
    excludes: &[PathBuf],
    root_text: &str,
) -> Result<Vec<PathBuf>, String> {
    let root_manifest = normalize_path(root_manifest);
    let mut manifests = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = Vec::new();

    if document.get("package").is_some() {
        add_member(
            root_manifest.clone(),
            excludes,
            &mut manifests,
            &mut seen,
            &mut pending,
        );
    }

    if let Some(members) = document
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Item::as_array)
    {
        for value in members.iter() {
            let Some(pattern) = value.as_str() else {
                return Err("workspace.members must contain only strings".to_string());
            };

            let joined = root_dir.join(pattern);
            if contains_glob(pattern) {
                let glob_pattern = joined.to_string_lossy().replace('\\', "/");
                let entries = glob::glob(&glob_pattern).map_err(|err| {
                    format!("invalid workspace member pattern {pattern:?}: {err}")
                })?;

                for entry in entries {
                    let path = entry.map_err(|err| {
                        format!("failed to expand workspace member pattern {pattern:?}: {err}")
                    })?;
                    add_member(
                        manifest_path_for(path),
                        excludes,
                        &mut manifests,
                        &mut seen,
                        &mut pending,
                    );
                }
            } else {
                add_member(
                    manifest_path_for(joined),
                    excludes,
                    &mut manifests,
                    &mut seen,
                    &mut pending,
                );
            }
        }
    }

    // Cargo automatically includes in-workspace path dependencies. Follow the
    // same rule so dependencies inherited by those crates are not considered
    // unused.
    let mut index = 0;
    while index < pending.len() {
        let manifest = pending[index].clone();
        index += 1;

        // The root manifest may have been rewritten in memory to merge
        // duplicate workspace dependencies. Re-parse that text instead of the
        // stale copy on disk.
        let text = if manifest == root_manifest {
            root_text.to_string()
        } else {
            fs::read_to_string(&manifest)
                .map_err(|err| format!("failed to read {}: {err}", manifest.display()))?
        };
        let document: Document<String> = text
            .parse()
            .map_err(|err| format!("failed to parse {}: {err}", manifest.display()))?;
        let crate_dir = manifest.parent().unwrap_or(root_dir).to_path_buf();

        let mut path_dependencies = Vec::new();
        visit_dependency_entries(&document, |_, item, _| {
            if let Some(path) = dependency_path(item) {
                path_dependencies.push(crate_dir.join(path));
            }
        });

        for path in path_dependencies {
            add_member(
                manifest_path_for(path),
                excludes,
                &mut manifests,
                &mut seen,
                &mut pending,
            );
        }
    }

    Ok(manifests)
}

fn add_member(
    manifest: PathBuf,
    excludes: &[PathBuf],
    manifests: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    pending: &mut Vec<PathBuf>,
) {
    if !manifest.is_file() {
        return;
    }

    let manifest = normalize_path(&manifest);
    if is_ignored_path(&manifest) || is_excluded(&manifest, excludes) {
        return;
    }

    if seen.insert(manifest.clone()) {
        manifests.push(manifest.clone());
        pending.push(manifest);
    }
}

fn manifest_path_for(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path
    }
}

fn contains_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '['])
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name == "target" || name == ".git")
    })
}

fn is_excluded(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|excluded| path.starts_with(excluded))
}

/// Lexically normalize a path without touching the filesystem.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn visit_dependency_entries<F>(document: &Document<String>, mut visit: F)
where
    F: FnMut(&str, &Item, &str),
{
    for section in DEPENDENCY_SECTIONS {
        if let Some(table) = document.get(section).and_then(Item::as_table_like) {
            for (name, item) in table.iter() {
                visit(name, item, section);
            }
        }
    }

    if let Some(targets) = document.get("target").and_then(Item::as_table_like) {
        for (_, target) in targets.iter() {
            let Some(target) = target.as_table_like() else {
                continue;
            };

            for section in DEPENDENCY_SECTIONS {
                if let Some(table) = target.get(section).and_then(Item::as_table_like) {
                    for (name, item) in table.iter() {
                        visit(name, item, section);
                    }
                }
            }
        }
    }
}

fn is_workspace_inherited(item: &Item) -> bool {
    match item {
        Item::Value(Value::InlineTable(table)) => table
            .get("workspace")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        Item::Table(table) => table
            .get("workspace")
            .and_then(Item::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

fn dependency_path(item: &Item) -> Option<&str> {
    match item {
        Item::Value(Value::InlineTable(table)) => table.get("path").and_then(Value::as_str),
        Item::Table(table) => table.get("path").and_then(Item::as_str),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cargo-undep-test-{}-{count}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn fixture_workspace(name: &str) -> PathBuf {
        let root = temp_dir(name);
        write(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["app", "tools/*"]
exclude = ["tools/ignored"]

[workspace.dependencies]
used-direct = "1"
used-dev = "1"
used-build = "1"
used-target = "1"
used-path = "1"

# remove me
unused-a = "1"

# ordinary comment
# snmp-parser = "0.11.0"
unused-b = { version = "1" }
"#,
        );
        write(
            &root.join("app/Cargo.toml"),
            r#"[package]
name = "app"
version = "0.1.0"

[dependencies]
used-direct = { workspace = true }
helper = { path = "../helper" }

[dev-dependencies]
used-dev = { workspace = true }

[target.'cfg(unix)'.dependencies]
used-target = { workspace = true }
"#,
        );
        write(
            &root.join("helper/Cargo.toml"),
            r#"[package]
name = "helper"
version = "0.1.0"

[dependencies]
used-path = { workspace = true }
"#,
        );
        write(
            &root.join("tools/build-helper/Cargo.toml"),
            r#"[package]
name = "build-helper"
version = "0.1.0"

[build-dependencies]
used-build = { workspace = true }
"#,
        );
        write(
            &root.join("tools/ignored/Cargo.toml"),
            r#"[package]
name = "ignored"
version = "0.1.0"

[dependencies]
unused-a = { workspace = true }
"#,
        );
        root
    }

    #[test]
    fn finds_unused_dependencies_across_all_sections() {
        let root = fixture_workspace("unused");
        let workspace = Workspace::load(&root.join("Cargo.toml")).unwrap();
        let unused: Vec<&str> = workspace
            .unused_dependencies()
            .unwrap()
            .iter()
            .map(|dependency| dependency.name.as_str())
            .collect();

        assert_eq!(unused, vec!["unused-a", "unused-b"]);
    }

    #[test]
    fn cleans_unused_dependencies_with_comment_rules() {
        let root = fixture_workspace("clean");
        let workspace = Workspace::load(&root.join("Cargo.toml")).unwrap();
        let unused = workspace.unused_dependencies().unwrap();
        let cleaned = crate::cleaner::remove_dependencies(&workspace.manifest_text, &unused);

        assert!(!cleaned.contains("unused-a"));
        assert!(!cleaned.contains("# remove me"));
        assert!(!cleaned.contains("unused-b"));
        assert!(cleaned.contains("# ordinary comment"));
        assert!(cleaned.contains("# snmp-parser = \"0.11.0\""));
        assert!(cleaned.contains("used-direct"));
    }

    #[test]
    fn discovers_path_dependency_members() {
        let root = fixture_workspace("path");
        let workspace = Workspace::load(&root.join("Cargo.toml")).unwrap();
        let manifests: Vec<String> = workspace
            .member_manifests
            .iter()
            .map(|path| {
                normalize_path(path)
                    .display()
                    .to_string()
                    .replace('\\', "/")
            })
            .collect();

        assert!(
            manifests
                .iter()
                .any(|path| path.ends_with("helper/Cargo.toml"))
        );
    }

    #[test]
    fn scopes_members_to_current_directory() {
        let root = fixture_workspace("scope");
        let workspace = Workspace::load(&root.join("Cargo.toml")).unwrap();

        let app = workspace.members_in_scope(&root.join("app"));
        assert_eq!(app, vec![normalize_path(&root.join("app/Cargo.toml"))]);

        let nested = workspace.members_in_scope(&root.join("app/src"));
        assert_eq!(nested, vec![normalize_path(&root.join("app/Cargo.toml"))]);

        let tools = workspace.members_in_scope(&root.join("tools"));
        assert_eq!(
            tools,
            vec![normalize_path(&root.join("tools/build-helper/Cargo.toml"))]
        );

        assert_eq!(
            workspace.members_in_scope(&root),
            workspace.member_manifests
        );
    }

    #[test]
    fn respects_exclude_list() {
        let root = fixture_workspace("exclude");
        let workspace = Workspace::load(&root.join("Cargo.toml")).unwrap();
        let unused: Vec<&str> = workspace
            .unused_dependencies()
            .unwrap()
            .iter()
            .map(|dependency| dependency.name.as_str())
            .collect();

        assert!(unused.contains(&"unused-a"));
    }

    #[test]
    fn load_for_sort_merges_duplicate_workspace_dependencies() {
        let root = temp_dir("sort-duplicates");
        let manifest = root.join("Cargo.toml");
        write(
            &manifest,
            r#"[package]
name = "root"
version = "0.1.0"

[workspace]
members = []

[workspace.dependencies]
serde = "1"
serde = { version = "1", features = ["derive"] }
"#,
        );

        assert!(Workspace::load(&manifest).is_err());
        assert_eq!(find_workspace_manifest(&manifest).unwrap(), manifest);

        let workspace = Workspace::load_for_sort(&manifest).unwrap();
        assert_eq!(workspace.merged_duplicates, 1);
        assert_eq!(workspace.dependencies.len(), 1);
        assert_eq!(workspace.dependencies[0].name, "serde");
        assert_eq!(workspace.manifest_text.matches("serde =").count(), 1);
        assert_eq!(
            workspace.original_manifest_text.matches("serde =").count(),
            2
        );
        assert!(workspace.manifest_text.contains("features = [\"derive\"]"));
    }
}
