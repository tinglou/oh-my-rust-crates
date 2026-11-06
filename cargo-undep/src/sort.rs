//! Sorting of dependency tables by path and crate category.
//!
//! `[workspace.dependencies]` and each member's `[dependencies]` table are
//! sorted when they contain at least ten entries. Path dependencies are placed
//! first, other dependencies are grouped by the first category reported by
//! Cargo (the part before `::`), and dependencies without a category are placed
//! last in a `misc` group.
//!
//! `toml_edit` preserves the raw formatting of each dependency, including
//! contiguous comments above it and inline comments after it. Only the order of
//! the entries changes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use toml_edit::{DocumentMut, Item, Table, Value};

use crate::workspace::Workspace;

const MIN_DEPENDENCIES: usize = 10;
const CATEGORY_COMMENT_PREFIX: &str = "# --- ";
const CATEGORY_COMMENT_SUFFIX: &str = " ---";

/// A file modification to be applied.
pub struct FileChange {
    /// Path to the file to modify.
    pub path: PathBuf,
    /// New file contents.
    pub contents: String,
}

/// Planned sorting changes for a workspace.
pub struct SortPlan {
    /// Files that need to be rewritten with sorted dependencies.
    pub changes: Vec<FileChange>,
}

/// Build a sorting plan for the workspace root and every member crate.
///
/// Returns a plan containing all `Cargo.toml` files that would be modified.
pub fn plan(workspace: &Workspace) -> Result<SortPlan, String> {
    let categories = Categories::load(workspace);
    let workspace_dependencies =
        workspace_dependency_index(&workspace.manifest_text, &workspace.manifest_path)?;

    let mut manifests: BTreeMap<PathBuf, String> = BTreeMap::new();
    manifests.insert(
        workspace.manifest_path.clone(),
        workspace.manifest_text.clone(),
    );

    for manifest_path in &workspace.member_manifests {
        if manifests.contains_key(manifest_path) {
            continue;
        }
        let text = fs::read_to_string(manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
        manifests.insert(manifest_path.clone(), text);
    }

    let mut changes = Vec::new();
    for (manifest_path, text) in manifests {
        let tables = if manifest_path == workspace.manifest_path {
            vec![DependencyTable::Workspace, DependencyTable::Dependencies]
        } else {
            vec![DependencyTable::Dependencies]
        };

        if let Some(contents) = sort_manifest(
            &text,
            &manifest_path,
            &tables,
            &categories,
            &workspace_dependencies,
        )? && contents != text
        {
            changes.push(FileChange {
                path: manifest_path,
                contents,
            });
        }
    }

    Ok(SortPlan { changes })
}

#[derive(Clone, Copy)]
enum DependencyTable {
    Workspace,
    Dependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Path,
    Category(String),
    Misc,
}

impl Group {
    fn label(&self) -> &str {
        match self {
            Self::Path => "path",
            Self::Category(category) => category,
            Self::Misc => "misc",
        }
    }
}

#[derive(Debug, Clone, Default)]
struct WorkspaceDependencyInfo {
    has_path: bool,
    package: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Default)]
struct Categories {
    first: BTreeMap<String, String>,
}

impl Categories {
    fn load(workspace: &Workspace) -> Self {
        match load_categories(workspace) {
            Ok(categories) => Self { first: categories },
            Err(error) => {
                eprintln!(
                    "warning: failed to load crate categories from `cargo metadata`: {error}; using misc for unclassified dependencies."
                );
                Self::default()
            }
        }
    }

    fn group(&self, package: &str) -> Group {
        match self.first.get(package) {
            Some(category) if !category.is_empty() => Group::Category(category.clone()),
            _ => Group::Misc,
        }
    }

    #[cfg(test)]
    fn from_entries(entries: &[(&str, &str)]) -> Self {
        let mut first = BTreeMap::new();
        for (package, category) in entries {
            if let Some(category) = normalize_category(category) {
                first.insert((*package).to_string(), category);
            }
        }
        Self { first }
    }
}

fn load_categories(workspace: &Workspace) -> Result<BTreeMap<String, String>, String> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            workspace.manifest_path.to_string_lossy().as_ref(),
        ])
        .output()
        .map_err(|error| format!("failed to run cargo metadata: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("failed to parse cargo metadata output: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata output has no `packages` array".to_string())?;

    let mut categories = BTreeMap::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(items) = package
            .get("categories")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        let Some(category) = items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .find_map(normalize_category)
        else {
            continue;
        };
        categories.entry(name.to_string()).or_insert(category);
    }

    Ok(categories)
}

fn normalize_category(category: &str) -> Option<String> {
    let category = category.split("::").next().unwrap_or(category).trim();
    (!category.is_empty()).then(|| category.to_string())
}

fn workspace_dependency_index(
    text: &str,
    manifest_path: &Path,
) -> Result<BTreeMap<String, WorkspaceDependencyInfo>, String> {
    let document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    let Some(table) = document
        .get("workspace")
        .and_then(Item::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Item::as_table)
    else {
        return Ok(BTreeMap::new());
    };

    let mut index = BTreeMap::new();
    for (name, item) in table.iter() {
        index.insert(
            name.to_string(),
            WorkspaceDependencyInfo {
                has_path: dependency_path(item).is_some(),
                package: dependency_package(item, name),
                category: comment_category(item),
            },
        );
    }
    Ok(index)
}

fn sort_manifest(
    text: &str,
    manifest_path: &Path,
    tables: &[DependencyTable],
    categories: &Categories,
    workspace_dependencies: &BTreeMap<String, WorkspaceDependencyInfo>,
) -> Result<Option<String>, String> {
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    let mut changed = false;

    for table_kind in tables {
        let table = match table_kind {
            DependencyTable::Workspace => document
                .get_mut("workspace")
                .and_then(Item::as_table_mut)
                .and_then(|workspace| workspace.get_mut("dependencies"))
                .and_then(Item::as_table_mut),
            DependencyTable::Dependencies => document
                .get_mut("dependencies")
                .and_then(Item::as_table_mut),
        };

        if let Some(table) = table
            && sort_table(table, categories, workspace_dependencies)
        {
            changed = true;
        }
    }

    if !changed {
        return Ok(None);
    }

    let contents = restore_line_endings(text, document.to_string());
    if contents == text {
        Ok(None)
    } else {
        Ok(Some(contents))
    }
}

fn sort_table(
    table: &mut Table,
    categories: &Categories,
    workspace_dependencies: &BTreeMap<String, WorkspaceDependencyInfo>,
) -> bool {
    if table.len() < MIN_DEPENDENCIES {
        return false;
    }

    let groups: BTreeMap<String, Group> = table
        .iter()
        .map(|(name, item)| {
            (
                name.to_string(),
                classify(name, item, categories, workspace_dependencies),
            )
        })
        .collect();

    let names: Vec<String> = table.iter().map(|(name, _)| name.to_string()).collect();
    for name in &names {
        if let Some(mut key) = table.key_mut(name) {
            let prefix = key
                .leaf_decor()
                .prefix()
                .and_then(|prefix| prefix.as_str())
                .unwrap_or_default();
            let stripped = strip_category_comments(prefix);
            if stripped != prefix {
                key.leaf_decor_mut().set_prefix(stripped);
            }
        }
    }

    table.sort_values_by(|left, _, right, _| {
        let left_group = groups.get(left.get());
        let right_group = groups.get(right.get());
        left_group
            .cmp(&right_group)
            .then_with(|| compare_dependency_names(left.get(), right.get()))
    });

    let sorted_names: Vec<String> = table.iter().map(|(name, _)| name.to_string()).collect();
    let mut emitted = BTreeSet::new();
    for name in sorted_names {
        let Some(group) = groups.get(&name) else {
            continue;
        };
        if !emitted.insert(group.clone()) {
            continue;
        }

        if let Some(mut key) = table.key_mut(&name) {
            let prefix = key
                .leaf_decor()
                .prefix()
                .and_then(|prefix| prefix.as_str())
                .unwrap_or_default();
            let prefix = strip_category_comments(prefix);
            let comment = format!(
                "\n{CATEGORY_COMMENT_PREFIX}{}{CATEGORY_COMMENT_SUFFIX}\n\n",
                group.label()
            );
            key.leaf_decor_mut()
                .set_prefix(format!("{comment}{prefix}"));
        }
    }

    true
}

fn classify(
    name: &str,
    item: &Item,
    categories: &Categories,
    workspace_dependencies: &BTreeMap<String, WorkspaceDependencyInfo>,
) -> Group {
    let inherited = is_workspace_inherited(item)
        .then(|| workspace_dependencies.get(name))
        .flatten();

    // A category written in the dependency's trailing comment has the highest
    // priority, even for path dependencies.
    if let Some(category) = comment_category(item)
        .or_else(|| inherited.and_then(|dependency| dependency.category.clone()))
    {
        return Group::Category(category);
    }

    let has_path = inherited
        .map(|dependency| dependency.has_path)
        .unwrap_or_else(|| dependency_path(item).is_some());
    if has_path {
        return Group::Path;
    }

    let package = inherited
        .and_then(|dependency| dependency.package.clone())
        .or_else(|| dependency_package(item, name));
    let package = package.as_deref().unwrap_or(name);

    categories.group(package)
}

fn compare_dependency_names(left: &str, right: &str) -> std::cmp::Ordering {
    left.to_ascii_lowercase()
        .cmp(&right.to_ascii_lowercase())
        .then_with(|| left.cmp(right))
}

fn strip_category_comments(prefix: &str) -> String {
    let mut output = String::with_capacity(prefix.len());
    let lines: Vec<_> = prefix.split_inclusive('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let content = line.trim_end_matches(['\r', '\n']).trim();
        if content.starts_with(CATEGORY_COMMENT_PREFIX)
            && content.ends_with(CATEGORY_COMMENT_SUFFIX)
        {
            // Remove all trailing blank lines from output
            while output.ends_with("\n") {
                output.pop();
                if output.ends_with('\r') {
                    output.pop();
                }
            }
            // Skip the category comment line itself
            // Skip all blank lines after it
            i += 1;
            while i < lines.len() {
                let next_content = lines[i].trim_end_matches(['\r', '\n']).trim();
                if next_content.is_empty() {
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }
        output.push_str(line);
        i += 1;
    }
    output
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

fn dependency_package(item: &Item, name: &str) -> Option<String> {
    let package = match item {
        Item::Value(Value::InlineTable(table)) => table.get("package").and_then(Value::as_str),
        Item::Table(table) => table.get("package").and_then(Item::as_str),
        _ => None,
    }?;
    (package != name).then(|| package.to_string())
}

fn comment_category(item: &Item) -> Option<String> {
    let comment = match item {
        Item::Value(value) => value.decor().suffix().and_then(|suffix| suffix.as_str()),
        Item::Table(table) => table.decor().suffix().and_then(|suffix| suffix.as_str()),
        _ => None,
    }?;

    category_from_comment(comment)
}

fn category_from_comment(comment: &str) -> Option<String> {
    for line in comment.lines() {
        let Some((_, comment)) = line.split_once('#') else {
            continue;
        };
        let comment = comment.trim();
        if !comment.starts_with("category") {
            continue;
        }

        let after_category = comment.strip_prefix("category").unwrap_or(comment).trim();
        if !after_category.starts_with('=') {
            continue;
        }

        let value = after_category
            .strip_prefix('=')
            .unwrap_or(after_category)
            .trim();
        let value = value.split_whitespace().next().unwrap_or(value);
        let value = value.trim_matches(['"', '\'', ',']);
        if let Some(category) = normalize_category(value) {
            return Some(category);
        }
    }

    None
}

fn restore_line_endings(original: &str, contents: String) -> String {
    if original.contains("\r\n") {
        contents.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        contents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn categories() -> Categories {
        Categories::from_entries(&[
            ("anyhow", "rust-patterns"),
            ("clap", "command-line-utility"),
            ("serde", "encoding"),
            ("tokio", "asynchronous"),
        ])
    }

    fn workspace_dependencies() -> BTreeMap<String, WorkspaceDependencyInfo> {
        BTreeMap::new()
    }

    #[test]
    fn normalizes_category_prefix() {
        assert_eq!(
            normalize_category("development-tools::profiling").as_deref(),
            Some("development-tools")
        );
        assert_eq!(normalize_category("").as_deref(), None);
    }

    #[test]
    fn sorts_path_then_categories_then_misc_and_preserves_comments() {
        let text = r#"[dependencies]
zeta = "1"
# comment for anyhow
anyhow = "1" # keep trailing comment
clap = "4"
local = { path = "../local" }
serde = "1"
tokio = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        let path_position = sorted.find("# --- path ---").unwrap();
        let category_position = sorted.find("# --- command-line-utility ---").unwrap();
        let encoding_position = sorted.find("# --- encoding ---").unwrap();
        let misc_position = sorted.find("# --- misc ---").unwrap();

        assert!(path_position < category_position);
        assert!(category_position < encoding_position);
        assert!(encoding_position < misc_position);
        assert!(sorted.contains("# comment for anyhow\nanyhow = \"1\" # keep trailing comment"));
        assert!(sorted.contains("local = { path = \"../local\" }"));
    }

    #[test]
    fn sorts_names_within_a_category() {
        let text = r#"[dependencies]
zeta = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
kappa = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        let alpha = sorted.find("alpha =").unwrap();
        let beta = sorted.find("beta =").unwrap();
        let delta = sorted.find("delta =").unwrap();
        assert!(alpha < beta && beta < delta);
        assert!(sorted.contains("# --- misc ---"));
    }

    #[test]
    fn leaves_small_tables_unchanged() {
        let text = "[dependencies]\nzeta = \"1\"\nalpha = \"1\"\n";
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap();
        assert!(sorted.is_none());
    }

    #[test]
    fn is_idempotent() {
        let text = r#"[dependencies]
zeta = "1"
anyhow = "1"
clap = "4"
local = { path = "../local" }
serde = "1"
tokio = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
"#;
        let first = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();
        let second = sort_manifest(
            &first,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap();
        assert!(second.is_none());
    }

    #[test]
    fn classifies_inherited_path_dependencies_from_workspace() {
        let mut index = BTreeMap::new();
        index.insert(
            "helper".to_string(),
            WorkspaceDependencyInfo {
                has_path: true,
                package: None,
                category: None,
            },
        );
        let text = r#"[dependencies]
zeta = "1"
helper = { workspace = true }
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("member/Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &index,
        )
        .unwrap()
        .unwrap();

        let path_position = sorted.find("# --- path ---").unwrap();
        let helper_position = sorted.find("helper =").unwrap();
        assert!(path_position < helper_position);
    }

    #[test]
    fn trailing_comment_category_has_highest_priority() {
        let text = r#"[dependencies]
zeta = "1"
local = { path = "../local" } # category = network-programming
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        let category_position = sorted.find("# --- network-programming ---").unwrap();
        let local_position = sorted.find("local =").unwrap();
        assert!(category_position < local_position);
        assert!(
            sorted.contains("local = { path = \"../local\" } # category = network-programming")
        );
        assert!(!sorted.contains("# --- path ---"));
    }

    #[test]
    fn trailing_comment_category_overrides_crate_category() {
        let text = r#"[dependencies]
serde = "1" # category = development-tools::profiling
zeta = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        assert!(sorted.contains("# --- development-tools ---"));
        assert!(!sorted.contains("# --- encoding ---"));
        assert!(sorted.contains("serde = \"1\" # category = development-tools::profiling"));
    }

    #[test]
    fn trailing_comment_category_without_spaces_and_extra_text() {
        let text = r#"[dependencies]
snmp2 = {version="0.5.3"} # category=network-programming snmp client sdk
zeta = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        assert!(sorted.contains("# --- network-programming ---"));
        assert!(sorted.contains(
            "snmp2 = {version=\"0.5.3\"} # category=network-programming snmp client sdk"
        ));
    }

    #[test]
    fn keeps_workspace_dependencies_separate_from_member_dependencies() {
        let text = r#"[workspace.dependencies]
zeta = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
kappa = "1"

[dependencies]
zeta = "1"
alpha = "1"
beta = "1"
gamma = "1"
delta = "1"
epsilon = "1"
eta = "1"
theta = "1"
iota = "1"
kappa = "1"
"#;
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Workspace, DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        let workspace_header = sorted.find("[workspace.dependencies]").unwrap();
        let dependencies_header = sorted.find("[dependencies]").unwrap();
        assert!(workspace_header < dependencies_header);
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let text = "[dependencies]\r\nzeta = \"1\"\r\nalpha = \"1\"\r\nbeta = \"1\"\r\ngamma = \"1\"\r\ndelta = \"1\"\r\nepsilon = \"1\"\r\neta = \"1\"\r\ntheta = \"1\"\r\niota = \"1\"\r\nkappa = \"1\"\r\n";
        let sorted = sort_manifest(
            text,
            Path::new("Cargo.toml"),
            &[DependencyTable::Dependencies],
            &categories(),
            &workspace_dependencies(),
        )
        .unwrap()
        .unwrap();

        assert!(sorted.contains("\r\n"));
        assert!(!sorted.replace("\r\n", "").contains('\n'));
    }
}
