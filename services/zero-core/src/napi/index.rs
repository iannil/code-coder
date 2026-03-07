//! NAPI bindings for Code Index
//!
//! Provides Node.js bindings for tree-sitter-based code indexing.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::path::Path;

use crate::index::{
    CodeIndexer, CodeSymbol, FileIndex, IndexOptions, IndexStats, Language, ProjectIndex,
    SymbolKind,
};

// ============================================================================
// NAPI Types
// ============================================================================

/// Supported language
#[napi(string_enum)]
pub enum NapiLanguage {
    Typescript,
    Javascript,
    Python,
    Go,
    Rust,
    Bash,
    Unknown,
}

impl From<Language> for NapiLanguage {
    fn from(lang: Language) -> Self {
        match lang {
            Language::TypeScript => NapiLanguage::Typescript,
            Language::JavaScript => NapiLanguage::Javascript,
            Language::Python => NapiLanguage::Python,
            Language::Go => NapiLanguage::Go,
            Language::Rust => NapiLanguage::Rust,
            Language::Bash => NapiLanguage::Bash,
            Language::Unknown => NapiLanguage::Unknown,
        }
    }
}

/// Symbol kind
#[napi(string_enum)]
pub enum NapiSymbolKind {
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

impl From<SymbolKind> for NapiSymbolKind {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Function => NapiSymbolKind::Function,
            SymbolKind::Method => NapiSymbolKind::Method,
            SymbolKind::Class => NapiSymbolKind::Class,
            SymbolKind::Interface => NapiSymbolKind::Interface,
            SymbolKind::Struct => NapiSymbolKind::Struct,
            SymbolKind::Enum => NapiSymbolKind::Enum,
            SymbolKind::Variable => NapiSymbolKind::Variable,
            SymbolKind::Constant => NapiSymbolKind::Constant,
            SymbolKind::Type => NapiSymbolKind::Type,
            SymbolKind::Module => NapiSymbolKind::Module,
            SymbolKind::Namespace => NapiSymbolKind::Namespace,
            SymbolKind::Property => NapiSymbolKind::Property,
            SymbolKind::Import => NapiSymbolKind::Import,
            SymbolKind::Export => NapiSymbolKind::Export,
        }
    }
}

/// Code symbol
#[napi(object)]
pub struct NapiCodeSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: String,
    /// Start line (1-indexed)
    pub start_line: u32,
    /// End line (1-indexed)
    pub end_line: u32,
    /// Start column (0-indexed)
    pub start_column: u32,
    /// End column (0-indexed)
    pub end_column: u32,
    /// Parent symbol name
    pub parent: Option<String>,
    /// Documentation comment
    pub doc_comment: Option<String>,
    /// Parameters
    pub parameters: Vec<String>,
    /// Return type
    pub return_type: Option<String>,
    /// Is exported
    pub is_exported: bool,
    /// Is async
    pub is_async: bool,
}

impl From<CodeSymbol> for NapiCodeSymbol {
    fn from(s: CodeSymbol) -> Self {
        Self {
            name: s.name,
            kind: format!("{:?}", s.kind).to_lowercase(),
            start_line: s.start_line,
            end_line: s.end_line,
            start_column: s.start_column,
            end_column: s.end_column,
            parent: s.parent,
            doc_comment: s.doc_comment,
            parameters: s.parameters,
            return_type: s.return_type,
            is_exported: s.is_exported,
            is_async: s.is_async,
        }
    }
}

/// File index result
#[napi(object)]
pub struct NapiFileIndex {
    /// File path
    pub path: String,
    /// Language
    pub language: String,
    /// All symbols
    pub symbols: Vec<NapiCodeSymbol>,
    /// Function symbols
    pub functions: Vec<NapiCodeSymbol>,
    /// Class symbols
    pub classes: Vec<NapiCodeSymbol>,
    /// Type symbols
    pub types: Vec<NapiCodeSymbol>,
    /// Import symbols
    pub imports: Vec<NapiCodeSymbol>,
    /// Export symbols
    pub exports: Vec<NapiCodeSymbol>,
    /// Total lines
    pub total_lines: u32,
    /// Parse errors
    pub errors: Vec<String>,
}

impl From<FileIndex> for NapiFileIndex {
    fn from(f: FileIndex) -> Self {
        Self {
            path: f.path,
            language: f.language,
            symbols: f.symbols.into_iter().map(NapiCodeSymbol::from).collect(),
            functions: f.functions.into_iter().map(NapiCodeSymbol::from).collect(),
            classes: f.classes.into_iter().map(NapiCodeSymbol::from).collect(),
            types: f.types.into_iter().map(NapiCodeSymbol::from).collect(),
            imports: f.imports.into_iter().map(NapiCodeSymbol::from).collect(),
            exports: f.exports.into_iter().map(NapiCodeSymbol::from).collect(),
            total_lines: f.total_lines,
            errors: f.errors,
        }
    }
}

/// Index statistics
#[napi(object)]
pub struct NapiIndexStats {
    /// Total files
    pub total_files: u32,
    /// Total symbols
    pub total_symbols: u32,
    /// Files by language
    pub by_language: HashMap<String, u32>,
    /// Error count
    pub error_count: u32,
    /// Duration in milliseconds
    pub duration_ms: u32,
}

impl From<IndexStats> for NapiIndexStats {
    fn from(s: IndexStats) -> Self {
        Self {
            total_files: s.total_files,
            total_symbols: s.total_symbols,
            by_language: s.by_language,
            error_count: s.error_count,
            duration_ms: s.duration_ms as u32,
        }
    }
}

/// Project index result
#[napi(object)]
pub struct NapiProjectIndex {
    /// Root directory
    pub root: String,
    /// Indexed files
    pub files: HashMap<String, NapiFileIndex>,
    /// Statistics
    pub stats: NapiIndexStats,
}

impl From<ProjectIndex> for NapiProjectIndex {
    fn from(p: ProjectIndex) -> Self {
        Self {
            root: p.root,
            files: p.files.into_iter().map(|(k, v)| (k, v.into())).collect(),
            stats: p.stats.into(),
        }
    }
}

/// Index options
#[napi(object)]
#[derive(Default)]
pub struct NapiIndexOptions {
    /// File patterns to include
    pub patterns: Option<Vec<String>>,
    /// Maximum depth
    pub max_depth: Option<u32>,
    /// Include hidden files
    pub include_hidden: Option<bool>,
    /// Include vendor directories
    pub include_vendor: Option<bool>,
    /// Languages to index
    pub languages: Option<Vec<String>>,
}

impl From<NapiIndexOptions> for IndexOptions {
    fn from(o: NapiIndexOptions) -> Self {
        IndexOptions {
            patterns: o.patterns.unwrap_or_default(),
            max_depth: o.max_depth,
            include_hidden: o.include_hidden.unwrap_or(false),
            include_vendor: o.include_vendor.unwrap_or(false),
            languages: o.languages.unwrap_or_default(),
        }
    }
}

// ============================================================================
// NAPI Handle
// ============================================================================

/// Handle to a code indexer
#[napi]
pub struct CodeIndexerHandle {
    inner: CodeIndexer,
}

#[napi]
impl CodeIndexerHandle {
    /// Create a new code indexer
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CodeIndexer::new(),
        }
    }

    /// Index a single file from content
    #[napi]
    pub fn index_file(&mut self, path: String, content: String) -> NapiFileIndex {
        self.inner.index_file(&path, &content).into()
    }

    /// Index a directory
    #[napi]
    pub fn index_directory(&self, root: String, options: Option<NapiIndexOptions>) -> NapiProjectIndex {
        let opts = options.unwrap_or_default().into();
        self.inner.index_directory(Path::new(&root), &opts).into()
    }

    /// Detect language from file path
    #[napi]
    pub fn detect_language(&self, path: String) -> String {
        Language::from_path(Path::new(&path)).as_str().to_string()
    }
}

impl Default for CodeIndexerHandle {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Create a new code indexer
#[napi]
pub fn create_code_indexer() -> CodeIndexerHandle {
    CodeIndexerHandle::new()
}

/// Index a single file from content
#[napi]
pub fn index_file_content(path: String, content: String) -> NapiFileIndex {
    let mut indexer = CodeIndexer::new();
    indexer.index_file(&path, &content).into()
}

/// Index multiple files (batch operation)
#[napi]
pub fn index_files_batch(files: Vec<NapiFileInput>) -> Vec<NapiFileIndex> {
    let mut indexer = CodeIndexer::new();
    files
        .into_iter()
        .map(|f| indexer.index_file(&f.path, &f.content).into())
        .collect()
}

/// File input for batch indexing
#[napi(object)]
pub struct NapiFileInput {
    pub path: String,
    pub content: String,
}

/// Detect language from file extension
#[napi]
pub fn detect_language_from_path(path: String) -> String {
    Language::from_path(Path::new(&path)).as_str().to_string()
}

/// Get supported languages
#[napi]
pub fn get_supported_languages() -> Vec<String> {
    vec![
        "typescript".to_string(),
        "javascript".to_string(),
        "python".to_string(),
        "go".to_string(),
        "rust".to_string(),
        "bash".to_string(),
    ]
}
