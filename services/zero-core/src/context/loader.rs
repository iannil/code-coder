//! Context Loader - High-performance project analysis
//!
//! Provides parallel directory scanning, tree-sitter based import extraction,
//! dependency graph building, and file categorization.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::fingerprint::{DirectoryInfo, FingerprintInfo, ProjectLanguage};

// ============================================================================
// Types
// ============================================================================

/// File entry in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Absolute path
    pub path: String,
    /// Relative path from root
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    /// File name
    pub name: String,
    /// File extension (if any)
    pub extension: Option<String>,
    /// Is this a directory
    pub directory: bool,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp (ms since epoch)
    #[serde(rename = "lastModified")]
    pub last_modified: u64,
}

/// Directory structure (recursive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryStructure {
    /// Directory path
    pub path: String,
    /// Directory name
    pub name: String,
    /// Files in this directory
    pub files: Vec<String>,
    /// Subdirectories
    pub subdirectories: Vec<DirectoryStructure>,
}

/// File index for fast lookups
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileIndex {
    /// Files by path
    #[serde(rename = "byPath")]
    pub by_path: HashMap<String, FileEntry>,
    /// Files grouped by extension
    #[serde(rename = "byExtension")]
    pub by_extension: HashMap<String, Vec<String>>,
    /// Files grouped by name
    #[serde(rename = "byName")]
    pub by_name: HashMap<String, Vec<String>>,
    /// Route files
    pub routes: Vec<String>,
    /// Component files
    pub components: Vec<String>,
    /// Test files
    pub tests: Vec<String>,
    /// Config files
    pub configs: Vec<String>,
}

/// Dependency graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Imports: file -> list of imported files
    pub imports: HashMap<String, Vec<String>>,
    /// Reverse imports: file -> list of files that import it
    #[serde(rename = "importedBy")]
    pub imported_by: HashMap<String, Vec<String>>,
}

/// Import info extracted from source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// Import source (the path in the import statement)
    pub source: String,
    /// Resolved relative path
    #[serde(rename = "resolvedPath")]
    pub resolved_path: Option<String>,
    /// Import type
    #[serde(rename = "importType")]
    pub import_type: ImportType,
}

/// Type of import
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportType {
    /// ES Module import
    EsModule,
    /// CommonJS require
    CommonJs,
    /// Dynamic import
    Dynamic,
    /// Re-export
    ReExport,
}

/// Scan options
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Maximum depth to scan
    pub max_depth: u32,
    /// Include hidden files/directories
    pub include_hidden: bool,
    /// Additional ignore patterns
    pub ignore_patterns: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 10,
            include_hidden: false,
            ignore_patterns: Vec::new(),
        }
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Directories to ignore during scanning
const IGNORED_DIRECTORIES: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    "target",
    "bin",
    "obj",
    ".vscode",
    ".idea",
    "coverage",
    ".cache",
    ".turbo",
    ".sst",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    "env",
];

/// Config file patterns
const CONFIG_FILE_PATTERNS: &[&str] = &[
    "package.json",
    "tsconfig.json",
    "jsconfig.json",
    "pyproject.toml",
    "go.mod",
    "Cargo.toml",
];

// ============================================================================
// Context Loader
// ============================================================================

/// High-performance context loader
pub struct ContextLoader {
    root: PathBuf,
    options: ScanOptions,
    ignored: HashSet<String>,
}

impl ContextLoader {
    /// Create a new context loader for the given root directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self::with_options(root, ScanOptions::default())
    }

    /// Create a new context loader with custom options
    pub fn with_options(root: impl AsRef<Path>, options: ScanOptions) -> Self {
        let mut ignored: HashSet<String> = IGNORED_DIRECTORIES.iter().map(|s| s.to_string()).collect();
        ignored.extend(options.ignore_patterns.iter().cloned());

        Self {
            root: root.as_ref().to_path_buf(),
            options,
            ignored,
        }
    }

    /// Scan the directory recursively and return file entries
    pub fn scan(&self) -> Result<(Vec<FileEntry>, DirectoryStructure)> {
        let entries = self.scan_parallel()?;
        let structure = self.build_structure(&entries);
        Ok((entries, structure))
    }

    /// Scan directory in parallel using rayon
    fn scan_parallel(&self) -> Result<Vec<FileEntry>> {
        let walker = WalkDir::new(&self.root)
            .max_depth(self.options.max_depth as usize)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| self.should_include(e));

        // Collect paths first (WalkDir is not Send)
        let paths: Vec<_> = walker
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != self.root)
            .map(|e| e.path().to_path_buf())
            .collect();

        // Process in parallel
        let entries: Vec<FileEntry> = paths
            .par_iter()
            .filter_map(|path| self.create_entry(path).ok())
            .collect();

        Ok(entries)
    }

    /// Check if a directory entry should be included
    fn should_include(&self, entry: &walkdir::DirEntry) -> bool {
        // Always include the root directory itself
        if entry.path() == self.root {
            return true;
        }

        let name = entry.file_name().to_string_lossy();

        // Check if it's an ignored directory
        if entry.file_type().is_dir() {
            if self.ignored.contains(name.as_ref()) {
                return false;
            }
            if !self.options.include_hidden && name.starts_with('.') {
                return false;
            }
        }

        true
    }

    /// Create a FileEntry from a path
    fn create_entry(&self, path: &Path) -> Result<FileEntry> {
        let metadata = std::fs::metadata(path)?;
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let relative_path = path.strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let extension = path.extension()
            .map(|e| e.to_string_lossy().to_string());

        let last_modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(FileEntry {
            path: path.to_string_lossy().to_string(),
            relative_path,
            name,
            extension,
            directory: metadata.is_dir(),
            size: metadata.len(),
            last_modified,
        })
    }

    /// Build directory structure from entries
    fn build_structure(&self, entries: &[FileEntry]) -> DirectoryStructure {
        // Group entries by parent directory
        let mut dirs: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();

        for entry in entries {
            if entry.directory {
                dirs.entry(entry.relative_path.clone()).or_default();
            } else {
                let parent = Path::new(&entry.relative_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                dirs.entry(parent).or_default().0.push(entry.name.clone());
            }
        }

        // Build tree recursively
        self.build_structure_recursive("", &dirs)
    }

    fn build_structure_recursive(
        &self,
        path: &str,
        dirs: &HashMap<String, (Vec<String>, Vec<String>)>,
    ) -> DirectoryStructure {
        let name = if path.is_empty() {
            self.root.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        } else {
            Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        let (files, _) = dirs.get(path).cloned().unwrap_or_default();

        // Find subdirectories
        let subdirs: Vec<_> = dirs.keys()
            .filter(|k| {
                if path.is_empty() {
                    !k.contains('/') && !k.is_empty()
                } else {
                    k.starts_with(path) &&
                    k.len() > path.len() &&
                    k[path.len()..].starts_with('/') &&
                    !k[path.len() + 1..].contains('/')
                }
            })
            .cloned()
            .collect();

        let subdirectories: Vec<DirectoryStructure> = subdirs
            .into_iter()
            .map(|subdir| self.build_structure_recursive(&subdir, dirs))
            .collect();

        DirectoryStructure {
            path: if path.is_empty() {
                self.root.to_string_lossy().to_string()
            } else {
                path.to_string()
            },
            name,
            files,
            subdirectories,
        }
    }

    /// Categorize files into routes, components, tests, configs
    pub fn categorize_files(&self, entries: &[FileEntry], fingerprint: &FingerprintInfo) -> FileIndex {
        let mut index = FileIndex::default();

        // Build lookup maps in parallel
        let by_path: HashMap<String, FileEntry> = entries.par_iter()
            .filter(|e| !e.directory)
            .map(|e| (e.relative_path.clone(), e.clone()))
            .collect();

        let by_extension: HashMap<String, Vec<String>> = entries.par_iter()
            .filter(|e| !e.directory && e.extension.is_some())
            .fold(
                || HashMap::<String, Vec<String>>::new(),
                |mut acc: HashMap<String, Vec<String>>, e| {
                    if let Some(ext) = &e.extension {
                        acc.entry(ext.clone()).or_default().push(e.relative_path.clone());
                    }
                    acc
                },
            )
            .reduce(
                || HashMap::new(),
                |mut a, b| {
                    for (k, v) in b {
                        a.entry(k).or_default().extend(v);
                    }
                    a
                },
            );

        let by_name: HashMap<String, Vec<String>> = entries.par_iter()
            .filter(|e| !e.directory)
            .fold(
                || HashMap::<String, Vec<String>>::new(),
                |mut acc: HashMap<String, Vec<String>>, e| {
                    acc.entry(e.name.clone()).or_default().push(e.relative_path.clone());
                    acc
                },
            )
            .reduce(
                || HashMap::new(),
                |mut a, b| {
                    for (k, v) in b {
                        a.entry(k).or_default().extend(v);
                    }
                    a
                },
            );

        // Categorize files
        let test_dirs: HashSet<_> = fingerprint.directories.tests.iter().cloned().collect();
        let component_patterns = self.get_component_patterns(&fingerprint.directories);
        let route_patterns = self.get_route_patterns(&fingerprint.directories);

        let categorized: (Vec<String>, Vec<String>, Vec<String>, Vec<String>) = entries.par_iter()
            .filter(|e| !e.directory)
            .fold(
                || (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                |mut acc, entry| {
                    let path = &entry.relative_path;
                    let name = &entry.name;

                    // Tests
                    let is_test = test_dirs.iter().any(|d| path.starts_with(d)) ||
                        name.contains(".test.") ||
                        name.contains(".spec.") ||
                        name.contains("_test.");
                    if is_test {
                        acc.2.push(path.clone());
                    }

                    // Configs
                    let is_config = name.contains(".config.") ||
                        name.starts_with('.') ||
                        CONFIG_FILE_PATTERNS.contains(&name.as_str());
                    if is_config {
                        acc.3.push(path.clone());
                    }

                    // Components
                    let is_component = component_patterns.iter().any(|p| {
                        path.starts_with(p) || path.starts_with(&format!("{}/", p))
                    });
                    if is_component {
                        acc.1.push(path.clone());
                    }

                    // Routes
                    let is_route = route_patterns.iter().any(|p| {
                        path.starts_with(p) || path.starts_with(&format!("{}/", p))
                    });
                    if is_route {
                        acc.0.push(path.clone());
                    }

                    acc
                },
            )
            .reduce(
                || (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                |mut a, b| {
                    a.0.extend(b.0);
                    a.1.extend(b.1);
                    a.2.extend(b.2);
                    a.3.extend(b.3);
                    a
                },
            );

        index.by_path = by_path;
        index.by_extension = by_extension;
        index.by_name = by_name;
        index.routes = categorized.0;
        index.components = categorized.1;
        index.tests = categorized.2;
        index.configs = categorized.3;

        index
    }

    fn get_component_patterns(&self, dirs: &DirectoryInfo) -> Vec<String> {
        let mut patterns = vec![
            "components".to_string(),
            "Components".to_string(),
            "src/components".to_string(),
        ];
        if let Some(ref c) = dirs.components {
            patterns.push(c.clone());
        }
        patterns
    }

    fn get_route_patterns(&self, dirs: &DirectoryInfo) -> Vec<String> {
        let mut patterns = vec![
            "routes".to_string(),
            "pages".to_string(),
            "src/routes".to_string(),
            "src/pages".to_string(),
            "app/routes".to_string(),
            "app/pages".to_string(),
        ];
        if let Some(ref r) = dirs.routes {
            patterns.push(r.clone());
        }
        if let Some(ref p) = dirs.pages {
            patterns.push(p.clone());
        }
        patterns
    }

    /// Extract imports from source files
    pub fn extract_imports(&self, entries: &[FileEntry], language: ProjectLanguage) -> DependencyGraph {
        let code_extensions = self.get_code_extensions(language);

        // Filter to code files only
        let code_files: Vec<_> = entries.par_iter()
            .filter(|e| {
                !e.directory &&
                e.extension.as_ref()
                    .map(|ext| code_extensions.contains(&ext.as_str()))
                    .unwrap_or(false)
            })
            .collect();

        // Extract imports in parallel
        let imports_map: HashMap<String, Vec<String>> = code_files.par_iter()
            .filter_map(|entry| {
                let content = std::fs::read_to_string(&entry.path).ok()?;
                let ext = entry.extension.as_ref()?;
                let imports = self.extract_import_paths(&content, ext, &entry.relative_path);
                if imports.is_empty() {
                    None
                } else {
                    Some((entry.relative_path.clone(), imports))
                }
            })
            .collect();

        // Build reverse map
        let mut imported_by: HashMap<String, Vec<String>> = HashMap::new();
        for (file, imports) in &imports_map {
            for imp in imports {
                imported_by.entry(imp.clone()).or_default().push(file.clone());
            }
        }

        DependencyGraph {
            imports: imports_map,
            imported_by,
        }
    }

    fn get_code_extensions(&self, language: ProjectLanguage) -> Vec<&'static str> {
        match language {
            ProjectLanguage::TypeScript => vec!["ts", "tsx"],
            ProjectLanguage::JavaScript => vec!["js", "jsx", "mjs"],
            ProjectLanguage::Python => vec!["py"],
            ProjectLanguage::Go => vec!["go"],
            ProjectLanguage::Rust => vec!["rs"],
            ProjectLanguage::CSharp => vec!["cs"],
            ProjectLanguage::Java => vec!["java"],
            ProjectLanguage::Other => vec!["ts", "tsx", "js", "jsx"],
        }
    }

    /// Extract import paths from source content using regex patterns
    /// Note: For production, consider using tree-sitter for more accurate parsing
    fn extract_import_paths(&self, content: &str, extension: &str, file_path: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let relative_dir = Path::new(file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        match extension {
            "ts" | "tsx" | "js" | "jsx" | "mjs" => {
                self.extract_js_imports(content, &relative_dir, extension, &mut imports);
            }
            "py" => {
                self.extract_python_imports(content, &mut imports);
            }
            "go" => {
                self.extract_go_imports(content, &mut imports);
            }
            _ => {}
        }

        imports
    }

    fn extract_js_imports(&self, content: &str, relative_dir: &str, extension: &str, imports: &mut Vec<String>) {
        // ES Module imports: import X from 'path'
        let import_re = regex::Regex::new(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        // Dynamic imports: import('path')
        let dynamic_re = regex::Regex::new(r#"import\(['"]([^'"]+)['"]\)"#).unwrap();
        // CommonJS requires: require('path')
        let require_re = regex::Regex::new(r#"require\(['"]([^'"]+)['"]\)"#).unwrap();
        // Re-exports: export X from 'path'
        let export_re = regex::Regex::new(r#"export\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap();

        for cap in import_re.captures_iter(content)
            .chain(dynamic_re.captures_iter(content))
            .chain(require_re.captures_iter(content))
            .chain(export_re.captures_iter(content))
        {
            if let Some(path) = cap.get(1) {
                let import_path = path.as_str();
                if import_path.starts_with('.') || import_path.starts_with('/') {
                    let resolved = self.resolve_import_path(import_path, relative_dir, extension);
                    imports.push(resolved);
                }
            }
        }
    }

    fn extract_python_imports(&self, content: &str, imports: &mut Vec<String>) {
        // from X import Y
        let from_re = regex::Regex::new(r#"from\s+([^\s]+)\s+import"#).unwrap();
        // import X
        let import_re = regex::Regex::new(r#"import\s+([^\s]+)"#).unwrap();

        for cap in from_re.captures_iter(content).chain(import_re.captures_iter(content)) {
            if let Some(path) = cap.get(1) {
                let import_path = path.as_str();
                if import_path.starts_with('.') {
                    // Convert Python relative import to path
                    let resolved = import_path.replace('.', "/") + ".py";
                    imports.push(resolved);
                }
            }
        }
    }

    fn extract_go_imports(&self, content: &str, imports: &mut Vec<String>) {
        // import "path"
        let import_re = regex::Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap();

        for cap in import_re.captures_iter(content) {
            if let Some(path) = cap.get(1) {
                let import_path = path.as_str();
                if import_path.starts_with('.') {
                    let resolved = import_path.replace("./", "") + ".go";
                    imports.push(resolved);
                }
            }
        }
    }

    fn resolve_import_path(&self, import_path: &str, relative_dir: &str, extension: &str) -> String {
        let mut resolved = if relative_dir.is_empty() {
            import_path.trim_start_matches("./").to_string()
        } else {
            let joined = Path::new(relative_dir).join(import_path);
            // Normalize path (handle ..)
            let mut components: Vec<_> = Vec::new();
            for comp in joined.components() {
                match comp {
                    std::path::Component::ParentDir => {
                        components.pop();
                    }
                    std::path::Component::Normal(s) => {
                        components.push(s.to_string_lossy().to_string());
                    }
                    std::path::Component::CurDir => {}
                    _ => {}
                }
            }
            components.join("/")
        };

        // Add extension if not present
        if !resolved.contains('.') ||
           (!resolved.ends_with(&format!(".{}", extension)) &&
            !resolved.ends_with(".ts") &&
            !resolved.ends_with(".tsx") &&
            !resolved.ends_with(".js") &&
            !resolved.ends_with(".jsx"))
        {
            resolved.push_str(&format!(".{}", extension));
        }

        resolved.replace('\\', "/")
    }

    /// Find files related to a given file
    pub fn find_related_files(&self, file_path: &str, index: &FileIndex, deps: &DependencyGraph) -> Vec<String> {
        let mut related = HashSet::new();

        if let Some(entry) = index.by_path.get(file_path) {
            let dir = Path::new(file_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            // Same extension, same directory
            if let Some(ext) = &entry.extension {
                if let Some(same_ext) = index.by_extension.get(ext) {
                    for other in same_ext {
                        if Path::new(other)
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default() == dir
                        {
                            related.insert(other.clone());
                        }
                    }
                }
            }

            // Dependencies
            if let Some(imports) = deps.imports.get(file_path) {
                related.extend(imports.iter().cloned());
            }
            if let Some(importers) = deps.imported_by.get(file_path) {
                related.extend(importers.iter().cloned());
            }

            // Test files
            let base_name = entry.name.split('.').next().unwrap_or(&entry.name);
            let test_patterns = [
                format!("{}.test", base_name),
                format!("{}.spec", base_name),
                format!("{}_test", base_name),
            ];

            for (name, paths) in &index.by_name {
                for pattern in &test_patterns {
                    if name.contains(pattern) {
                        related.extend(paths.iter().cloned());
                    }
                }
            }
        }

        related.remove(file_path);
        related.into_iter().collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_directory() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("test.ts"), "export const x = 1;").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.ts"), "import { x } from '../test';").unwrap();

        let loader = ContextLoader::new(dir.path());
        let (entries, structure) = loader.scan().unwrap();

        assert!(entries.len() >= 2, "Expected at least 2 entries, got {}", entries.len());
        assert!(entries.iter().any(|e| e.name == "test.ts"));
        assert!(!structure.subdirectories.is_empty() || !structure.files.is_empty());
    }

    #[test]
    fn test_extract_js_imports() {
        let content = r#"
            import React from 'react';
            import { foo } from './utils';
            import type { Bar } from '../types';
            const dynamic = import('./lazy');
            const cjs = require('./cjs-module');
            export { baz } from './exports';
        "#;

        let loader = ContextLoader::new(".");
        let mut imports = Vec::new();
        loader.extract_js_imports(content, "src/components", "ts", &mut imports);

        // Should only include relative imports
        assert!(imports.iter().any(|i| i.contains("utils")));
        assert!(imports.iter().any(|i| i.contains("types")));
        assert!(imports.iter().any(|i| i.contains("lazy")));
        assert!(imports.iter().any(|i| i.contains("cjs-module")));
        assert!(imports.iter().any(|i| i.contains("exports")));
        // Should NOT include 'react' (external package)
        assert!(!imports.iter().any(|i| i == "react"));
    }

    #[test]
    fn test_resolve_import_path() {
        let loader = ContextLoader::new(".");

        // Simple relative
        let resolved = loader.resolve_import_path("./utils", "src", "ts");
        assert_eq!(resolved, "src/utils.ts");

        // Parent directory
        let resolved = loader.resolve_import_path("../shared", "src/components", "ts");
        assert_eq!(resolved, "src/shared.ts");

        // With extension
        let resolved = loader.resolve_import_path("./styles.css", "src", "ts");
        assert!(resolved.contains("styles.css"));
    }

    #[test]
    fn test_ignored_directories() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        std::fs::write(dir.path().join("node_modules/pkg/index.js"), "").unwrap();
        std::fs::write(dir.path().join("src.ts"), "").unwrap();

        let loader = ContextLoader::new(dir.path());
        let (entries, _) = loader.scan().unwrap();

        // Should not include node_modules
        assert!(!entries.iter().any(|e| e.relative_path.contains("node_modules")));
        // Should include src.ts
        assert!(entries.iter().any(|e| e.name == "src.ts"));
    }
}
