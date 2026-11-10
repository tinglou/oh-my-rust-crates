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

use std::collections::BTreeMap;
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
    /// Number of duplicate `[workspace.dependencies]` declarations that were
    /// merged while preparing the plan.
    pub merged: usize,
}

/// Build a sorting plan for the workspace root and every member crate.
///
/// Returns a plan containing all `Cargo.toml` files that would be modified.
/// Duplicate entries in the root `[workspace.dependencies]` table are merged
/// into one entry before sorting. The root manifest is compared against
/// [`Workspace::original_manifest_text`], so a deduplication-only change is
/// still reported even when no table is large enough to sort.
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
        let is_root = manifest_path == workspace.manifest_path;
        let tables = if is_root {
            vec![DependencyTable::Workspace, DependencyTable::Dependencies]
        } else {
            vec![DependencyTable::Dependencies]
        };

        let sorted = sort_manifest(
            &text,
            &manifest_path,
            &tables,
            &categories,
            &workspace_dependencies,
        )?;
        let contents = sorted.unwrap_or_else(|| text.clone());
        let original = if is_root {
            &workspace.original_manifest_text
        } else {
            &text
        };

        if contents != *original {
            changes.push(FileChange {
                path: manifest_path,
                contents,
            });
        }
    }

    Ok(SortPlan {
        changes,
        merged: workspace.merged_duplicates,
    })
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
        // `cargo metadata` reads the manifest from disk, which still contains
        // the duplicate keys before `--fix` writes the merged text. Skip it and
        // let the caller re-plan after writing.
        if workspace.merged_duplicates > 0 {
            eprintln!(
                "warning: duplicate workspace dependencies were merged in memory; crate categories are unavailable until the merged manifest is written, so unclassified dependencies use misc for now."
            );
            return Self::default();
        }

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

    let mut contents = document.to_string();
    contents = align_inline_comments(&contents);
    let contents = restore_line_endings(text, contents);
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
    let mut emitted = BTreeMap::new();
    let mut is_first_group = true;
    for name in sorted_names {
        let Some(group) = groups.get(&name) else {
            continue;
        };
        let is_first_in_group = emitted.insert(group.clone(), ()).is_none();

        if let Some(mut key) = table.key_mut(&name) {
            let prefix = key
                .leaf_decor()
                .prefix()
                .and_then(|prefix| prefix.as_str())
                .unwrap_or_default();
            let prefix = strip_category_comments(prefix);

            if is_first_in_group {
                // First item in group: prepend category comment
                let comment = if is_first_group {
                    format!(
                        "{CATEGORY_COMMENT_PREFIX}{}{CATEGORY_COMMENT_SUFFIX}\n",
                        group.label()
                    )
                } else {
                    format!(
                        "\n{CATEGORY_COMMENT_PREFIX}{}{CATEGORY_COMMENT_SUFFIX}\n",
                        group.label()
                    )
                };
                is_first_group = false;
                key.leaf_decor_mut()
                    .set_prefix(format!("{comment}{prefix}"));
            } else {
                // Subsequent items in group: strip leading blank lines from prefix
                let prefix = trim_leading_blank_lines(&prefix);
                key.leaf_decor_mut().set_prefix(prefix);
            }
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

fn trim_leading_blank_lines(s: &str) -> String {
    let mut start = 0;
    let bytes = s.as_bytes();
    while start < bytes.len() {
        if bytes[start] == b'\n' || bytes[start] == b'\r' {
            start += 1;
        } else {
            break;
        }
    }
    s[start..].to_string()
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

/// Align inline suffix comments (`# ...`) within each dependency group so that
/// the `#` characters start at the same column.
///
/// Only lines inside a group (i.e. after a `# --- category ---` header) are
/// considered. Lines without an inline comment are left untouched. Category
/// header lines and bare comments are not treated as inline comments.
fn align_inline_comments(text: &str) -> String {
    let ends_with_newline = text.ends_with('\n');
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();

    // Assign each line to a group. Lines before the first header (or outside any
    // group) get `None`. Each `# --- ... ---` header starts a new group.
    let mut group_ids: Vec<Option<usize>> = vec![None; n];
    let mut current_group: Option<usize> = None;
    let mut group_count = 0usize;

    for (i, line) in lines.iter().enumerate() {
        if is_category_header(line) {
            current_group = Some(group_count);
            group_count += 1;
        }
        group_ids[i] = current_group;
    }

    // First pass: find the maximum inline-comment (`#`) byte offset per group.
    let mut max_comment_pos: Vec<usize> = vec![0; group_count];
    for (i, line) in lines.iter().enumerate() {
        let Some(g) = group_ids[i] else {
            continue;
        };
        if let Some(pos) = inline_comment_position(line) {
            max_comment_pos[g] = max_comment_pos[g].max(pos);
        }
    }

    // Second pass: emit lines, padding spaces before `#` to align within group.
    let mut output: Vec<String> = Vec::with_capacity(n);
    for (i, line) in lines.iter().enumerate() {
        let Some(g) = group_ids[i] else {
            output.push(line.to_string());
            continue;
        };
        match inline_comment_position(line) {
            Some(pos) if pos < max_comment_pos[g] => {
                let pad = max_comment_pos[g] - pos;
                let mut new_line = line.to_string();
                new_line.insert_str(pos, &" ".repeat(pad));
                output.push(new_line);
            }
            _ => output.push(line.to_string()),
        }
    }

    let mut result = output.join("\n");
    if ends_with_newline && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Returns `true` if `line` is a category header comment like `# --- misc ---`.
fn is_category_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with(CATEGORY_COMMENT_PREFIX) && trimmed.ends_with(CATEGORY_COMMENT_SUFFIX)
}

/// Returns the byte offset of the first `#` that begins an inline comment,
/// i.e. a `#` appearing after `=` and outside of any quoted string.
///
/// Returns `None` for lines without an inline comment (bare comments, category
/// headers, blank lines, and dependency declarations without a trailing comment).
fn inline_comment_position(line: &str) -> Option<usize> {
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;
    let mut found_eq = false;

    for (i, ch) in line.char_indices() {
        if in_double {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
        } else if in_single {
            if ch == '\'' {
                in_single = false;
            }
        } else if ch == '"' {
            in_double = true;
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '=' {
            found_eq = true;
        } else if ch == '#' && found_eq {
            return Some(i);
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

    #[test]
    fn aligns_inline_comments_within_a_group() {
        let text = r#"[dependencies]
aaa = "1" # short
bbbbbbbbbb = "2" # a longer comment
cc = "3" # medium length comment here
ddd = "4"
eee = "5" # another comment
fff = "6"
ggg = "7"
hhh = "8"
iii = "9"
jjj = "10"
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

        // Collect the byte offset of the inline `#` in each dependency line
        // (excluding the category header line).
        let positions: Vec<usize> = sorted
            .lines()
            .filter(|line| !is_category_header(line))
            .filter_map(inline_comment_position)
            .collect();

        assert!(!positions.is_empty());
        // All inline comments in the single misc group must share one column.
        assert!(
            positions.windows(2).all(|w| w[0] == w[1]),
            "comment positions should be aligned, got {positions:?}"
        );
    }

    #[test]
    fn aligns_inline_comments_per_group_independently() {
        let text = r#"[dependencies]
zeta = "1"
alpha = "1" # short comment
clap = "4" # this is a much longer comment
local = { path = "../local" }
serde = "1" # tiny
tokio = "1" # async runtime comment here
beta = "1"
gamma = "1" # beta-ish comment
delta = "1" # delta comment text
epsilon = "1" # epsilon comment here
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

        // Each group's comments should be aligned within that group, but
        // different groups may have different alignment columns.
        let mut prev_group_positions: Option<Vec<usize>> = None;
        let mut current_group_positions: Vec<usize> = Vec::new();

        for line in sorted.lines() {
            if is_category_header(line) {
                if let Some(prev) = prev_group_positions.take() {
                    assert!(
                        prev.windows(2).all(|w| w[0] == w[1]),
                        "previous group comments should be aligned"
                    );
                }
                prev_group_positions = Some(current_group_positions);
                current_group_positions = Vec::new();
                continue;
            }
            if let Some(pos) = inline_comment_position(line) {
                current_group_positions.push(pos);
            }
        }
        if let Some(prev) = prev_group_positions {
            assert!(
                prev.windows(2).all(|w| w[0] == w[1]),
                "last group comments should be aligned"
            );
        }
    }

    fn workspace_with_duplicate_root(text: &str) -> Workspace {
        let merged =
            crate::manifest::merge_workspace_dependency_duplicates(text, Path::new("Cargo.toml"))
                .unwrap();
        Workspace {
            root_dir: PathBuf::from("."),
            manifest_path: PathBuf::from("Cargo.toml"),
            manifest_text: merged.text,
            original_manifest_text: text.to_string(),
            merged_duplicates: merged.merged,
            dependencies: Vec::new(),
            member_manifests: Vec::new(),
        }
    }

    #[test]
    fn plans_duplicate_merge_even_when_table_is_small() {
        let text = r#"[workspace]
members = []

[workspace.dependencies]
# encoding comment
serde = "1" # category = encoding
serde = { version = "1.2", features = ["derive"] } # extra
"#;
        let workspace = workspace_with_duplicate_root(text);
        let plan = plan(&workspace).unwrap();

        assert_eq!(plan.merged, 1);
        assert_eq!(plan.changes.len(), 1);
        let contents = &plan.changes[0].contents;
        assert_eq!(contents.matches("serde =").count(), 1);
        assert!(contents.contains("features = [\"derive\"]"));
        assert!(contents.contains("# category = encoding"));
        assert!(contents.contains("# encoding comment"));
        assert!(contents.contains("# extra"));
    }

    #[test]
    fn alignment_is_idempotent() {
        let text = r#"[dependencies]
aaa = "1" # short
bbbbbbbbbb = "2" # a longer comment
cc = "3" # medium length comment here
ddd = "4"
eee = "5" # another comment
fff = "6"
ggg = "7"
hhh = "8"
iii = "9"
jjj = "10"
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
}
