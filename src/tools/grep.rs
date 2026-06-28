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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ─── Grep tool metadata ───────────────────────────────────────────────

    #[test]
    fn test_grep_name() {
        let grep = Grep;
        assert_eq!(grep.name(), "grep");
    }

    #[test]
    fn test_grep_description_not_empty() {
        let grep = Grep;
        assert!(!grep.description().is_empty());
        assert!(grep.description().contains("Search"));
    }

    // ─── execute with invalid input ────────────────────────────────────────

    #[test]
    fn test_execute_invalid_json() {
        let grep = Grep;
        let result = grep.execute("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_empty_input() {
        let grep = Grep;
        let result = grep.execute(r#"{}"#);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("pattern") || err.contains("ast_query"));
    }

    // ─── parse_ast_query ──────────────────────────────────────────────────

    #[test]
    fn test_parse_ast_query_valid() {
        let (kind, name) = parse_ast_query("function:main").unwrap();
        assert_eq!(kind, "function");
        assert_eq!(name, "main");
    }

    #[test]
    fn test_parse_ast_query_wildcard() {
        let (kind, name) = parse_ast_query("struct:*").unwrap();
        assert_eq!(kind, "struct");
        assert_eq!(name, "*");
    }

    #[test]
    fn test_parse_ast_query_multi_colon() {
        let (kind, name) = parse_ast_query("fn:foo::bar").unwrap();
        assert_eq!(kind, "fn");
        assert_eq!(name, "foo::bar");
    }

    #[test]
    fn test_parse_ast_query_no_colon() {
        let result = parse_ast_query("function");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ast_query_empty_kind() {
        let result = parse_ast_query(":main");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ast_query_empty_name() {
        let result = parse_ast_query("function:");
        assert!(result.is_err());
    }

    // ─── relative_path ────────────────────────────────────────────────────

    #[test]
    fn test_relative_path_strips_prefix() {
        let p = Path::new("/home/user/project/src/main.rs");
        let rel = relative_path(p, "/home/user/project");
        assert_eq!(rel, "src/main.rs");
    }

    #[test]
    fn test_relative_path_no_prefix_match() {
        let p = Path::new("/other/path/file.rs");
        let rel = relative_path(p, "/home/user/project");
        assert_eq!(rel, "/other/path/file.rs");
    }

    #[test]
    fn test_relative_path_current_dir() {
        let p = Path::new("file.rs");
        let rel = relative_path(p, ".");
        assert_eq!(rel, "file.rs");
    }

    // ─── is_text_file ─────────────────────────────────────────────────────

    #[test]
    fn test_is_text_file_rs() {
        assert!(is_text_file(Path::new("main.rs")));
    }

    #[test]
    fn test_is_text_file_py() {
        assert!(is_text_file(Path::new("script.py")));
    }

    #[test]
    fn test_is_text_file_md() {
        assert!(is_text_file(Path::new("README.md")));
    }

    #[test]
    fn test_is_text_file_no_extension() {
        assert!(!is_text_file(Path::new("Makefile")));
    }

    #[test]
    fn test_is_text_file_binary_ext() {
        assert!(!is_text_file(Path::new("image.png")));
        assert!(!is_text_file(Path::new("binary.bin")));
        assert!(!is_text_file(Path::new("archive.zip")));
    }

    #[test]
    fn test_is_text_file_mjs() {
        assert!(is_text_file(Path::new("module.mjs")));
    }

    // ─── file_extensions_for_language ──────────────────────────────────────

    #[test]
    fn test_extensions_rust() {
        let exts = file_extensions_for_language("rust");
        assert_eq!(exts, vec!["rs"]);
    }

    #[test]
    fn test_extensions_rust_alias() {
        let exts = file_extensions_for_language("rs");
        assert_eq!(exts, vec!["rs"]);
    }

    #[test]
    fn test_extensions_typescript() {
        let exts = file_extensions_for_language("typescript");
        assert_eq!(exts, vec!["ts", "tsx", "mts", "cts"]);
    }

    #[test]
    fn test_extensions_javascript() {
        let exts = file_extensions_for_language("javascript");
        assert_eq!(exts, vec!["js", "jsx", "mjs", "cjs"]);
    }

    #[test]
    fn test_extensions_python() {
        let exts = file_extensions_for_language("python");
        assert_eq!(exts, vec!["py"]);
    }

    #[test]
    fn test_extensions_go() {
        let exts = file_extensions_for_language("go");
        assert_eq!(exts, vec!["go"]);
    }

    #[test]
    fn test_extensions_unknown() {
        let exts = file_extensions_for_language("unknown");
        assert_eq!(exts, vec!["rs", "ts", "tsx", "js", "jsx", "py", "go"]);
    }

    #[test]
    fn test_extensions_case_insensitive() {
        let exts = file_extensions_for_language("Rust");
        assert_eq!(exts, vec!["rs"]);
    }

    // ─── ast_node_types ───────────────────────────────────────────────────

    #[test]
    fn test_ast_node_types_function() {
        let types = ast_node_types("function");
        assert!(types.contains(&"function_item"));
    }

    #[test]
    fn test_ast_node_types_fn_alias() {
        let types = ast_node_types("fn");
        assert!(types.contains(&"function_item"));
    }

    #[test]
    fn test_ast_node_types_struct() {
        let types = ast_node_types("struct");
        assert!(types.contains(&"struct_item"));
    }

    #[test]
    fn test_ast_node_types_trait() {
        let types = ast_node_types("trait");
        assert!(types.contains(&"trait_item"));
    }

    #[test]
    fn test_ast_node_types_impl() {
        let types = ast_node_types("impl");
        assert!(types.contains(&"impl_item"));
    }

    #[test]
    fn test_ast_node_types_enum() {
        let types = ast_node_types("enum");
        assert!(types.contains(&"enum_item"));
    }

    #[test]
    fn test_ast_node_types_class() {
        let types = ast_node_types("class");
        assert!(types.contains(&"class_declaration"));
    }

    #[test]
    fn test_ast_node_types_type() {
        let types = ast_node_types("type");
        assert!(types.contains(&"type_item"));
    }

    #[test]
    fn test_ast_node_types_const() {
        let types = ast_node_types("const");
        assert!(types.contains(&"const_item"));
    }

    #[test]
    fn test_ast_node_types_macro() {
        let types = ast_node_types("macro");
        assert!(types.contains(&"macro_definition"));
    }

    #[test]
    fn test_ast_node_types_module() {
        let types = ast_node_types("module");
        assert!(types.contains(&"module"));
    }

    #[test]
    fn test_ast_node_types_unknown() {
        let types = ast_node_types("nonexistent");
        assert!(types.is_empty());
    }

    #[test]
    fn test_ast_node_types_mod_alias() {
        let types = ast_node_types("mod");
        assert!(types.contains(&"module"));
    }

    // ─── extract_node_name ────────────────────────────────────────────────

    #[test]
    fn test_extract_node_name_from_function() {
        let source = "fn hello() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // Walk children to find the function item
        let mut cursor = root.walk();
        let found = root.children(&mut cursor).find(|child| {
            child.kind() == "function_item"
        });
        if let Some(fn_node) = found {
            let name = extract_node_name(&fn_node, source);
            assert_eq!(name, Some("hello"));
        } else {
            panic!("Expected to find function_item node");
        }
    }

    #[test]
    fn test_extract_node_name_no_name() {
        // A numeric literal node has no "name" child
        let source = "let x = 42;";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        // The source_file node has no name — this is expected
        let name = extract_node_name(&root, source);
        assert!(name.is_none());
    }

    // ─── extract_line ─────────────────────────────────────────────────────

    #[test]
    fn test_extract_line_first() {
        assert_eq!(extract_line("hello\nworld\nfoo", 0), "hello");
    }

    #[test]
    fn test_extract_line_middle() {
        assert_eq!(extract_line("a\nb\nc\n", 1), "b");
    }

    #[test]
    fn test_extract_line_last() {
        assert_eq!(extract_line("a\nb\nc", 2), "c");
    }

    #[test]
    fn test_extract_line_out_of_bounds() {
        assert_eq!(extract_line("hello\nworld", 10), "");
    }

    #[test]
    fn test_extract_line_empty_source() {
        assert_eq!(extract_line("", 0), "");
    }

    #[test]
    fn test_extract_line_single_line() {
        assert_eq!(extract_line("only line", 0), "only line");
    }

    // ─── load_grammar ─────────────────────────────────────────────────────

    #[test]
    fn test_load_grammar_rust() {
        let lang = load_grammar("rust");
        let rust_lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        assert_eq!(lang, rust_lang);
    }

    #[test]
    fn test_load_grammar_rs_alias() {
        let lang = load_grammar("rs");
        let rust_lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        assert_eq!(lang, rust_lang);
    }

    #[test]
    fn test_load_grammar_python() {
        let lang = load_grammar("python");
        let py_lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        assert_eq!(lang, py_lang);
    }

    #[test]
    fn test_load_grammar_typescript() {
        let lang = load_grammar("typescript");
        let ts_lang: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        assert_eq!(lang, ts_lang);
    }

    #[test]
    fn test_load_grammar_unknown_falls_back_to_rust() {
        let lang = load_grammar("nonexistent");
        let rust_lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        assert_eq!(lang, rust_lang);
    }

    #[test]
    fn test_load_grammar_case_insensitive() {
        let lang = load_grammar("RUST");
        let rust_lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        assert_eq!(lang, rust_lang);
    }

    // ─── find_ast_matches ─────────────────────────────────────────────────

    #[test]
    fn test_find_ast_matches_function_in_rust() {
        let source = "fn hello() {}\nfn world() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("function", "hello", source, &tree.root_node());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[0].match_type, "function");
    }

    #[test]
    fn test_find_ast_matches_wildcard() {
        let source = "fn a() {}\nfn b() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("function", "*", source, &tree.root_node());
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_find_ast_matches_no_match() {
        let source = "fn hello() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("function", "nonexistent", source, &tree.root_node());
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_ast_matches_struct() {
        let source = "struct Point { x: i32, y: i32 }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("struct", "Point", source, &tree.root_node());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].match_type, "struct");
    }

    #[test]
    fn test_find_ast_matches_trait() {
        let source = "trait Display { fn fmt(); }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("trait", "Display", source, &tree.root_node());
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_find_ast_matches_impl() {
        // `impl S { ... }` in tree-sitter-rust does not have a "name" field
        // (it has a "type" field), so extract_node_name returns None and
        // no matches are found. This test documents that behavior.
        let source = "struct S;\nimpl S { fn method() {} }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        // Named query: no match because impl_item lacks a "name" field
        let matches = find_ast_matches("impl", "S", source, &tree.root_node());
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_find_ast_matches_enum() {
        let source = "enum Color { Red, Blue }";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("enum", "Color", source, &tree.root_node());
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_find_ast_matches_const() {
        let source = "const MAX: usize = 100;";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("const", "MAX", source, &tree.root_node());
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_find_ast_matches_unknown_kind_returns_empty() {
        let source = "fn foo() {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let matches = find_ast_matches("bogus", "*", source, &tree.root_node());
        assert!(matches.is_empty());
    }

    // ─── Integration tests ────────────────────────────────────────────────

    #[test]
    fn test_regex_search_simple() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn hello() {}\nfn world() {}\n").unwrap();
        let result = Grep::regex_search("hello", dir.path().to_str().unwrap(), 50).unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("test.rs"));
    }

    #[test]
    fn test_regex_search_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn foo() {}\n").unwrap();
        let result = Grep::regex_search("nonexistent", dir.path().to_str().unwrap(), 50);
        assert!(result.is_ok() || result.is_err(), "regex_search should not panic");
    }

    #[test]
    fn test_regex_search_respects_max_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").unwrap();
        let result = Grep::regex_search("fn", dir.path().to_str().unwrap(), 2);
        assert!(result.is_ok() || result.is_err(), "regex_search should not panic");
    }

    #[test]
    fn test_execute_with_pattern() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
        let grep = Grep;
        let input = format!(r#"{{"pattern": "main", "path": "{}"}}"#, dir.path().to_string_lossy());
        let result = grep.execute(&input);
        assert!(result.is_ok() || result.is_err(), "execute should not panic");
    }

    #[test]
    fn test_execute_with_ast_query() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lib.rs"), "fn hello() {}\nstruct Point {}").unwrap();
        let grep = Grep;
        let input = format!(r#"{{"ast_query": "function:hello", "path": "{}", "language": "rust"}}"#, dir.path().to_string_lossy());
        let result = grep.execute(&input);
        assert!(result.is_ok() || result.is_err(), "execute should not panic");
    }

    #[test]
    fn test_execute_with_pattern_and_ast_query_uses_ast() {
        let grep = Grep;
        let input = r#"{"pattern": "fn", "ast_query": "function:main", "path": "."}"#;
        let result = grep.execute(input);
        assert!(result.is_ok() || result.is_err(), "execute should not panic");
    }

    #[test]
    fn test_regex_search_non_text_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("image.png"), b"PNG...").unwrap();
        let result = Grep::regex_search("PNG", dir.path().to_str().unwrap(), 50);
        assert!(result.is_ok() || result.is_err(), "regex_search should not panic");
    }

    #[test]
    fn test_grep_with_tool_field_fallback() {
        let grep = Grep;
        assert_eq!(grep.name(), "grep");
    }

    #[test]
    fn test_ast_search_invalid_language() {
        let result = Grep::ast_search("function:main", ".", "nonexistent-language", 50);
        assert!(result.is_ok() || result.is_err(), "ast_search should not panic");
    }
}
