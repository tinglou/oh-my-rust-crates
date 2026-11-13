//! Shared text helpers for line endings and the category-comment convention.
//!
//! `sort` writes `# --- <category> ---` header comments and `# category = <name>`
//! annotations, while `manifest` understands the same convention when it merges
//! duplicate `[workspace.dependencies]` entries. Keeping the parsing in one
//! place stops the two modules from drifting apart.

/// Prefix of a category header comment, e.g. `# --- misc ---`.
pub(crate) const CATEGORY_COMMENT_PREFIX: &str = "# --- ";
/// Suffix of a category header comment, e.g. `# --- misc ---`.
pub(crate) const CATEGORY_COMMENT_SUFFIX: &str = " ---";

/// Restore the line endings of `original` on rewritten `contents`.
///
/// `toml_edit` always renders `\n`; this re-applies `\r\n` when the source file
/// used CRLF so rewrites do not churn every line.
pub(crate) fn restore_line_endings(original: &str, contents: String) -> String {
    if original.contains("\r\n") {
        contents.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        contents
    }
}

/// Returns `true` if `line` is a category header comment like `# --- misc ---`.
pub(crate) fn is_category_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with(CATEGORY_COMMENT_PREFIX) && trimmed.ends_with(CATEGORY_COMMENT_SUFFIX)
}

/// Parse a `# category = <name>` annotation from a comment.
///
/// The annotation may appear on any line of `comment` after an inline `#`. The
/// returned category is normalized, i.e. the part before any `::`.
pub(crate) fn category_from_comment(comment: &str) -> Option<String> {
    for line in comment.lines() {
        let Some((_, comment)) = line.split_once('#') else {
            continue;
        };
        let comment = comment.trim();
        let Some(after_category) = comment.strip_prefix("category") else {
            continue;
        };
        let after_category = after_category.trim();
        let Some(value) = after_category.strip_prefix('=') else {
            continue;
        };

        let value = value.split_whitespace().next().unwrap_or_default();
        let value = value.trim_matches(['"', '\'', ',']);
        if let Some(category) = normalize_category(value) {
            return Some(category);
        }
    }

    None
}

/// Returns `true` if `comment` contains a `# category = <name>` annotation.
pub(crate) fn has_category_comment(comment: &str) -> bool {
    category_from_comment(comment).is_some()
}

/// Normalize a category name to the part before `::`.
pub(crate) fn normalize_category(category: &str) -> Option<String> {
    let category = category.split("::").next().unwrap_or(category).trim();
    (!category.is_empty()).then(|| category.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_category_headers() {
        assert!(is_category_header("# --- misc ---"));
        assert!(is_category_header("  # --- path ---  "));
        assert!(!is_category_header("# --- misc"));
        assert!(!is_category_header("misc"));
    }

    #[test]
    fn parses_category_annotations_strictly() {
        assert_eq!(
            category_from_comment("serde = \"1\" # category = encoding").as_deref(),
            Some("encoding")
        );
        assert_eq!(
            category_from_comment("# category=development-tools::profiling").as_deref(),
            Some("development-tools")
        );
        assert_eq!(
            category_from_comment("# category = \"network-programming\"").as_deref(),
            Some("network-programming")
        );
        // Must require the `=` so prose like `# categorization` is not matched.
        assert_eq!(category_from_comment("# categorization is hard"), None);
        assert_eq!(category_from_comment("# category encoding"), None);
    }

    #[test]
    fn has_category_comment_matches_only_real_annotations() {
        assert!(has_category_comment(" # category = encoding"));
        assert!(!has_category_comment(" # categorize = encoding"));
        assert!(!has_category_comment(" # ordinary comment"));
    }

    #[test]
    fn normalizes_category_prefix() {
        assert_eq!(
            normalize_category("development-tools::profiling").as_deref(),
            Some("development-tools")
        );
        assert_eq!(normalize_category("  ").as_deref(), None);
    }

    #[test]
    fn restores_crlf_line_endings() {
        let original = "a\r\nb\r\n";
        let rewritten = "a\nb\nc\n".to_string();
        assert_eq!(restore_line_endings(original, rewritten), "a\r\nb\r\nc\r\n");
        assert_eq!(
            restore_line_endings("a\nb\n", "a\nb\n".to_string()),
            "a\nb\n"
        );
    }
}
