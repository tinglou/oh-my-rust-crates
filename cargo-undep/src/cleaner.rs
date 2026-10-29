//! Raw-text removal of workspace dependency declarations.
//!
//! `toml_edit` is used to find line ranges, but the actual edit is performed on
//! the original string. This keeps line endings intact and makes it possible to
//! apply the project's comment rules precisely.

use crate::workspace::WorkspaceDependency;

/// Remove dependency declarations and their attached comments from `text`.
///
/// The function only removes lines that belong to the supplied dependencies.
/// Comments directly above a declaration are removed when they form a
/// contiguous block with no blank lines. A commented-out dependency such as
/// `# snmp-parser = "0.11.0"` is preserved and acts as a boundary, so neither
/// it nor comments above it are deleted.
pub fn remove_dependencies(text: &str, dependencies: &[&WorkspaceDependency]) -> String {
    if dependencies.is_empty() {
        return text.to_string();
    }

    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if lines.is_empty() {
        return text.to_string();
    }

    let mut ranges = Vec::with_capacity(dependencies.len());
    for dependency in dependencies {
        let start = dependency.start_line.min(lines.len() - 1);
        let end = dependency.end_line.min(lines.len() - 1).max(start);

        let mut first = start;
        while first > 0 {
            let previous = lines[first - 1].trim_start();
            if is_commented_dependency(previous) {
                break;
            }
            if previous.starts_with('#') {
                first -= 1;
            } else {
                break;
            }
        }

        ranges.push((first, end));
    }

    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut()
            && start <= last.1.saturating_add(1)
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }

    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end) in merged {
        for line in &lines[cursor..start] {
            output.push_str(line);
        }
        cursor = end + 1;
    }
    for line in &lines[cursor..] {
        output.push_str(line);
    }
    output
}

/// Returns whether `line` looks like a commented-out dependency declaration.
///
/// Accepted forms include:
///
/// ```toml
/// # snmp-parser = "0.11.0"
/// # foo = { version = "1.0", features = ["x"] }
/// # bar = 2
/// # "quoted-name" = "1.0"
/// ```
fn is_commented_dependency(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix('#') else {
        return false;
    };
    let rest = rest.trim();

    let Some((left, right)) = rest.split_once('=') else {
        return false;
    };

    is_dependency_key(left.trim()) && looks_like_dependency_value(right.trim())
}

fn is_dependency_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    let quoted = (key.starts_with('"') && key.ends_with('"'))
        || (key.starts_with('\'') && key.ends_with('\''));
    if quoted {
        return key.len() >= 2;
    }

    key.chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn looks_like_dependency_value(value: &str) -> bool {
    match value.chars().next() {
        Some(first) => matches!(first, '"' | '\'' | '{') || first.is_ascii_digit(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependency(name: &str, start_line: usize, end_line: usize) -> WorkspaceDependency {
        WorkspaceDependency {
            name: name.to_string(),
            start_line,
            end_line,
        }
    }

    #[test]
    fn removes_declaration_and_attached_comments() {
        let text = "[workspace.dependencies]\n\
                    # remove me\n\
                    unused-a = \"1\"\n\
                    used = \"1\"\n";
        let unused = dependency("unused-a", 2, 2);

        let output = remove_dependencies(text, &[&unused]);
        assert_eq!(output, "[workspace.dependencies]\nused = \"1\"\n");
    }

    #[test]
    fn preserves_commented_out_dependencies() {
        let text = "[workspace.dependencies]\n\
                    # ordinary comment\n\
                    # snmp-parser = \"0.11.0\"\n\
                    unused-b = { version = \"1\" }\n\
                    used = \"1\"\n";
        let unused = dependency("unused-b", 3, 3);

        let output = remove_dependencies(text, &[&unused]);
        assert_eq!(
            output,
            "[workspace.dependencies]\n\
             # ordinary comment\n\
             # snmp-parser = \"0.11.0\"\n\
             used = \"1\"\n"
        );
    }

    #[test]
    fn preserves_commented_inline_table_dependency() {
        let text = "[workspace.dependencies]\n\
                    # foo = { version = \"1.0\", features = [\"x\"] }\n\
                    unused = \"1\"\n";
        let unused = dependency("unused", 2, 2);

        let output = remove_dependencies(text, &[&unused]);
        assert_eq!(
            output,
            "[workspace.dependencies]\n\
             # foo = { version = \"1.0\", features = [\"x\"] }\n"
        );
    }

    #[test]
    fn does_not_treat_plain_prose_as_commented_dependency() {
        assert!(!is_commented_dependency("# note = remember this"));
        assert!(!is_commented_dependency("# see the docs = later"));
        assert!(is_commented_dependency("# snmp-parser = \"0.11.0\""));
        assert!(is_commented_dependency("# bar = 2"));
        assert!(is_commented_dependency("# foo = { version = \"1\" }"));
        assert!(is_commented_dependency("# \"quoted-name\" = \"1.0\""));
    }

    #[test]
    fn removes_multiple_dependencies_and_merges_ranges() {
        let text = "[workspace.dependencies]\n\
                    # first\n\
                    a = \"1\"\n\
                    # second\n\
                    b = \"1\"\n\
                    used = \"1\"\n";
        let a = dependency("a", 2, 2);
        let b = dependency("b", 4, 4);

        let output = remove_dependencies(text, &[&a, &b]);
        assert_eq!(output, "[workspace.dependencies]\nused = \"1\"\n");
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let text = "[workspace.dependencies]\r\n\
                    # remove me\r\n\
                    unused = \"1\"\r\n\
                    used = \"1\"\r\n";
        let unused = dependency("unused", 2, 2);

        let output = remove_dependencies(text, &[&unused]);
        assert_eq!(output, "[workspace.dependencies]\r\nused = \"1\"\r\n");
    }

    #[test]
    fn removes_multiline_declaration() {
        let text = "[workspace.dependencies]\n\
                    unused = {\n\
                        version = \"1\",\n\
                        features = [\"a\"],\n\
                    }\n\
                    used = \"1\"\n";
        let unused = dependency("unused", 1, 4);

        let output = remove_dependencies(text, &[&unused]);
        assert_eq!(output, "[workspace.dependencies]\nused = \"1\"\n");
    }
}
