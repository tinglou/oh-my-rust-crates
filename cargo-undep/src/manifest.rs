//! Duplicate dependency merging for `[workspace.dependencies]`.
//!
//! Duplicate keys are not valid TOML, so `toml_edit` cannot parse a manifest
//! that contains them. This module uses the lower-level `toml_parser` event
//! stream to locate the duplicate entries, merges their values, and rebuilds
//! the manifest text before it is parsed by `toml_edit`.
//!
//! Only `[workspace.dependencies]` is handled. Duplicate keys in member
//! manifests or in the root package's `[dependencies]` table are left alone and
//! are reported by the normal TOML parser.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};
use toml_parser::parser::{Event, EventKind, parse_document};
use toml_parser::{Raw, Source};

use crate::text::{has_category_comment, is_category_header, restore_line_endings};

const WORKSPACE_DEPENDENCIES: [&str; 2] = ["workspace", "dependencies"];

/// Result of merging duplicate workspace dependencies.
#[derive(Debug)]
pub(crate) struct MergeResult {
    /// The rewritten manifest text.
    pub text: String,
    /// Number of duplicate declarations that were merged away.
    pub merged: usize,
}

/// Merge duplicate entries in `[workspace.dependencies]`.
///
/// Returns the original text unchanged when there are no duplicates. Conflicting
/// versions or sources are reported as errors so the caller can abort without
/// writing a partial result.
pub(crate) fn merge_workspace_dependency_duplicates(
    text: &str,
    path: &Path,
) -> Result<MergeResult, String> {
    let collected = collect_entries(text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;

    let mut table_forms = BTreeSet::new();
    for name in &collected.table_forms {
        if !table_forms.insert(name.clone()) {
            return Err(format!(
                "duplicate `[workspace.dependencies.{name}]` table in {}",
                path.display()
            ));
        }
        if collected.entries.iter().any(|entry| entry.key == *name) {
            return Err(format!(
                "dependency `{name}` in {} is declared both as a value and as a table",
                path.display()
            ));
        }
    }

    let mut groups: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    for entry in collected.entries {
        groups.entry(entry.key.clone()).or_default().push(entry);
    }

    let mut replacements = Vec::new();
    let mut merged = 0;
    for (key, entries) in groups {
        if entries.len() < 2 {
            continue;
        }
        let (mut group_replacements, count) = merge_group(text, path, &key, &entries)?;
        replacements.append(&mut group_replacements);
        merged += count;
    }

    if merged == 0 {
        return Ok(MergeResult {
            text: text.to_string(),
            merged: 0,
        });
    }

    let output = apply_replacements(text, replacements)?;
    Ok(MergeResult {
        text: restore_line_endings(text, output),
        merged,
    })
}

struct Collected {
    entries: Vec<Entry>,
    table_forms: Vec<String>,
}

struct Entry {
    key: String,
    key_start: usize,
    value_start: usize,
    value_end: usize,
    prefix_start: usize,
    line_end: usize,
}

struct EntryBuilder {
    key: String,
    key_parts: usize,
    key_start: usize,
    value_start: Option<usize>,
    value_end: Option<usize>,
    target: bool,
}

fn collect_entries(text: &str) -> Result<Collected, String> {
    let source = Source::new(text);
    let tokens = source.lex().into_vec();
    let mut events = Vec::new();
    let mut errors = Vec::new();
    parse_document(&tokens, &mut events, &mut errors);

    if let Some(error) = errors.first() {
        return Err(error.description().to_string());
    }

    let mut table_path: Vec<String> = Vec::new();
    let mut header_keys: Vec<String> = Vec::new();
    let mut in_header = false;
    let mut header_is_array = false;
    let mut current_table_is_array = false;
    let mut entry: Option<EntryBuilder> = None;
    let mut depth = 0usize;
    let mut entries = Vec::new();
    let mut table_forms = Vec::new();

    for event in &events {
        match event.kind() {
            EventKind::StdTableOpen | EventKind::ArrayTableOpen => {
                in_header = true;
                header_is_array = matches!(event.kind(), EventKind::ArrayTableOpen);
                header_keys.clear();
                entry = None;
                depth = 0;
            }
            EventKind::StdTableClose | EventKind::ArrayTableClose => {
                in_header = false;
                current_table_is_array = header_is_array;
                table_path = std::mem::take(&mut header_keys);
                if !header_is_array
                    && table_path.len() == 3
                    && table_path[0] == WORKSPACE_DEPENDENCIES[0]
                    && table_path[1] == WORKSPACE_DEPENDENCIES[1]
                {
                    table_forms.push(table_path[2].clone());
                }
                entry = None;
                depth = 0;
            }
            EventKind::SimpleKey => {
                let key = decode_key(text, event)?;
                if in_header {
                    header_keys.push(key);
                } else if depth == 0 {
                    if let Some(builder) = entry.as_mut() {
                        builder.key_parts += 1;
                    } else {
                        entry = Some(EntryBuilder {
                            key,
                            key_parts: 1,
                            key_start: event.span().start(),
                            value_start: None,
                            value_end: None,
                            target: !current_table_is_array && table_path == WORKSPACE_DEPENDENCIES,
                        });
                    }
                }
            }
            EventKind::Scalar => {
                if let Some(builder) = entry.as_mut()
                    && depth == 0
                    && builder.value_start.is_none()
                {
                    builder.value_start = Some(event.span().start());
                    builder.value_end = Some(event.span().end());
                }
            }
            EventKind::InlineTableOpen | EventKind::ArrayOpen => {
                if let Some(builder) = entry.as_mut() {
                    if depth == 0 && builder.value_start.is_none() {
                        builder.value_start = Some(event.span().start());
                    }
                    depth += 1;
                }
            }
            EventKind::InlineTableClose | EventKind::ArrayClose => {
                if let Some(builder) = entry.as_mut()
                    && depth > 0
                {
                    depth -= 1;
                    if depth == 0 {
                        builder.value_end = Some(event.span().end());
                    }
                }
            }
            EventKind::Newline if depth == 0 => {
                if let Some(builder) = entry.take() {
                    finish_entry(text, builder, &mut entries);
                }
            }
            _ => {}
        }
    }

    if let Some(builder) = entry {
        finish_entry(text, builder, &mut entries);
    }

    Ok(Collected {
        entries,
        table_forms,
    })
}

fn finish_entry(text: &str, builder: EntryBuilder, entries: &mut Vec<Entry>) {
    if builder.key_parts != 1 || !builder.target {
        return;
    }
    let (Some(value_start), Some(value_end)) = (builder.value_start, builder.value_end) else {
        return;
    };
    entries.push(Entry {
        key: builder.key,
        key_start: builder.key_start,
        value_start,
        value_end,
        prefix_start: find_prefix_start(text, builder.key_start),
        line_end: line_end_including_newline(text, value_end),
    });
}

fn decode_key(text: &str, event: &Event) -> Result<String, String> {
    let span = event.span();
    let raw_text = text
        .get(span.start()..span.end())
        .ok_or_else(|| "invalid key span".to_string())?;
    let raw = Raw::new_unchecked(raw_text, event.encoding(), span);
    let mut key = String::new();
    let mut error = None;
    raw.decode_key(&mut key, &mut error);
    if let Some(error) = error {
        return Err(error.description().to_string());
    }
    Ok(key)
}

fn merge_group(
    text: &str,
    path: &Path,
    key: &str,
    entries: &[Entry],
) -> Result<(Vec<Replacement>, usize), String> {
    let first = &entries[0];
    let first_key_line_start = line_start(text, first.key_start);
    let first_prefix = &text[first.prefix_start..first_key_line_start];
    let first_item = parse_value(&text[first.value_start..first.value_end], key, path)?;
    let (mut merged_table, first_was_string) = normalize_item(&first_item, key, path)?;

    let mut comments = prefix_comments(text, first.prefix_start, first_key_line_start);
    let mut trailing = Vec::new();
    let first_trailing = trailing_comment(text, first.value_end, first.line_end);
    if !first_trailing.trim().is_empty() {
        trailing.push(first_trailing);
    }

    for entry in &entries[1..] {
        let item = parse_value(&text[entry.value_start..entry.value_end], key, path)?;
        let (table, _) = normalize_item(&item, key, path)?;
        merged_table = merge_tables(merged_table, &table, key, path)?;
        comments.extend(prefix_comments(
            text,
            entry.prefix_start,
            line_start(text, entry.key_start),
        ));
        let comment = trailing_comment(text, entry.value_end, entry.line_end);
        if !comment.trim().is_empty() {
            trailing.push(comment);
        }
    }

    check_workspace_source_conflicts(&merged_table, key, path)?;

    let inline = trailing
        .iter()
        .find(|comment| has_category_comment(comment))
        .or_else(|| trailing.first())
        .cloned()
        .unwrap_or_default();
    for comment in &trailing {
        if *comment != inline {
            let trimmed = comment.trim();
            if !trimmed.is_empty() {
                comments.push(trimmed.to_string());
            }
        }
    }

    let mut seen = BTreeSet::new();
    comments.retain(|comment| seen.insert(comment.clone()));
    let prefix = build_prefix(first_prefix, &comments);
    let key_part = &text[first.key_start..first.value_start];
    let value = render_value(&merged_table, first_was_string);

    let mut block = String::new();
    block.push_str(&prefix);
    block.push_str(key_part);
    block.push_str(&value);
    if !inline.is_empty() {
        if !inline.starts_with([' ', '\t']) {
            block.push(' ');
        }
        block.push_str(&inline);
    }
    block.push('\n');

    let mut replacements = Vec::with_capacity(entries.len());
    replacements.push(Replacement {
        start: first.prefix_start,
        end: first.line_end,
        text: block,
    });
    for entry in &entries[1..] {
        replacements.push(Replacement {
            start: entry.prefix_start,
            end: entry.line_end,
            text: String::new(),
        });
    }

    Ok((replacements, entries.len() - 1))
}

fn normalize_item(item: &Item, key: &str, path: &Path) -> Result<(InlineTable, bool), String> {
    match item {
        Item::Value(Value::String(version)) => {
            let mut table = InlineTable::new();
            table.insert("version", Value::from(version.value().clone()));
            Ok((table, true))
        }
        Item::Value(Value::InlineTable(table)) => Ok((table.clone(), false)),
        Item::Table(_) => Err(format!(
            "dependency `{key}` in {} uses table form, which cannot be merged automatically",
            path.display()
        )),
        _ => Err(format!(
            "dependency `{key}` in {} has an unsupported value",
            path.display()
        )),
    }
}

fn merge_tables(
    mut merged: InlineTable,
    incoming: &InlineTable,
    key: &str,
    path: &Path,
) -> Result<InlineTable, String> {
    for (field, value) in incoming.iter() {
        match field {
            "version" => {
                if let Some(existing) = merged.get("version") {
                    let left = existing.as_str().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-string version",
                            path.display()
                        )
                    })?;
                    let right = value.as_str().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-string version",
                            path.display()
                        )
                    })?;
                    let version = crate::hoist::merge_versions(left, right).ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has incompatible versions `{left}` and `{right}`",
                            path.display()
                        )
                    })?;
                    merged.insert("version", Value::from(version));
                } else {
                    merged.insert("version", value.clone());
                }
            }
            "features" => {
                if let Some(existing) = merged.get("features") {
                    merged.insert("features", merge_features(existing, value, key, path)?);
                } else {
                    merged.insert("features", value.clone());
                }
            }
            "default-features" => {
                if let Some(existing) = merged.get("default-features") {
                    let left = existing.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `default-features`",
                            path.display()
                        )
                    })?;
                    let right = value.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `default-features`",
                            path.display()
                        )
                    })?;
                    if left != right {
                        return Err(format!(
                            "dependency `{key}` in {} has conflicting `default-features` values",
                            path.display()
                        ));
                    }
                } else {
                    merged.insert("default-features", value.clone());
                }
            }
            "optional" | "public" => {
                if let Some(existing) = merged.get(field) {
                    let left = existing.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `{field}`",
                            path.display()
                        )
                    })?;
                    let right = value.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `{field}`",
                            path.display()
                        )
                    })?;
                    merged.insert(field, Value::from(left || right));
                } else {
                    merged.insert(field, value.clone());
                }
            }
            "workspace" => {
                if let Some(existing) = merged.get("workspace") {
                    let left = existing.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `workspace`",
                            path.display()
                        )
                    })?;
                    let right = value.as_bool().ok_or_else(|| {
                        format!(
                            "dependency `{key}` in {} has a non-boolean `workspace`",
                            path.display()
                        )
                    })?;
                    if left != right {
                        return Err(format!(
                            "dependency `{key}` in {} has conflicting `workspace` values",
                            path.display()
                        ));
                    }
                } else {
                    merged.insert("workspace", value.clone());
                }
            }
            "path" | "git" | "branch" | "tag" | "rev" | "package" | "registry"
            | "registry-index" => {
                if let Some(existing) = merged.get(field) {
                    if existing.to_string() != value.to_string() {
                        return Err(format!(
                            "dependency `{key}` in {} has conflicting `{field}` values",
                            path.display()
                        ));
                    }
                } else {
                    merged.insert(field, value.clone());
                }
            }
            _ => {
                if let Some(existing) = merged.get(field) {
                    if existing.to_string() != value.to_string() {
                        return Err(format!(
                            "dependency `{key}` in {} has conflicting `{field}` values",
                            path.display()
                        ));
                    }
                } else {
                    merged.insert(field, value.clone());
                }
            }
        }
    }
    Ok(merged)
}

fn merge_features(
    existing: &Value,
    incoming: &Value,
    key: &str,
    path: &Path,
) -> Result<Value, String> {
    let mut array = Array::new();
    let mut seen = BTreeSet::new();
    for value in [existing, incoming] {
        let Some(features) = value.as_array() else {
            return Err(format!(
                "dependency `{key}` in {} has a non-array `features` value",
                path.display()
            ));
        };
        for feature in features.iter() {
            let Some(feature) = feature.as_str() else {
                return Err(format!(
                    "dependency `{key}` in {} has a non-string feature",
                    path.display()
                ));
            };
            if seen.insert(feature.to_string()) {
                array.push(feature.to_string());
            }
        }
    }
    Ok(Value::Array(array))
}

fn check_workspace_source_conflicts(
    table: &InlineTable,
    key: &str,
    path: &Path,
) -> Result<(), String> {
    if table.get("workspace").and_then(Value::as_bool) != Some(true) {
        return Ok(());
    }
    for field in [
        "version",
        "path",
        "git",
        "branch",
        "tag",
        "rev",
        "registry",
        "registry-index",
    ] {
        if table.contains_key(field) {
            return Err(format!(
                "dependency `{key}` in {} combines `workspace = true` with `{field}`",
                path.display()
            ));
        }
    }
    Ok(())
}

fn parse_value(value: &str, key: &str, path: &Path) -> Result<Item, String> {
    let snippet = format!("[dependencies]\n__cargo_undep_dep__ = {value}\n");
    let document: DocumentMut = snippet.parse().map_err(|error| {
        format!(
            "failed to parse dependency `{key}` in {}: {error}",
            path.display()
        )
    })?;
    document
        .get("dependencies")
        .and_then(Item::as_table)
        .and_then(|table| table.get("__cargo_undep_dep__"))
        .cloned()
        .ok_or_else(|| format!("failed to read dependency `{key}` in {}", path.display()))
}

fn render_value(table: &InlineTable, first_was_string: bool) -> String {
    if first_was_string
        && table.iter().count() == 1
        && let Some(version) = table.get("version").and_then(Value::as_str)
    {
        return Value::from(version.to_string()).to_string();
    }
    Value::InlineTable(table.clone()).to_string()
}

fn build_prefix(first_prefix: &str, comments: &[String]) -> String {
    let starts_with_blank = first_prefix
        .lines()
        .next()
        .is_some_and(|line| line.trim().is_empty());
    let mut prefix = String::new();
    if comments.is_empty() {
        if starts_with_blank {
            prefix.push('\n');
        }
        return prefix;
    }

    if starts_with_blank {
        prefix.push('\n');
    }
    for comment in comments {
        prefix.push_str(comment);
        prefix.push('\n');
    }
    prefix
}

fn prefix_comments(text: &str, start: usize, end: usize) -> Vec<String> {
    text[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('#') && !is_category_header(line))
        .map(str::to_string)
        .collect()
}

fn trailing_comment(text: &str, value_end: usize, line_end: usize) -> String {
    text[value_end..line_end]
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

fn find_prefix_start(text: &str, key_start: usize) -> usize {
    let mut start = line_start(text, key_start);
    while start > 0 {
        let previous_start = line_start(text, start - 1);
        let line = &text[previous_start..start];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            start = previous_start;
        } else {
            break;
        }
    }
    start
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn line_end_including_newline(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map(|index| offset + index + 1)
        .unwrap_or(text.len())
}

fn apply_replacements(text: &str, mut replacements: Vec<Replacement>) -> Result<String, String> {
    replacements.sort_by_key(|replacement| replacement.start);
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for replacement in replacements {
        if replacement.start < cursor {
            return Err("overlapping duplicate dependency entries".to_string());
        }
        output.push_str(&text[cursor..replacement.start]);
        output.push_str(&replacement.text);
        cursor = replacement.end;
    }
    output.push_str(&text[cursor..]);
    Ok(output)
}

struct Replacement {
    start: usize,
    end: usize,
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn merge(text: &str) -> MergeResult {
        merge_workspace_dependency_duplicates(text, &PathBuf::from("Cargo.toml")).unwrap()
    }

    #[test]
    fn merges_duplicate_workspace_dependencies() {
        let text = r#"[workspace.dependencies]
serde = "1"
serde = { version = "1", features = ["derive"] }
serde = { version = "1.0", features = ["std"] }
"#;
        let result = merge(text);
        assert_eq!(result.merged, 2);
        assert_eq!(result.text.matches("serde =").count(), 1);
        assert!(result.text.contains("features = [\"derive\", \"std\"]"));
    }

    #[test]
    fn merges_string_and_inline_table() {
        let text = r#"[workspace.dependencies]
serde = "1"
serde = { version = "1.2", features = ["derive"] }
"#;
        let result = merge(text);
        assert!(
            result
                .text
                .contains("serde = { version = \"1.2\", features = [\"derive\"] }")
        );
    }

    #[test]
    fn preserves_comments() {
        let text = r#"[workspace.dependencies]
# first comment
serde = "1" # category = encoding
# second comment
serde = { version = "1", features = ["derive"] } # extra
"#;
        let result = merge(text);
        assert!(result.text.contains("# first comment"));
        assert!(result.text.contains("# second comment"));
        assert!(result.text.contains("# extra"));
        assert!(result.text.contains("# category = encoding"));
        assert_eq!(result.text.matches("serde =").count(), 1);
    }

    #[test]
    fn keeps_category_comment_inline() {
        let text = r#"[workspace.dependencies]
serde = "1" # category = encoding
serde = { version = "1", features = ["derive"] } # ordinary
"#;
        let result = merge(text);
        let serde_line = result
            .text
            .lines()
            .find(|line| line.starts_with("serde ="))
            .unwrap();
        assert!(serde_line.contains("# category = encoding"));
        assert!(result.text.contains("# ordinary"));
    }

    #[test]
    fn errors_on_version_conflict() {
        let text = r#"[workspace.dependencies]
serde = "1"
serde = "2"
"#;
        let error =
            merge_workspace_dependency_duplicates(text, Path::new("Cargo.toml")).unwrap_err();
        assert!(error.contains("incompatible versions"));
    }

    #[test]
    fn errors_on_source_conflict() {
        let text = r#"[workspace.dependencies]
helper = { path = "../a" }
helper = { path = "../b" }
"#;
        let error =
            merge_workspace_dependency_duplicates(text, Path::new("Cargo.toml")).unwrap_err();
        assert!(error.contains("conflicting `path`"));
    }

    #[test]
    fn ignores_member_dependencies() {
        let text = r#"[dependencies]
serde = "1"
serde = "2"
"#;
        let result = merge(text);
        assert_eq!(result.merged, 0);
        assert_eq!(result.text, text);
    }

    #[test]
    fn keeps_category_comment_inline_requires_equals() {
        // `# categorize = ...` is prose, not a `# category = ...` annotation,
        // so the real annotation on the second entry wins the inline slot.
        let text = r#"[workspace.dependencies]
serde = "1" # categorize = encoding
serde = { version = "1", features = ["derive"] } # category = encoding
"#;
        let result = merge(text);
        let serde_line = result
            .text
            .lines()
            .find(|line| line.starts_with("serde ="))
            .unwrap();
        assert!(serde_line.contains("# category = encoding"));
        assert!(result.text.contains("# categorize = encoding"));
    }

    #[test]
    fn is_idempotent() {
        let text = r#"[workspace.dependencies]
serde = "1"
serde = { version = "1", features = ["derive"] }
"#;
        let first = merge(text);
        let second = merge(&first.text);
        assert_eq!(second.merged, 0);
        assert_eq!(second.text, first.text);
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let text = "[workspace.dependencies]\r\nserde = \"1\"\r\nserde = { version = \"1\", features = [\"derive\"] }\r\n";
        let result = merge(text);
        assert!(result.text.contains("\r\n"));
        assert!(!result.text.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn merges_duplicates_after_package_and_before_dependencies() {
        let text = r#"[package]
name = "sort-demo"
version = "0.1.0"
edition = "2021"

[workspace]

[workspace.dependencies]
# serialization
serde = "1" # category = encoding
serde = { version = "1", features = ["derive"] } # keep derive
# logging
log = "0.4"

[dependencies]
serde = { workspace = true }
log = { workspace = true }
"#;
        let result = merge(text);
        assert_eq!(result.merged, 1);
        assert_eq!(result.text.matches("serde =").count(), 2);
        assert!(result.text.contains("features = [\"derive\"]"));
    }

    #[test]
    fn reports_duplicate_table_form() {
        let text = r#"[workspace.dependencies.helper]
version = "1"

[workspace.dependencies.helper]
version = "1"
"#;
        let error =
            merge_workspace_dependency_duplicates(text, Path::new("Cargo.toml")).unwrap_err();
        assert!(error.contains("duplicate `[workspace.dependencies.helper]`"));
    }
}
