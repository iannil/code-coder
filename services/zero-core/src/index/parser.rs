//! Language Parser Module
//!
//! Wraps tree-sitter parsers for each supported language.

use tree_sitter::{Parser, Tree};

use super::{CodeSymbol, Language as CodeLang};

/// Language-specific parser wrapper
pub struct LanguageParser {
    parser: Parser,
    language: CodeLang,
}

impl LanguageParser {
    /// Create a new parser for the specified language
    pub fn new(language: CodeLang) -> Self {
        let mut parser = Parser::new();

        let ts_lang = match language {
            CodeLang::TypeScript => tree_sitter_typescript::LANGUAGE_TSX.into(),
            CodeLang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            CodeLang::Python => tree_sitter_python::LANGUAGE.into(),
            CodeLang::Go => tree_sitter_go::LANGUAGE.into(),
            CodeLang::Rust => tree_sitter_rust::LANGUAGE.into(),
            CodeLang::Bash => tree_sitter_bash::LANGUAGE.into(),
            CodeLang::Unknown => tree_sitter_javascript::LANGUAGE.into(), // fallback
        };

        parser.set_language(&ts_lang).expect("Failed to set language");

        Self { parser, language }
    }

    /// Parse source code and extract symbols
    pub fn parse(&mut self, source: &str) -> Vec<CodeSymbol> {
        let tree = match self.parser.parse(source, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root = tree.root_node();
        let mut symbols = Vec::new();

        // Extract symbols based on language
        match self.language {
            CodeLang::TypeScript | CodeLang::JavaScript => {
                super::extractor::extract_js_ts_symbols(&root, source, &mut symbols);
            }
            CodeLang::Python => {
                super::extractor::extract_python_symbols(&root, source, &mut symbols);
            }
            CodeLang::Go => {
                super::extractor::extract_go_symbols(&root, source, &mut symbols);
            }
            CodeLang::Rust => {
                super::extractor::extract_rust_symbols(&root, source, &mut symbols);
            }
            CodeLang::Bash => {
                super::extractor::extract_bash_symbols(&root, source, &mut symbols);
            }
            CodeLang::Unknown => {}
        }

        symbols
    }

    /// Get the syntax tree for advanced operations
    pub fn parse_tree(&mut self, source: &str) -> Option<Tree> {
        self.parser.parse(source, None)
    }
}

impl Clone for LanguageParser {
    fn clone(&self) -> Self {
        Self::new(self.language)
    }
}
