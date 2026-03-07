//! Code Index Module
//!
//! Provides high-performance code indexing using tree-sitter for accurate AST parsing.
//! Supports multiple languages: TypeScript, JavaScript, Python, Go, Rust.
//!
//! # Example
//!
//! ```rust
//! use zero_core::index::{CodeIndexer, Language};
//!
//! let mut indexer = CodeIndexer::new();
//! let index = indexer.index_file("main.ts", "function hello() { return 'world'; }");
//! assert!(index.functions.iter().any(|f| f.name == "hello"));
//! ```

mod extractor;
mod parser;

pub use extractor::*;
pub use parser::*;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Types
// ============================================================================

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Go,
    Rust,
    Bash,
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "rs" => Language::Rust,
            "sh" | "bash" => Language::Bash,
            _ => Language::Unknown,
        }
    }

    /// Get language from file path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }

    /// Get language name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Rust => "rust",
            Language::Bash => "bash",
            Language::Unknown => "unknown",
        }
    }
}

/// A code symbol (function, class, method, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Start line (1-indexed)
    pub start_line: u32,
    /// End line (1-indexed)
    pub end_line: u32,
    /// Start column (0-indexed)
    pub start_column: u32,
    /// End column (0-indexed)
    pub end_column: u32,
    /// Parent symbol name (for methods, nested functions)
    pub parent: Option<String>,
    /// Documentation comment (if any)
    pub doc_comment: Option<String>,
    /// Parameters (for functions/methods)
    pub parameters: Vec<String>,
    /// Return type (if available)
    pub return_type: Option<String>,
    /// Is exported/public
    pub is_exported: bool,
    /// Is async
    pub is_async: bool,
}

/// Symbol kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Type,
    Module,
    Namespace,
    Property,
    Import,
    Export,
}

/// Index for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    /// File path
    pub path: String,
    /// Detected language
    pub language: String,
    /// All symbols in the file
    pub symbols: Vec<CodeSymbol>,
    /// Functions (convenience accessor)
    pub functions: Vec<CodeSymbol>,
    /// Classes
    pub classes: Vec<CodeSymbol>,
    /// Interfaces/Types
    pub types: Vec<CodeSymbol>,
    /// Imports
    pub imports: Vec<CodeSymbol>,
    /// Exports
    pub exports: Vec<CodeSymbol>,
    /// Total lines in file
    pub total_lines: u32,
    /// Parse errors (if any)
    pub errors: Vec<String>,
}

impl FileIndex {
    pub fn empty(path: &str) -> Self {
        Self {
            path: path.to_string(),
            ..Default::default()
        }
    }
}

/// Index for a project (multiple files)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIndex {
    /// Root directory
    pub root: String,
    /// All indexed files
    pub files: HashMap<String, FileIndex>,
    /// Statistics
    pub stats: IndexStats,
}

/// Indexing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    /// Total files indexed
    pub total_files: u32,
    /// Total symbols found
    pub total_symbols: u32,
    /// Files by language
    pub by_language: HashMap<String, u32>,
    /// Parse errors count
    pub error_count: u32,
    /// Indexing duration in milliseconds
    pub duration_ms: u64,
}

/// Options for indexing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexOptions {
    /// File patterns to include (globs)
    pub patterns: Vec<String>,
    /// Maximum directory depth
    pub max_depth: Option<u32>,
    /// Include hidden files
    pub include_hidden: bool,
    /// Include node_modules etc.
    pub include_vendor: bool,
    /// Languages to index
    pub languages: Vec<String>,
}

// ============================================================================
// Code Indexer
// ============================================================================

/// Main code indexer using tree-sitter
pub struct CodeIndexer {
    ts_parser: LanguageParser,
    js_parser: LanguageParser,
    py_parser: LanguageParser,
    go_parser: LanguageParser,
    rust_parser: LanguageParser,
    bash_parser: LanguageParser,
}

impl CodeIndexer {
    /// Create a new code indexer with all language parsers initialized
    pub fn new() -> Self {
        Self {
            ts_parser: LanguageParser::new(Language::TypeScript),
            js_parser: LanguageParser::new(Language::JavaScript),
            py_parser: LanguageParser::new(Language::Python),
            go_parser: LanguageParser::new(Language::Go),
            rust_parser: LanguageParser::new(Language::Rust),
            bash_parser: LanguageParser::new(Language::Bash),
        }
    }

    /// Index a single file from content
    pub fn index_file(&mut self, path: &str, content: &str) -> FileIndex {
        let lang = Language::from_path(Path::new(path));

        if lang == Language::Unknown {
            return FileIndex::empty(path);
        }

        let parser = self.get_parser_mut(lang);
        let symbols = parser.parse(content);

        // Categorize symbols
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut types = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        for symbol in &symbols {
            match symbol.kind {
                SymbolKind::Function | SymbolKind::Method => functions.push(symbol.clone()),
                SymbolKind::Class | SymbolKind::Struct => classes.push(symbol.clone()),
                SymbolKind::Interface | SymbolKind::Type | SymbolKind::Enum => types.push(symbol.clone()),
                SymbolKind::Import => imports.push(symbol.clone()),
                SymbolKind::Export => exports.push(symbol.clone()),
                _ => {}
            }
        }

        let total_lines = content.lines().count() as u32;

        FileIndex {
            path: path.to_string(),
            language: lang.as_str().to_string(),
            symbols: symbols.clone(),
            functions,
            classes,
            types,
            imports,
            exports,
            total_lines,
            errors: Vec::new(),
        }
    }

    /// Index a directory with options
    pub fn index_directory(&self, root: &Path, options: &IndexOptions) -> ProjectIndex {
        let start = std::time::Instant::now();

        // Collect files to index
        let files: Vec<PathBuf> = self.collect_files(root, options);

        // Index files in parallel
        let indexed: Vec<(String, FileIndex)> = files
            .par_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(path).ok()?;
                let relative = path.strip_prefix(root).unwrap_or(path);
                let path_str = relative.to_string_lossy().to_string();

                // Create a new parser for thread safety
                let mut indexer = CodeIndexer::new();
                let index = indexer.index_file(&path_str, &content);

                if !index.symbols.is_empty() {
                    Some((path_str, index))
                } else {
                    None
                }
            })
            .collect();

        // Build stats
        let mut by_language: HashMap<String, u32> = HashMap::new();
        let mut total_symbols = 0u32;
        let mut error_count = 0u32;

        for (_, index) in &indexed {
            *by_language.entry(index.language.clone()).or_default() += 1;
            total_symbols += index.symbols.len() as u32;
            error_count += index.errors.len() as u32;
        }

        let files_map: HashMap<String, FileIndex> = indexed.into_iter().collect();

        ProjectIndex {
            root: root.to_string_lossy().to_string(),
            stats: IndexStats {
                total_files: files_map.len() as u32,
                total_symbols,
                by_language,
                error_count,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            files: files_map,
        }
    }

    fn get_parser_mut(&mut self, lang: Language) -> &mut LanguageParser {
        match lang {
            Language::TypeScript => &mut self.ts_parser,
            Language::JavaScript => &mut self.js_parser,
            Language::Python => &mut self.py_parser,
            Language::Go => &mut self.go_parser,
            Language::Rust => &mut self.rust_parser,
            Language::Bash => &mut self.bash_parser,
            Language::Unknown => &mut self.ts_parser, // fallback
        }
    }

    fn collect_files(&self, root: &Path, options: &IndexOptions) -> Vec<PathBuf> {
        use ignore::WalkBuilder;

        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(!options.include_hidden)
            .git_ignore(!options.include_vendor)
            .git_global(!options.include_vendor)
            .git_exclude(!options.include_vendor);

        if let Some(max_depth) = options.max_depth {
            builder.max_depth(Some(max_depth as usize));
        }

        builder
            .build()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .filter(|e| {
                let lang = Language::from_path(e.path());
                lang != Language::Unknown
            })
            .map(|e| e.into_path())
            .collect()
    }
}

impl Default for CodeIndexer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("sh"), Language::Bash);
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn test_index_typescript_function() {
        let mut indexer = CodeIndexer::new();
        let code = r#"
export function hello(name: string): string {
    return `Hello, ${name}!`;
}

const goodbye = () => {
    return 'Goodbye';
};
"#;

        let index = indexer.index_file("test.ts", code);
        assert_eq!(index.language, "typescript");
        assert!(!index.functions.is_empty(), "Should find functions");
    }

    #[test]
    fn test_index_python_function() {
        let mut indexer = CodeIndexer::new();
        let code = r#"
def hello(name: str) -> str:
    """Say hello."""
    return f"Hello, {name}!"

class Greeter:
    def greet(self, name: str) -> str:
        return f"Greetings, {name}!"
"#;

        let index = indexer.index_file("test.py", code);
        assert_eq!(index.language, "python");
        assert!(!index.functions.is_empty() || !index.classes.is_empty());
    }

    #[test]
    fn test_index_go_function() {
        let mut indexer = CodeIndexer::new();
        let code = r#"
package main

func Hello(name string) string {
    return "Hello, " + name
}

type Greeter struct {
    Name string
}

func (g *Greeter) Greet() string {
    return "Hello, " + g.Name
}
"#;

        let index = indexer.index_file("main.go", code);
        assert_eq!(index.language, "go");
    }

    #[test]
    fn test_index_rust_function() {
        let mut indexer = CodeIndexer::new();
        let code = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub struct Greeter {
    name: String,
}

impl Greeter {
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}
"#;

        let index = indexer.index_file("lib.rs", code);
        assert_eq!(index.language, "rust");
    }

    #[test]
    fn test_file_index_empty() {
        let index = FileIndex::empty("unknown.xyz");
        assert!(index.symbols.is_empty());
    }
}
