//! Rust source parser using tree-sitter

use std::collections::HashMap;
use tree_sitter::Parser;

/// Parsed plugin information
#[derive(Debug, Default)]
pub struct ParsedPlugin {
    /// Module doc comments (`//!`)
    pub doc_comments: Vec<String>,
    /// Use statements with optional version comments
    /// Key: crate name, Value: (version, optional features)
    pub dependencies: HashMap<String, (String, Option<String>)>,
    /// Function names marked with `#[php_function]`
    pub php_functions: Vec<String>,
    /// Whether the source already has `#[php_module]`
    pub has_php_module: bool,
    /// Whether the source already has `micro_plugin_info`
    pub has_plugin_boilerplate: bool,
    /// Whether the source has `use ext_php_rs::prelude::*`
    pub has_prelude: bool,
}

/// Standard library crates that don't need dependencies
const STD_CRATES: &[&str] = &[
    "std",
    "core",
    "alloc",
    "ext_php_rs",
    "self",
    "super",
    "crate",
];

/// Parse a Rust source file and extract plugin information
///
/// If `auto_export_functions` is true, all top-level functions are treated as PHP functions
/// (useful for `eval()` where users expect all functions to be exported).
/// If false, only functions with `#[php_function]` are exported (for file-based loading).
pub fn parse_plugin(source: &str, auto_export_functions: bool) -> ParsedPlugin {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading Rust grammar");

    let Some(tree) = parser.parse(source, None) else {
        return ParsedPlugin::default();
    };

    let mut result = ParsedPlugin::default();
    let root = tree.root_node();

    // Check for existing constructs
    result.has_php_module = source.contains("#[php_module]");
    result.has_plugin_boilerplate = source.contains("micro_plugin_info");
    result.has_prelude = source.contains("use ext_php_rs::prelude::*");

    // Walk the tree to find use declarations
    let mut cursor = root.walk();

    // Collect use declarations
    for node in root.children(&mut cursor) {
        if node.kind() == "use_declaration" {
            if let Some(crate_name) = extract_crate_from_node(&node, source) {
                if !STD_CRATES.contains(&crate_name.as_str()) {
                    // Get the line to check for version comment
                    let start = node.start_position().row;
                    let line = source.lines().nth(start).unwrap_or("");

                    let (version, features, pkg_override) =
                        if let Some(comment_start) = line.find("//") {
                            let comment = line[comment_start + 2..].trim();
                            if !comment.is_empty()
                                && comment
                                    .chars()
                                    .next()
                                    .is_some_and(|c: char| c.is_ascii_digit())
                            {
                                parse_dep_comment(comment)
                            } else {
                                ("*".to_string(), None, None)
                            }
                        } else {
                            ("*".to_string(), None, None)
                        };

                    // Use explicit package name if provided, otherwise use the identifier
                    let package_name = pkg_override.unwrap_or(crate_name);
                    result
                        .dependencies
                        .insert(package_name, (version, features));
                }
            }
        }
    }

    // Simple line-based parsing for #[php_function] (more reliable)
    let mut next_is_php_function = false;
    for line in source.lines() {
        let trimmed = line.trim();

        // Collect doc comments at the start
        if trimmed.starts_with("//!") {
            if result
                .doc_comments
                .last()
                .is_none_or(|l| l.trim().starts_with("//!") || l.trim().is_empty())
            {
                result.doc_comments.push(line.to_string());
                continue;
            }
        } else if trimmed.is_empty()
            && result
                .doc_comments
                .iter()
                .all(|l| l.trim().starts_with("//!") || l.trim().is_empty())
            && !result.doc_comments.is_empty()
        {
            result.doc_comments.push(line.to_string());
            continue;
        }

        if trimmed == "#[php_function]" || trimmed.starts_with("#[php_function(") {
            next_is_php_function = true;
        } else if next_is_php_function {
            // Skip over any additional attributes (like #[php(name = "...")])
            if trimmed.starts_with("#[") {
                // Keep waiting for the fn line
                continue;
            }
            if let Some(fn_name) = extract_fn_name(trimmed) {
                if !result.php_functions.contains(&fn_name) {
                    result.php_functions.push(fn_name);
                }
            }
            next_is_php_function = false;
        } else if auto_export_functions {
            // Auto-export mode: treat all top-level functions as PHP functions
            if let Some(fn_name) = extract_fn_name(trimmed) {
                if !result.php_functions.contains(&fn_name) {
                    result.php_functions.push(fn_name);
                }
            }
        }
    }

    result
}

/// Extract crate name from a `use_declaration` node
fn extract_crate_from_node(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "scoped_identifier" | "scoped_use_list" => {
                // Get the first identifier (crate name)
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "identifier" {
                        return Some(
                            source[inner_child.start_byte()..inner_child.end_byte()].to_string(),
                        );
                    }
                }
            }
            "identifier" => {
                return Some(source[child.start_byte()..child.end_byte()].to_string());
            }
            _ => {}
        }
    }

    None
}

/// Parse dependency comment
///
/// Formats:
/// - `1.0` → version only
/// - `1.0, features = ["derive"]` → version with features
/// - `0.23 as tree-sitter` → version with explicit package name
/// - `1.0, features = ["derive"] as my-crate` → all three
/// - `path = "../my-crate"` → local path dependency
/// - `git = "https://...", branch = "main"` → git dependency
///
/// Returns: (`version_or_spec`, features, `package_name_override`)
/// For path/git deps, `version_or_spec` contains the full spec (e.g., `path = "..."`).
fn parse_dep_comment(comment: &str) -> (String, Option<String>, Option<String>) {
    let comment = comment.trim();

    // Check for path or git dependencies (full spec mode)
    if comment.starts_with("path =") || comment.starts_with("git =") {
        // Check for "as package-name" at the end
        if let Some(as_pos) = comment.rfind(" as ") {
            let pkg = comment[as_pos + 4..].trim().to_string();
            let spec = comment[..as_pos].trim().to_string();
            return (spec, None, Some(pkg));
        }
        return (comment.to_string(), None, None);
    }

    // Check for "as package-name" at the end
    let (main_part, package_override) = if let Some(as_pos) = comment.rfind(" as ") {
        let pkg = comment[as_pos + 4..].trim().to_string();
        let main = comment[..as_pos].trim();
        (main, Some(pkg))
    } else {
        (comment, None)
    };

    // Parse version and features from main part
    if let Some(comma_pos) = main_part.find(',') {
        let version = main_part[..comma_pos].trim().to_string();
        let features = main_part[comma_pos + 1..].trim().to_string();
        (version, Some(features), package_override)
    } else {
        (main_part.to_string(), None, package_override)
    }
}

/// Extract function name from a line like "fn foo(" or "pub fn foo("
fn extract_fn_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let after_fn = if trimmed.starts_with("pub fn ") {
        trimmed.strip_prefix("pub fn ")
    } else if trimmed.starts_with("fn ") {
        trimmed.strip_prefix("fn ")
    } else {
        None
    }?;

    after_fn.split('(').next().map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_plugin() {
        let source = r#"
//! Test plugin

use serde::Serialize; // 1.0, features = ["derive"]

#[php_function]
fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

#[php_function]
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}
"#;

        let parsed = parse_plugin(source, false);
        assert!(parsed.dependencies.contains_key("serde"));
        assert_eq!(parsed.dependencies["serde"].0, "1.0");
        assert!(parsed.php_functions.contains(&"hello".to_string()));
        assert!(parsed.php_functions.contains(&"add".to_string()));
        assert!(!parsed.has_php_module);
    }

    #[test]
    fn test_hyphenated_crate_name() {
        let source = r"
use tree_sitter::Parser; // 0.23 as tree-sitter
use some_crate::Thing; // 1.0
";

        let parsed = parse_plugin(source, false);
        // Should use explicit package name "tree-sitter"
        assert!(parsed.dependencies.contains_key("tree-sitter"));
        assert_eq!(parsed.dependencies["tree-sitter"].0, "0.23");
        // Should use identifier as-is when no override
        assert!(parsed.dependencies.contains_key("some_crate"));
        assert_eq!(parsed.dependencies["some_crate"].0, "1.0");
    }

    #[test]
    fn test_parse_dep_comment() {
        // Simple version
        assert_eq!(parse_dep_comment("1.0"), ("1.0".into(), None, None));

        // Version with features
        let (v, f, p) = parse_dep_comment("1.0, features = [\"derive\"]");
        assert_eq!(v, "1.0");
        assert_eq!(f, Some("features = [\"derive\"]".into()));
        assert_eq!(p, None);

        // Version with package override
        let (v, f, p) = parse_dep_comment("0.23 as tree-sitter");
        assert_eq!(v, "0.23");
        assert_eq!(f, None);
        assert_eq!(p, Some("tree-sitter".into()));

        // All three
        let (v, f, p) = parse_dep_comment("1.0, features = [\"derive\"] as my-crate");
        assert_eq!(v, "1.0");
        assert_eq!(f, Some("features = [\"derive\"]".into()));
        assert_eq!(p, Some("my-crate".into()));

        // Path dependency
        let (spec, f, p) = parse_dep_comment("path = \"../my-crate\"");
        assert_eq!(spec, "path = \"../my-crate\"");
        assert_eq!(f, None);
        assert_eq!(p, None);

        // Path dependency with package override
        let (spec, f, p) = parse_dep_comment("path = \"../my_crate\" as my-crate");
        assert_eq!(spec, "path = \"../my_crate\"");
        assert_eq!(f, None);
        assert_eq!(p, Some("my-crate".into()));

        // Git dependency
        let (spec, f, p) = parse_dep_comment("git = \"https://github.com/user/repo\"");
        assert_eq!(spec, "git = \"https://github.com/user/repo\"");
        assert_eq!(f, None);
        assert_eq!(p, None);

        // Git dependency with branch
        let (spec, f, p) =
            parse_dep_comment("git = \"https://github.com/user/repo\", branch = \"main\"");
        assert_eq!(
            spec,
            "git = \"https://github.com/user/repo\", branch = \"main\""
        );
        assert_eq!(f, None);
        assert_eq!(p, None);

        // Git dependency with tag
        let (spec, f, p) =
            parse_dep_comment("git = \"https://github.com/user/repo\", tag = \"v1.0.0\"");
        assert_eq!(
            spec,
            "git = \"https://github.com/user/repo\", tag = \"v1.0.0\""
        );
        assert_eq!(f, None);
        assert_eq!(p, None);

        // Git dependency with package override
        let (spec, f, p) = parse_dep_comment(
            "git = \"https://github.com/user/repo\", branch = \"main\" as my-crate",
        );
        assert_eq!(
            spec,
            "git = \"https://github.com/user/repo\", branch = \"main\""
        );
        assert_eq!(f, None);
        assert_eq!(p, Some("my-crate".into()));
    }

    #[test]
    fn test_auto_export_functions() {
        let source = r#"
fn plain_function(x: i64) -> i64 {
    x * 2
}

#[php_function]
fn marked_function() -> String {
    "hello".to_string()
}

pub fn another_plain(name: String) -> String {
    format!("Hi, {}", name)
}
"#;

        // Without auto_export, only marked function is found
        let parsed = parse_plugin(source, false);
        assert_eq!(parsed.php_functions.len(), 1);
        assert!(parsed
            .php_functions
            .contains(&"marked_function".to_string()));

        // With auto_export, all functions are found
        let parsed = parse_plugin(source, true);
        assert_eq!(parsed.php_functions.len(), 3);
        assert!(parsed.php_functions.contains(&"plain_function".to_string()));
        assert!(parsed
            .php_functions
            .contains(&"marked_function".to_string()));
        assert!(parsed.php_functions.contains(&"another_plain".to_string()));
    }
}
