/// ─── GrepTool ──────────────────────────────────────────────────────────────
///
/// Search source code with regex patterns or AST-based symbol queries.
/// Respects .gitignore.  Returns structured JSON results.

use super::Tool;
use ignore::WalkBuilder;
use regex::Regex;
use std::path::Path;

pub struct Grep;

impl Tool for Grep {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        r#"Search code with regex or find symbols via AST.
Text mode: {"pattern":"fn main","path":"src/","max_matches":50}
AST mode:  {"ast_query":"function:main","path":"src/","language":"rust"}
Supported AST queries: function:<name>, struct:<name>, trait:<name>,
impl:<name>, enum:<name>, class:<name>, type:<name>, const:<name>"#
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct GrepInput {
            #[serde(default)]
            pattern: String,
            #[serde(default)]
            ast_query: String,
            #[serde(default = "default_path")]
            path: String,
            #[serde(default = "default_language")]
            language: String,
            #[serde(default = "default_max_matches")]
            max_matches: usize,
        }

        fn default_path() -> String { ".".into() }
        fn default_language() -> String { "rust".into() }
        fn default_max_matches() -> usize { 50 }

        let parsed: GrepInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid grep input: {e}"))?;

        if !parsed.ast_query.is_empty() {
            return Self::ast_search(&parsed.ast_query, &parsed.path, &parsed.language, parsed.max_matches);
        }
        if !parsed.pattern.is_empty() {
            return Self::regex_search(&parsed.pattern, &parsed.path, parsed.max_matches);
        }

        anyhow::bail!("grep requires either \"pattern\" (text mode) or \"ast_query\" (AST mode)")
    }
}

impl Grep {
    /// ─── Text / Regex search ──────────────────────────────────────────────

    fn regex_search(pattern: &str, base_path: &str, max_matches: usize) -> anyhow::Result<String> {
        let re = Regex::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {e}", pattern))?;

        let walker = WalkBuilder::new(base_path)
            .standard_filters(true)
            .build();

        let mut results: Vec<GrepMatch> = Vec::new();

        for entry in walker {
            if results.len() >= max_matches {
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();

            if !is_text_file(path) {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let rel_path = relative_path(path, base_path);

            for (line_num, line_content) in content.lines().enumerate() {
                if results.len() >= max_matches {
                    break;
                }
                if re.is_match(line_content) {
                    results.push(GrepMatch {
                        file: rel_path.clone(),
                        line_number: line_num + 1,
                        column: None,
                        line_content: line_content.to_string(),
                        match_type: "text".into(),
                    });
                }
            }
        }

        #[derive(serde::Serialize)]
        struct GrepResult {
            matches: Vec<GrepMatch>,
            total: usize,
            truncated: bool,
        }

        let total = results.len();
        let truncated = total >= max_matches;

        Ok(serde_json::to_string(&GrepResult {
            matches: results,
            total,
            truncated,
        })?)
    }

    /// ─── AST-based symbol search ──────────────────────────────────────────

    fn ast_search(
        query: &str,
        base_path: &str,
        language: &str,
        max_matches: usize,
    ) -> anyhow::Result<String> {
        let (kind, name) = parse_ast_query(query)?;

        let mut parser = tree_sitter::Parser::new();
        let grammar = load_grammar(language);
        parser.set_language(&grammar)
            .map_err(|e| anyhow::anyhow!("Failed to set tree-sitter language '{language}': {e}"))?;

        let extensions = file_extensions_for_language(language);

        let walker = WalkBuilder::new(base_path)
            .standard_filters(true)
            .build();

        let mut results: Vec<GrepMatch> = Vec::new();

        for entry in walker {
            if results.len() >= max_matches {
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !extensions.iter().any(|e| *e == ext) {
                    continue;
                }
            } else {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let tree = match parser.parse(&content, None) {
                Some(t) => t,
                None => continue,
            };

            let rel_path = relative_path(path, base_path);
            let root = tree.root_node();
            let matched = find_ast_matches(&kind, &name, &content, &root);

            for m in matched {
                if results.len() >= max_matches {
                    break;
                }
                results.push(GrepMatch {
                    file: rel_path.clone(),
                    line_number: m.line_number,
                    column: m.column,
                    line_content: m.line_content,
                    match_type: m.match_type,
                });
            }
        }

        #[derive(serde::Serialize)]
        struct GrepResult {
            matches: Vec<GrepMatch>,
            total: usize,
            truncated: bool,
        }

        let total = results.len();
        let truncated = total >= max_matches;

        Ok(serde_json::to_string(&GrepResult {
            matches: results,
            total,
            truncated,
        })?)
    }
}

/// ─── Data types ────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct GrepMatch {
    file: String,
    line_number: usize,
    column: Option<usize>,
    line_content: String,
    match_type: String,
}

/// ─── Helpers ───────────────────────────────────────────────────────────────

fn relative_path(path: &Path, base_path: &str) -> String {
    if let Ok(rel) = path.strip_prefix(base_path) {
        rel.to_string_lossy().to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn is_text_file(path: &Path) -> bool {
    let text_extensions = [
        "rs", "go", "py", "js", "ts", "tsx", "jsx", "rb", "java", "c", "h",
        "cpp", "hpp", "swift", "kt", "scala", "r", "sh", "bash",
        "toml", "json", "yaml", "yml", "md", "txt", "xml", "html", "css",
        "scss", "less", "sql", "lua", "zig", "ex", "exs", "php", "pl",
        "lisp", "clj", "dart", "nim", "v", "vue", "svelte", "astro", "mjs",
        "cjs", "mts", "cts",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| text_extensions.contains(&e))
        .unwrap_or(false)
}

/// ─── AST query parsing ─────────────────────────────────────────────────────

fn parse_ast_query(query: &str) -> anyhow::Result<(&str, &str)> {
    let colon_pos = query.find(':')
        .ok_or_else(|| anyhow::anyhow!("Invalid AST query '{query}'. Expected format: 'kind:name' (e.g. 'function:main')"))?;
    let kind = &query[..colon_pos];
    let name = &query[colon_pos + 1..];
    if kind.is_empty() || name.is_empty() {
        anyhow::bail!("Invalid AST query '{query}': both kind and name must be non-empty");
    }
    Ok((kind, name))
}

/// ─── File extensions for a given language ──────────────────────────────────

fn file_extensions_for_language(language: &str) -> Vec<&'static str> {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => vec!["rs"],
        "typescript" | "ts" => vec!["ts", "tsx", "mts", "cts"],
        "javascript" | "js" => vec!["js", "jsx", "mjs", "cjs"],
        "python" | "py" => vec!["py"],
        "go" => vec!["go"],
        _ => vec!["rs", "ts", "tsx", "js", "jsx", "py", "go"],
    }
}

/// ─── Grammar loading ───────────────────────────────────────────────────────

fn load_grammar(language: &str) -> tree_sitter::Language {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => tree_sitter_rust::LANGUAGE.into(),
        "typescript" | "ts" => {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        }
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "python" | "py" => tree_sitter_python::LANGUAGE.into(),
        _ => tree_sitter_rust::LANGUAGE.into(),
    }
}

/// ─── AST node matching ─────────────────────────────────────────────────────

struct AstMatch {
    line_number: usize,
    column: Option<usize>,
    line_content: String,
    match_type: String,
}

fn find_ast_matches(
    kind: &str,
    name: &str,
    source: &str,
    node: &tree_sitter::Node,
) -> Vec<AstMatch> {
    let mut results = Vec::new();

    let node_types = ast_node_types(kind);

    if node_types.iter().any(|nt| *nt == node.kind()) {
        if let Some(matched_name) = extract_node_name(node, source) {
            if name == "*" || matched_name == name {
                let start_pos = node.start_position();
                results.push(AstMatch {
                    line_number: start_pos.row + 1,
                    column: Some(start_pos.column + 1),
                    line_content: extract_line(source, start_pos.row).to_string(),
                    match_type: kind.to_string(),
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        results.extend(find_ast_matches(kind, name, source, &child));
    }

    results
}

/// Map user-friendly kind names to tree-sitter node types.
fn ast_node_types(kind: &str) -> Vec<&'static str> {
    match kind {
        "function" | "fn" | "func" => vec![
            "function_item",       // Rust
            "function_definition", // Rust macro, Python
            "method_definition",   // Python
            "function_declaration", // Go
            "function",            // TypeScript/JS
        ],
        "struct" => vec![
            "struct_item",       // Rust
        ],
        "trait" => vec![
            "trait_item", // Rust
        ],
        "impl" => vec![
            "impl_item", // Rust
        ],
        "enum" => vec![
            "enum_item",        // Rust
            "enum_declaration", // TypeScript
        ],
        "class" => vec![
            "class_declaration", // TypeScript/JS
        ],
        "type" | "typedef" => vec![
            "type_item",       // Rust
            "type_alias",      // Rust
            "type_alias_declaration", // TypeScript
        ],
        "const" => vec![
            "const_item",         // Rust
            "const_declaration",  // TypeScript
            "lexical_declaration", // TypeScript (const/let)
        ],
        "macro" => vec![
            "macro_definition", // Rust
        ],
        "module" | "mod" => vec![
            "module", // Rust
        ],
        _ => vec![],
    }
}

/// Extract the name of a definition node.
fn extract_node_name<'a>(node: &tree_sitter::Node, source: &'a str) -> Option<&'a str> {
    // Check named "name" child first
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(text) = name_node.utf8_text(source.as_bytes()) {
            return Some(text);
        }
    }
    // Fallback: look for child named "identifier"
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return child.utf8_text(source.as_bytes()).ok();
        }
    }
    None
}

/// Extract a line from source by 0-based row index.
fn extract_line<'a>(source: &'a str, row: usize) -> &'a str {
    source.lines().nth(row).unwrap_or("")
}
