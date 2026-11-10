use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use toml_edit::{DocumentMut, InlineTable, Item, Key, Table, Value};

use crate::workspace::{DEPENDENCY_SECTIONS, Workspace, normalize_path};

const SOURCE_KEYS: &[&str] = &[
    "version",
    "path",
    "git",
    "branch",
    "tag",
    "rev",
    "registry",
    "registry-index",
    "package",
];

/// Planned result of a hoisting operation.
pub struct HoistPlan {
    /// Dependencies that can be hoisted to the workspace level.
    pub hoisted: Vec<HoistedDependency>,
    /// Dependencies already in `[workspace.dependencies]` whose members will be
    /// rewritten to use `workspace = true`.
    pub enforced: Vec<HoistedDependency>,
    /// Dependencies that could not be hoisted due to conflicts.
    pub skipped: Vec<SkippedDependency>,
    /// Files that need to be rewritten.
    pub changes: Vec<FileChange>,
}

impl HoistPlan {
    /// Returns `true` if the plan has no changes to apply.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// A dependency that can be hoisted to `[workspace.dependencies]`.
pub struct HoistedDependency {
    /// The dependency name (key).
    pub name: String,
    /// A human-readable description of the source (version, path, or git).
    pub source: String,
    /// Number of member crates declaring this dependency.
    pub declarations: usize,
    /// Manifest paths of the member crates declaring this dependency.
    pub manifests: Vec<PathBuf>,
}

/// A dependency that could not be hoisted.
pub struct SkippedDependency {
    /// The dependency name.
    pub name: String,
    /// Explanation of why it was skipped.
    pub reason: String,
    /// The type of conflict that caused the skip.
    pub conflict_type: ConflictType,
}

/// The reason a dependency could not be hoisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Incompatible version requirements across members.
    VersionConflict,
    /// Mixed `default-features` values across members.
    DefaultFeaturesConflict,
    /// Already declared in `[workspace.dependencies]` (enforced instead).
    AlreadyInWorkspace,
    /// Any other conflict.
    Other,
}

/// A file modification to be applied.
pub struct FileChange {
    /// Path to the file to modify.
    pub path: PathBuf,
    /// New file contents.
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Location {
    manifest: PathBuf,
    target: Option<String>,
    section: String,
    name: String,
}

#[derive(Debug, Clone)]
struct Declaration {
    location: Location,
    package: Option<String>,
    default_features: bool,
    source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Version(String),
    Path {
        path: PathBuf,
        version: Option<String>,
    },
    Git {
        source: GitSource,
        version: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitSource {
    git: String,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
}

#[derive(Debug, Clone)]
struct Group {
    name: String,
    package: Option<String>,
    default_features: bool,
    default_features_conflict: bool,
    source: Source,
    declarations: Vec<Declaration>,
}

impl Group {
    fn new(name: String, declaration: Declaration) -> Self {
        Self {
            name,
            package: declaration.package.clone(),
            default_features: declaration.default_features,
            default_features_conflict: false,
            source: declaration.source.clone(),
            declarations: vec![declaration],
        }
    }

    fn merge(&mut self, declaration: &Declaration) -> Result<(), String> {
        if self.package != declaration.package {
            return Err("conflicting package names".to_string());
        }

        match (&mut self.source, &declaration.source) {
            (Source::Version(current), Source::Version(incoming)) => {
                let merged = merge_versions(current, incoming).ok_or_else(|| {
                    format!("incompatible version requirements `{current}` and `{incoming}`")
                })?;
                *current = merged;
            }
            (
                Source::Path { path, version },
                Source::Path {
                    path: incoming_path,
                    version: incoming_version,
                },
            ) => {
                if path.as_path() != incoming_path.as_path() {
                    return Err("different paths".to_string());
                }
                merge_optional_versions(version, incoming_version)?;
            }
            (
                Source::Git { source, version },
                Source::Git {
                    source: incoming_source,
                    version: incoming_version,
                },
            ) => {
                if source != incoming_source {
                    return Err("different git sources".to_string());
                }
                merge_optional_versions(version, incoming_version)?;
            }
            _ => return Err("conflicting source types".to_string()),
        }

        if self.default_features != declaration.default_features {
            self.default_features_conflict = true;
        }

        self.declarations.push(declaration.clone());
        Ok(())
    }
}

/// Compute a hoisting plan for the entire workspace.
pub fn plan(workspace: &Workspace) -> Result<HoistPlan, String> {
    plan_scoped(workspace, None)
}

/// Compute a hoisting plan scoped to a specific directory.
///
/// If `scope` is `None`, the entire workspace is processed. If `scope` points
/// inside a member crate, only that crate and its sub-crates are processed.
pub fn plan_scoped(workspace: &Workspace, scope: Option<&Path>) -> Result<HoistPlan, String> {
    let members = match scope {
        Some(scope) => workspace.members_in_scope(scope),
        None => workspace.member_manifests.clone(),
    };
    let mut documents = load_documents(workspace, &members)?;
    let root_names: BTreeSet<String> = workspace
        .dependencies
        .iter()
        .map(|dependency| dependency.name.clone())
        .collect();

    let mut groups: BTreeMap<String, Result<Group, String>> = BTreeMap::new();
    let mut skipped: BTreeMap<String, (String, ConflictType)> = BTreeMap::new();
    let mut modified = BTreeSet::new();

    for manifest in &members {
        let Some((_, document)) = documents.get(manifest) else {
            continue;
        };
        let manifest_dir = manifest.parent().unwrap_or(&workspace.root_dir);

        visit_dependency_entries(
            document,
            |target, section, name, item| match parse_declaration(
                manifest,
                manifest_dir,
                target,
                section,
                name,
                item,
            ) {
                Ok(ParseResult::Declaration(declaration)) => {
                    let declaration = *declaration;
                    match groups.entry(name.to_string()) {
                        Entry::Vacant(entry) => {
                            entry.insert(Ok(Group::new(name.to_string(), declaration)));
                        }
                        Entry::Occupied(mut entry) => {
                            if let Ok(group) = entry.get_mut()
                                && let Err(reason) = group.merge(&declaration)
                            {
                                let _ = entry.insert(Err(reason));
                            }
                        }
                    }
                }
                Ok(ParseResult::Inherited) => {}
                Ok(ParseResult::Skip(reason)) => {
                    skipped
                        .entry(name.to_string())
                        .or_insert((reason, ConflictType::Other));
                }
                Err(reason) => {
                    let conflict_type = if reason.contains("incompatible version") {
                        ConflictType::VersionConflict
                    } else if reason.contains("default-features") {
                        ConflictType::DefaultFeaturesConflict
                    } else if reason.contains("already declared") {
                        ConflictType::AlreadyInWorkspace
                    } else {
                        ConflictType::Other
                    };
                    skipped
                        .entry(name.to_string())
                        .or_insert((reason, conflict_type));
                }
            },
        );
    }

    let mut hoisted_groups = Vec::new();
    let mut enforced_groups = Vec::new();
    for (name, group) in groups {
        if skipped.contains_key(&name) {
            continue;
        }
        if root_names.contains(&name) {
            match group {
                Ok(group) => enforced_groups.push(group),
                Err(reason) => {
                    skipped.entry(name).or_insert((reason, ConflictType::Other));
                }
            }
        } else {
            match group {
                Ok(group) if group.default_features_conflict => {
                    skipped.entry(name).or_insert_with(|| {
                        (
                            "mixed default-features".to_string(),
                            ConflictType::DefaultFeaturesConflict,
                        )
                    });
                }
                Ok(group) => hoisted_groups.push(group),
                Err(reason) => {
                    let conflict_type = if reason.contains("incompatible version") {
                        ConflictType::VersionConflict
                    } else {
                        ConflictType::Other
                    };
                    skipped.entry(name).or_insert((reason, conflict_type));
                }
            }
        }
    }

    if !hoisted_groups.is_empty() {
        modified.insert(workspace.manifest_path.clone());
        let root_document = documents
            .get_mut(&workspace.manifest_path)
            .ok_or_else(|| {
                format!(
                    "failed to load workspace manifest {}",
                    workspace.manifest_path.display()
                )
            })?
            .1
            .as_table_mut();

        let workspace_item = root_document
            .entry("workspace")
            .or_insert_with(|| Item::Table(Table::new()));
        let workspace_table = workspace_item.as_table_like_mut().ok_or_else(|| {
            format!(
                "{} contains a non-table [workspace] entry",
                workspace.manifest_path.display()
            )
        })?;

        let dependencies_item = workspace_table
            .entry("dependencies")
            .or_insert_with(|| Item::Table(Table::new()));
        let dependencies_table = dependencies_item.as_table_like_mut().ok_or_else(|| {
            format!(
                "{} contains a non-table [workspace.dependencies] entry",
                workspace.manifest_path.display()
            )
        })?;

        for group in &hoisted_groups {
            let _ = dependencies_table
                .insert(&group.name, workspace_item_for(group, &workspace.root_dir));
        }
    }

    // Process enforced groups (already in workspace.dependencies) - rewrite member deps to workspace = true
    for group in &enforced_groups {
        for declaration in &group.declarations {
            modified.insert(declaration.location.manifest.clone());
            let document = documents
                .get_mut(&declaration.location.manifest)
                .ok_or_else(|| {
                    format!("failed to load {}", declaration.location.manifest.display())
                })?
                .1
                .as_table_mut();
            let item = dependency_item_mut(document, &declaration.location).ok_or_else(|| {
                format!(
                    "failed to locate dependency `{}` in {}",
                    declaration.location.name,
                    declaration.location.manifest.display()
                )
            })?;
            rewrite_member_dependency(item)?;
        }
    }

    for group in &hoisted_groups {
        for declaration in &group.declarations {
            modified.insert(declaration.location.manifest.clone());
            let document = documents
                .get_mut(&declaration.location.manifest)
                .ok_or_else(|| {
                    format!("failed to load {}", declaration.location.manifest.display())
                })?
                .1
                .as_table_mut();
            let item = dependency_item_mut(document, &declaration.location).ok_or_else(|| {
                format!(
                    "failed to locate dependency `{}` in {}",
                    declaration.location.name,
                    declaration.location.manifest.display()
                )
            })?;
            rewrite_member_dependency(item)?;
        }
    }

    let changes = documents
        .into_iter()
        .filter_map(|(path, (original, document))| {
            if !modified.contains(&path) {
                return None;
            }
            let contents = restore_line_endings(&original, document.to_string());
            (contents != original).then_some(FileChange { path, contents })
        })
        .collect();

    let hoisted = hoisted_groups
        .iter()
        .map(|group| HoistedDependency {
            name: group.name.clone(),
            source: describe_source(&group.source, &workspace.root_dir),
            declarations: group.declarations.len(),
            manifests: group
                .declarations
                .iter()
                .map(|declaration| declaration.location.manifest.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        })
        .collect();

    let skipped = skipped
        .into_iter()
        .map(|(name, (reason, conflict_type))| SkippedDependency {
            name,
            reason,
            conflict_type,
        })
        .collect();

    Ok(HoistPlan {
        hoisted,
        enforced: enforced_groups
            .iter()
            .map(|group| HoistedDependency {
                name: group.name.clone(),
                source: describe_source(&group.source, &workspace.root_dir),
                declarations: group.declarations.len(),
                manifests: group
                    .declarations
                    .iter()
                    .map(|declaration| declaration.location.manifest.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            })
            .collect(),
        skipped,
        changes,
    })
}

fn load_documents(
    workspace: &Workspace,
    members: &[PathBuf],
) -> Result<BTreeMap<PathBuf, (String, DocumentMut)>, String> {
    let mut documents = BTreeMap::new();
    let mut paths = Vec::with_capacity(members.len() + 1);
    paths.push(workspace.manifest_path.clone());
    paths.extend(members.iter().cloned());

    for path in paths {
        if documents.contains_key(&path) {
            continue;
        }

        let text = if path == workspace.manifest_path {
            workspace.manifest_text.clone()
        } else {
            fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?
        };
        let document = text
            .parse::<DocumentMut>()
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        documents.insert(path, (text, document));
    }

    Ok(documents)
}

fn visit_dependency_entries<F>(document: &DocumentMut, mut visit: F)
where
    F: FnMut(Option<&str>, &str, &str, &Item),
{
    let root = document.as_table();

    for section in DEPENDENCY_SECTIONS {
        if let Some(table) = root.get(section).and_then(Item::as_table_like) {
            for (name, item) in table.iter() {
                visit(None, section, name, item);
            }
        }
    }

    if let Some(targets) = root.get("target").and_then(Item::as_table_like) {
        for (target_name, target) in targets.iter() {
            let Some(target) = target.as_table_like() else {
                continue;
            };

            for section in DEPENDENCY_SECTIONS {
                if let Some(table) = target.get(section).and_then(Item::as_table_like) {
                    for (name, item) in table.iter() {
                        visit(Some(target_name), section, name, item);
                    }
                }
            }
        }
    }
}

fn dependency_item_mut<'a>(document: &'a mut Table, location: &Location) -> Option<&'a mut Item> {
    if let Some(target_name) = &location.target {
        let targets = document.get_mut("target")?.as_table_like_mut()?;
        let target = targets.get_mut(target_name)?.as_table_like_mut()?;
        target
            .get_mut(&location.section)?
            .as_table_like_mut()?
            .get_mut(&location.name)
    } else {
        document
            .get_mut(&location.section)?
            .as_table_like_mut()?
            .get_mut(&location.name)
    }
}

enum ParseResult {
    Declaration(Box<Declaration>),
    Inherited,
    Skip(String),
}

fn parse_declaration(
    manifest: &Path,
    manifest_dir: &Path,
    target: Option<&str>,
    section: &str,
    name: &str,
    item: &Item,
) -> Result<ParseResult, String> {
    if get_bool(item, "workspace").unwrap_or(false) {
        return Ok(ParseResult::Inherited);
    }

    if get_str(item, "registry").is_some() || get_str(item, "registry-index").is_some() {
        return Ok(ParseResult::Skip(
            "custom registry is not supported".to_string(),
        ));
    }

    let package =
        get_str(item, "package").and_then(|package| (package != name).then(|| package.to_string()));
    let default_features = get_bool(item, "default-features").unwrap_or(true);
    let version = get_str(item, "version").map(str::to_string);
    let path = get_str(item, "path");
    let git = get_str(item, "git");

    let source = if let Some(path) = path {
        if git.is_some() {
            return Ok(ParseResult::Skip(
                "both path and git are specified".to_string(),
            ));
        }
        Source::Path {
            path: normalize_path(&manifest_dir.join(path)),
            version,
        }
    } else if let Some(git) = git {
        Source::Git {
            source: GitSource {
                git: git.to_string(),
                branch: get_str(item, "branch").map(str::to_string),
                tag: get_str(item, "tag").map(str::to_string),
                rev: get_str(item, "rev").map(str::to_string),
            },
            version,
        }
    } else if let Some(version) = version {
        Source::Version(version)
    } else if let Some(version) = item.as_str() {
        Source::Version(version.to_string())
    } else {
        return Ok(ParseResult::Skip(
            "no version, path, or git source".to_string(),
        ));
    };

    Ok(ParseResult::Declaration(Box::new(Declaration {
        location: Location {
            manifest: manifest.to_path_buf(),
            target: target.map(str::to_string),
            section: section.to_string(),
            name: name.to_string(),
        },
        package,
        default_features,
        source,
    })))
}

fn get_str<'a>(item: &'a Item, key: &str) -> Option<&'a str> {
    match item {
        Item::Value(Value::InlineTable(table)) => table.get(key).and_then(Value::as_str),
        Item::Table(table) => table.get(key).and_then(Item::as_str),
        _ => None,
    }
}

fn get_bool(item: &Item, key: &str) -> Option<bool> {
    match item {
        Item::Value(Value::InlineTable(table)) => table.get(key).and_then(Value::as_bool),
        Item::Table(table) => table.get(key).and_then(Item::as_bool),
        _ => None,
    }
}

fn rewrite_member_dependency(item: &mut Item) -> Result<(), String> {
    match item {
        Item::Value(Value::String(_)) => {
            let decor = item.as_value().unwrap().decor().clone();
            let mut table = InlineTable::new();
            let _ = table.insert("workspace", Value::from(true));
            let mut value = Value::InlineTable(table);
            *value.decor_mut() = decor;
            *item = Item::Value(value);
            Ok(())
        }
        Item::Value(Value::InlineTable(table)) => {
            rewrite_inline_table(table);
            Ok(())
        }
        Item::Table(table) => {
            for key in SOURCE_KEYS {
                let _ = table.remove(key);
            }
            let _ = table.remove("workspace");
            let _ = table.insert("workspace", Item::Value(Value::from(true)));
            Ok(())
        }
        _ => Err("unsupported dependency declaration".to_string()),
    }
}

fn rewrite_inline_table(table: &mut InlineTable) {
    let decor = table.decor().clone();
    let dotted = table.is_dotted();
    let mut rewritten = InlineTable::new();
    rewritten.set_dotted(dotted);
    let _ = rewritten.insert("workspace", Value::from(true));

    for (key, value) in table.iter() {
        if key != "workspace" && !is_source_key(key) {
            let key = table.key(key).cloned().unwrap_or_else(|| Key::new(key));
            let _ = rewritten.insert_formatted(&key, value.clone());
        }
    }

    *table = rewritten;
    *table.decor_mut() = decor;
}

fn is_source_key(key: &str) -> bool {
    SOURCE_KEYS.contains(&key)
}

fn workspace_item_for(group: &Group, root_dir: &Path) -> Item {
    if group.default_features
        && group.package.is_none()
        && let Source::Version(version) = &group.source
    {
        return Item::Value(Value::from(version.clone()));
    }

    let mut table = InlineTable::new();

    if let Some(package) = &group.package {
        let _ = table.insert("package", Value::from(package.clone()));
    }

    match &group.source {
        Source::Version(version) => {
            let _ = table.insert("version", Value::from(version.clone()));
        }
        Source::Path { path, version } => {
            let _ = table.insert("path", Value::from(path_string(path, root_dir)));
            if let Some(version) = version {
                let _ = table.insert("version", Value::from(version.clone()));
            }
        }
        Source::Git { source, version } => {
            let _ = table.insert("git", Value::from(source.git.clone()));
            if let Some(branch) = &source.branch {
                let _ = table.insert("branch", Value::from(branch.clone()));
            }
            if let Some(tag) = &source.tag {
                let _ = table.insert("tag", Value::from(tag.clone()));
            }
            if let Some(rev) = &source.rev {
                let _ = table.insert("rev", Value::from(rev.clone()));
            }
            if let Some(version) = version {
                let _ = table.insert("version", Value::from(version.clone()));
            }
        }
    }

    if !group.default_features {
        let _ = table.insert("default-features", Value::from(false));
    }

    Item::Value(Value::InlineTable(table))
}

fn describe_source(source: &Source, root_dir: &Path) -> String {
    match source {
        Source::Version(version) => format!("version = {version:?}"),
        Source::Path { path, version } => {
            let mut parts = vec![format!("path = {:?}", path_string(path, root_dir))];
            if let Some(version) = version {
                parts.push(format!("version = {version:?}"));
            }
            parts.join(", ")
        }
        Source::Git { source, version } => {
            let mut parts = vec![format!("git = {:?}", source.git)];
            if let Some(branch) = &source.branch {
                parts.push(format!("branch = {branch:?}"));
            }
            if let Some(tag) = &source.tag {
                parts.push(format!("tag = {tag:?}"));
            }
            if let Some(rev) = &source.rev {
                parts.push(format!("rev = {rev:?}"));
            }
            if let Some(version) = version {
                parts.push(format!("version = {version:?}"));
            }
            parts.join(", ")
        }
    }
}

fn path_string(path: &Path, root_dir: &Path) -> String {
    let relative = relative_path(path, root_dir);
    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        relative.to_string_lossy().replace('\\', "/")
    }
}

fn restore_line_endings(original: &str, contents: String) -> String {
    if original.contains("\r\n") {
        contents.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        contents
    }
}

fn relative_path(path: &Path, base: &Path) -> PathBuf {
    let path_components: Vec<Component<'_>> = path.components().collect();
    let base_components: Vec<Component<'_>> = base.components().collect();
    let common = path_components
        .iter()
        .zip(&base_components)
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common..base_components.len() {
        relative.push("..");
    }
    for component in &path_components[common..] {
        relative.push(component.as_os_str());
    }
    relative
}

fn merge_optional_versions(
    current: &mut Option<String>,
    incoming: &Option<String>,
) -> Result<(), String> {
    match (current.clone(), incoming.clone()) {
        (None, None) => Ok(()),
        (Some(left), Some(right)) => {
            let merged = merge_versions(&left, &right).ok_or_else(|| {
                format!("incompatible version requirements `{left}` and `{right}`")
            })?;
            *current = Some(merged);
            Ok(())
        }
        _ => Err("version is specified only by some declarations".to_string()),
    }
}

pub(crate) fn merge_versions(left: &str, right: &str) -> Option<String> {
    if left == right {
        return Some(left.to_string());
    }
    if left == "*" {
        return Some(right.to_string());
    }
    if right == "*" {
        return Some(left.to_string());
    }

    let left = SimpleVersion::parse(left)?;
    let right = SimpleVersion::parse(right)?;

    if left.major != right.major {
        return None;
    }

    if left.major == 0 {
        match (left.minor, right.minor) {
            (Some(left_minor), Some(right_minor)) if left_minor == right_minor => {
                if left_minor == 0 && left.patch != right.patch {
                    return None;
                }
            }
            (None, None) => {
                if left.patch != right.patch {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(left.max(right).to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SimpleVersion {
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
}

impl SimpleVersion {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('^').unwrap_or(text);
        if text.is_empty() {
            return None;
        }

        let mut parts = text.split('.');
        let major = parse_version_part(parts.next()?)?;
        let minor = match parts.next() {
            Some(part) => Some(parse_version_part(part)?),
            None => None,
        };
        let patch = match parts.next() {
            Some(part) => Some(parse_version_part(part)?),
            None => None,
        };

        if parts.next().is_some() {
            return None;
        }

        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for SimpleVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.major)?;
        if let Some(minor) = self.minor {
            write!(formatter, ".{minor}")?;
            if let Some(patch) = self.patch {
                write!(formatter, ".{patch}")?;
            }
        }
        Ok(())
    }
}

fn parse_version_part(part: &str) -> Option<u64> {
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cargo-undep-hoist-test-{}-{count}-{name}",
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

    fn workspace(root: &Path) -> Workspace {
        Workspace::load(&root.join("Cargo.toml")).unwrap()
    }

    fn changed<'a>(plan: &'a HoistPlan, path: &Path) -> &'a str {
        plan.changes
            .iter()
            .find(|change| change.path == path)
            .map(|change| change.contents.as_str())
            .unwrap_or_else(|| panic!("no change for {}", path.display()))
    }

    fn changed_ending<'a>(plan: &'a HoistPlan, suffix: &str) -> &'a str {
        plan.changes
            .iter()
            .find(|change| {
                change
                    .path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .ends_with(suffix)
            })
            .map(|change| change.contents.as_str())
            .unwrap_or_else(|| panic!("no change ending with {suffix}"))
    }

    #[test]
    fn hoists_compatible_versions_and_preserves_member_attributes() {
        let root = temp_dir("basic");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"], optional = true }
helper = { path = "../helper", version = "0.1" }
"#,
        );
        write(
            &root.join("b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"

[dependencies]
serde = { version = "1.2" }
"#,
        );
        write(
            &root.join("helper/Cargo.toml"),
            "[package]\nname = \"helper\"\nversion = \"0.1.0\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert_eq!(plan.hoisted.len(), 2);
        assert!(plan.skipped.is_empty());

        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains(r#"serde = "1.2""#));
        assert!(root_text.contains(r#"helper = { path = "helper", version = "0.1" }"#));

        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(
            a_text.contains(
                r#"serde = { workspace = true, features = ["derive"], optional = true }"#
            )
        );
        assert!(a_text.contains("helper = { workspace = true }"));

        let b_text = changed_ending(&plan, "b/Cargo.toml");
        assert!(b_text.contains("serde = { workspace = true }"));
    }

    #[test]
    fn skips_mixed_default_features() {
        let root = temp_dir("mixed-default-features");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[dependencies]
serde = { version = "1", default-features = false }
"#,
        );
        write(
            &root.join("b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"

[dependencies]
serde = "1"
"#,
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert!(plan.hoisted.is_empty());
        assert!(plan.changes.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].name, "serde");
        assert!(plan.skipped[0].reason.contains("default-features"));
    }

    #[test]
    fn hoists_matching_default_features_false() {
        let root = temp_dir("default-features-false");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[dependencies]
helper = { path = "../helper", version = "0.1", default-features = false }
"#,
        );
        write(
            &root.join("b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"

[dependencies]
helper = { path = "../helper", version = "0.1", default-features = false }
"#,
        );
        write(
            &root.join("helper/Cargo.toml"),
            "[package]\nname = \"helper\"\nversion = \"0.1.0\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert_eq!(plan.hoisted.len(), 1);
        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains(
            r#"helper = { path = "helper", version = "0.1", default-features = false }"#
        ));
        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(a_text.contains("helper = { workspace = true, default-features = false }"));
    }

    #[test]
    fn skips_incompatible_versions() {
        let root = temp_dir("incompatible-versions");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nthiserror = \"1.0.26\"\n",
        );
        write(
            &root.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies]\nthiserror = \"2.0.18\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert!(plan.hoisted.is_empty());
        assert!(plan.changes.is_empty());
        assert_eq!(plan.skipped[0].name, "thiserror");
        assert!(plan.skipped[0].reason.contains("incompatible version"));
    }

    #[test]
    fn enforces_inheritance_for_dependencies_already_in_workspace() {
        let root = temp_dir("already-declared");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.dependencies]\nanyhow = \"1\"\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nanyhow = \"1\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert!(plan.hoisted.is_empty());
        assert_eq!(plan.enforced.len(), 1);
        assert_eq!(plan.enforced[0].name, "anyhow");
        assert!(!plan.changes.is_empty()); // member rewritten to workspace = true
        assert!(plan.skipped.is_empty());
    }

    #[test]
    fn hoists_target_specific_dependencies() {
        let root = temp_dir("target-dependencies");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
"#,
        );
        write(
            &root.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies]\nlibc = \"0.2.150\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert_eq!(plan.hoisted.len(), 1);
        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains(r#"libc = "0.2.150""#));
        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(a_text.contains("[target.'cfg(unix)'.dependencies]"));
        assert!(a_text.contains("libc = { workspace = true }"));
    }

    #[test]
    fn preserves_comments() {
        let root = temp_dir("comments");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.dependencies]\n# keep me\nexisting = \"1\"\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[dependencies]
# keep this comment
new-dep = "1"
"#,
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains("# keep me"));
        assert!(root_text.contains("existing = \"1\""));
        assert!(root_text.contains("new-dep = \"1\""));

        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(a_text.contains("# keep this comment"));
        assert!(a_text.contains("new-dep = { workspace = true }"));
    }

    #[test]
    fn scopes_hoist_to_current_directory() {
        let root = temp_dir("scoped");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
        );
        write(
            &root.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies]\nanyhow = \"1\"\n",
        );

        let workspace = workspace(&root);
        let plan = plan_scoped(&workspace, Some(&root.join("a"))).unwrap();

        assert_eq!(plan.hoisted.len(), 1);
        assert_eq!(plan.hoisted[0].name, "serde");
        assert!(
            plan.changes
                .iter()
                .all(|change| !change.path.ends_with("b/Cargo.toml"))
        );

        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains(r#"serde = "1""#));
        assert!(!root_text.contains("anyhow"));
        assert!(changed_ending(&plan, "a/Cargo.toml").contains("serde = { workspace = true }"));
    }

    #[test]
    fn hoists_package_aliases_and_git_sources() {
        let root = temp_dir("git-alias");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[dependencies]
foo = { package = "bar", git = "https://example.com/bar", rev = "abc123", version = "0.1" }
"#,
        );
        write(
            &root.join("b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"

[dependencies]
foo = { package = "bar", git = "https://example.com/bar", rev = "abc123", version = "0.1.5" }
"#,
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        assert_eq!(plan.hoisted.len(), 1);
        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains(
            r#"foo = { package = "bar", git = "https://example.com/bar", rev = "abc123", version = "0.1.5" }"#
        ));
        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(a_text.contains("foo = { workspace = true }"));
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let root = temp_dir("crlf");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\r\nmembers = [\"a\"]\r\n\r\n[workspace.dependencies]\r\n",
        );
        write(
            &root.join("a/Cargo.toml"),
            "[package]\r\nname = \"a\"\r\nversion = \"0.1.0\"\r\n\r\n[dependencies]\r\nanyhow = \"1\"\r\n",
        );

        let workspace = workspace(&root);
        let plan = plan(&workspace).unwrap();

        let root_text = changed(&plan, &workspace.manifest_path);
        assert!(root_text.contains("\r\n"));
        assert!(!root_text.replace("\r\n", "").contains('\n'));

        let a_text = changed_ending(&plan, "a/Cargo.toml");
        assert!(a_text.contains("\r\n"));
        assert!(!a_text.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn merges_simple_version_requirements_conservatively() {
        assert_eq!(merge_versions("1.0.102", "1"), Some("1.0.102".to_string()));
        assert_eq!(merge_versions("1.2", "1.3.1"), Some("1.3.1".to_string()));
        assert_eq!(merge_versions("0.2", "0.2.5"), Some("0.2.5".to_string()));
        assert_eq!(merge_versions("2.0", "1.0"), None);
        assert_eq!(merge_versions("0.2", "0.3"), None);
        assert_eq!(merge_versions("0.0.1", "0.0.2"), None);
        assert_eq!(merge_versions("=1.0", "1.0"), None);
        assert_eq!(merge_versions("*", "1.0"), Some("1.0".to_string()));
    }
}
