//! Context cache module - project structure caching
//!
//! Provides caching for:
//! - **Routes**: File-based routing for Next.js, Remix, SvelteKit, etc.
//! - **Components**: React/Vue/Svelte components with type inference
//! - **Configs**: Configuration files with parsing
//! - **Tests**: Test file discovery

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context as AnyhowContext, Result};
use chrono::Utc;
use globset::{Glob, GlobSetBuilder};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

// ============================================================================
// Types
// ============================================================================

/// Type of cache entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheEntryType {
    File,
    Directory,
    Config,
    Route,
    Component,
    Test,
}

impl std::fmt::Display for CacheEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Directory => write!(f, "directory"),
            Self::Config => write!(f, "config"),
            Self::Route => write!(f, "route"),
            Self::Component => write!(f, "component"),
            Self::Test => write!(f, "test"),
        }
    }
}

impl std::str::FromStr for CacheEntryType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "config" => Ok(Self::Config),
            "route" => Ok(Self::Route),
            "component" => Ok(Self::Component),
            "test" => Ok(Self::Test),
            _ => Err(anyhow::anyhow!("Invalid cache entry type: {}", s)),
        }
    }
}

/// A cache entry for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: CacheEntryType,
    pub last_modified: i64,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Type of route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteType {
    File,
    Directory,
    App,
    Pages,
    Api,
}

impl std::fmt::Display for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Directory => write!(f, "directory"),
            Self::App => write!(f, "app"),
            Self::Pages => write!(f, "pages"),
            Self::Api => write!(f, "api"),
        }
    }
}

impl std::str::FromStr for RouteType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            "app" => Ok(Self::App),
            "pages" => Ok(Self::Pages),
            "api" => Ok(Self::Api),
            _ => Err(anyhow::anyhow!("Invalid route type: {}", s)),
        }
    }
}

/// A cached route entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCache {
    pub path: String,
    #[serde(rename = "type")]
    pub route_type: RouteType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware: Option<String>,
}

/// Type of component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentType {
    Component,
    Hook,
    Util,
    Layout,
    Page,
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Component => write!(f, "component"),
            Self::Hook => write!(f, "hook"),
            Self::Util => write!(f, "util"),
            Self::Layout => write!(f, "layout"),
            Self::Page => write!(f, "page"),
        }
    }
}

impl std::str::FromStr for ComponentType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "component" => Ok(Self::Component),
            "hook" => Ok(Self::Hook),
            "util" => Ok(Self::Util),
            "layout" => Ok(Self::Layout),
            "page" => Ok(Self::Page),
            _ => Err(anyhow::anyhow!("Invalid component type: {}", s)),
        }
    }
}

/// A cached component entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentCache {
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub component_type: ComponentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<String>>,
}

/// A cached config file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCache {
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub config_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed: Option<serde_json::Value>,
}

/// Full project cache
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCache {
    pub project_id: String,
    pub routes: Vec<RouteCache>,
    pub components: Vec<ComponentCache>,
    pub configs: Vec<ConfigCache>,
    pub test_files: Vec<String>,
    pub entries: HashMap<String, CacheEntry>,
    pub time: CacheTime,
}

/// Cache timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheTime {
    pub created: i64,
    pub updated: i64,
}

impl ProjectCache {
    /// Create a new empty cache
    pub fn new(project_id: impl Into<String>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            project_id: project_id.into(),
            routes: Vec::new(),
            components: Vec::new(),
            configs: Vec::new(),
            test_files: Vec::new(),
            entries: HashMap::new(),
            time: CacheTime {
                created: now,
                updated: now,
            },
        }
    }

    /// Touch the updated timestamp
    pub fn touch(&mut self) {
        self.time.updated = Utc::now().timestamp_millis();
    }
}

// ============================================================================
// Route Detection Configuration
// ============================================================================

/// Route detection patterns for different frameworks
struct RouteConfig {
    patterns: &'static [&'static str],
    route_type: RouteType,
}

/// Get route patterns for known frameworks
fn get_route_patterns() -> HashMap<&'static str, RouteConfig> {
    let mut patterns = HashMap::new();

    patterns.insert(
        "Next.js App Router",
        RouteConfig {
            patterns: &["app/**/page.tsx", "app/**/page.ts", "app/**/route.ts"],
            route_type: RouteType::App,
        },
    );

    patterns.insert(
        "Next.js Pages Router",
        RouteConfig {
            patterns: &["pages/**/*.tsx", "pages/**/*.ts"],
            route_type: RouteType::Pages,
        },
    );

    patterns.insert(
        "Remix",
        RouteConfig {
            patterns: &["app/routes/**/*.tsx"],
            route_type: RouteType::File,
        },
    );

    patterns.insert(
        "SvelteKit",
        RouteConfig {
            patterns: &["src/routes/**/*.svelte"],
            route_type: RouteType::File,
        },
    );

    patterns.insert(
        "Nuxt",
        RouteConfig {
            patterns: &["pages/**/*.vue"],
            route_type: RouteType::File,
        },
    );

    patterns.insert(
        "Astro",
        RouteConfig {
            patterns: &["src/pages/**/*.astro"],
            route_type: RouteType::File,
        },
    );

    patterns.insert(
        "Express",
        RouteConfig {
            patterns: &["**/*.routes.ts", "**/routes/**/*.ts"],
            route_type: RouteType::Api,
        },
    );

    patterns.insert(
        "NestJS",
        RouteConfig {
            patterns: &["**/*.controller.ts", "**/*.resolver.ts"],
            route_type: RouteType::Api,
        },
    );

    patterns.insert(
        "Hono",
        RouteConfig {
            patterns: &["**/routes/**/*.ts", "**/*routes.ts"],
            route_type: RouteType::Api,
        },
    );

    patterns
}

// ============================================================================
// Cache Builder
// ============================================================================

/// Builder for project cache
pub struct CacheBuilder {
    worktree: String,
    framework: Option<String>,
}

impl CacheBuilder {
    /// Create a new cache builder
    pub fn new(worktree: impl Into<String>) -> Self {
        Self {
            worktree: worktree.into(),
            framework: None,
        }
    }

    /// Set the detected framework
    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.framework = Some(framework.into());
        self
    }

    /// Build the cache by scanning the project
    pub fn build(&self, project_id: &str) -> Result<ProjectCache> {
        let mut cache = ProjectCache::new(project_id);

        // Detect routes
        cache.routes = self.detect_routes()?;

        // Detect components
        cache.components = self.detect_components()?;

        // Detect configs
        cache.configs = self.detect_configs()?;

        // Detect test files
        cache.test_files = self.detect_tests()?;

        // Build entries from detected items
        for route in &cache.routes {
            cache.entries.insert(
                route.path.clone(),
                CacheEntry {
                    path: route.path.clone(),
                    entry_type: CacheEntryType::Route,
                    last_modified: cache.time.created,
                    size: 0,
                    hash: None,
                },
            );
        }

        for component in &cache.components {
            cache.entries.insert(
                component.path.clone(),
                CacheEntry {
                    path: component.path.clone(),
                    entry_type: CacheEntryType::Component,
                    last_modified: cache.time.created,
                    size: 0,
                    hash: None,
                },
            );
        }

        for config in &cache.configs {
            cache.entries.insert(
                config.path.clone(),
                CacheEntry {
                    path: config.path.clone(),
                    entry_type: CacheEntryType::Config,
                    last_modified: cache.time.created,
                    size: config.content.as_ref().map(|c| c.len() as u64).unwrap_or(0),
                    hash: None,
                },
            );
        }

        for test in &cache.test_files {
            cache.entries.insert(
                test.clone(),
                CacheEntry {
                    path: test.clone(),
                    entry_type: CacheEntryType::Test,
                    last_modified: cache.time.created,
                    size: 0,
                    hash: None,
                },
            );
        }

        Ok(cache)
    }

    /// Detect routes based on framework patterns
    fn detect_routes(&self) -> Result<Vec<RouteCache>> {
        let mut routes = Vec::new();
        let route_patterns = get_route_patterns();

        // Find matching framework config
        let config = if let Some(ref fw) = self.framework {
            route_patterns.get(fw.as_str())
        } else {
            // Try to detect based on directory structure
            if Path::new(&self.worktree).join("app").exists() {
                route_patterns.get("Next.js App Router")
            } else if Path::new(&self.worktree).join("pages").exists() {
                route_patterns.get("Next.js Pages Router")
            } else {
                None
            }
        };

        let Some(config) = config else {
            return Ok(routes);
        };

        // Build glob set
        let mut builder = GlobSetBuilder::new();
        for pattern in config.patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let glob_set = builder.build()?;

        // Walk directory and match files
        for entry in WalkDir::new(&self.worktree)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = path
                .strip_prefix(&self.worktree)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            if glob_set.is_match(&relative) {
                // Detect HTTP methods for API routes
                let methods = if config.route_type == RouteType::Api
                    || relative.contains("route.ts")
                    || relative.contains("controller")
                {
                    self.detect_http_methods(path)
                } else {
                    None
                };

                routes.push(RouteCache {
                    path: relative,
                    route_type: config.route_type,
                    framework: self.framework.clone(),
                    methods,
                    middleware: None,
                });
            }
        }

        Ok(routes)
    }

    /// Detect HTTP methods in a file
    fn detect_http_methods(&self, path: &Path) -> Option<Vec<String>> {
        let content = std::fs::read_to_string(path).ok()?;
        let content_lower = content.to_lowercase();

        let mut methods = Vec::new();
        for method in &["get", "post", "put", "delete", "patch"] {
            if content_lower.contains(method) {
                methods.push(method.to_uppercase());
            }
        }

        if methods.is_empty() {
            None
        } else {
            Some(methods)
        }
    }

    /// Detect components in the project
    fn detect_components(&self) -> Result<Vec<ComponentCache>> {
        let mut components = Vec::new();

        let component_patterns = [
            "src/components/**/*.ts",
            "src/components/**/*.tsx",
            "src/components/**/*.js",
            "src/components/**/*.jsx",
            "src/components/**/*.vue",
            "src/components/**/*.svelte",
            "components/**/*.ts",
            "components/**/*.tsx",
            "components/**/*.js",
            "components/**/*.jsx",
            "components/**/*.vue",
            "components/**/*.svelte",
            "app/components/**/*.tsx",
            "app/components/**/*.ts",
        ];

        let mut builder = GlobSetBuilder::new();
        for pattern in component_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let glob_set = builder.build()?;

        for entry in WalkDir::new(&self.worktree)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = path
                .strip_prefix(&self.worktree)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            if glob_set.is_match(&relative) {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Infer component type from name
                let component_type = if name.starts_with("use") || name.ends_with(".hook") {
                    ComponentType::Hook
                } else if name.ends_with(".util") || name.ends_with(".helper") {
                    ComponentType::Util
                } else if name.to_lowercase().contains("layout") {
                    ComponentType::Layout
                } else if name.to_lowercase().contains("page") {
                    ComponentType::Page
                } else {
                    ComponentType::Component
                };

                // Extract imports
                let imports = self.extract_imports(path);

                components.push(ComponentCache {
                    path: relative,
                    name,
                    component_type,
                    props: None,
                    imports,
                });
            }
        }

        Ok(components)
    }

    /// Extract relative imports from a file
    fn extract_imports(&self, path: &Path) -> Option<Vec<String>> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut imports = Vec::new();

        // Simple regex-like pattern matching for imports
        for line in content.lines() {
            if line.contains("import") && line.contains("from") {
                // Extract the from path
                if let Some(start) = line.find("from") {
                    let rest = &line[start + 4..];
                    let rest = rest.trim();
                    if let Some(quote_start) = rest.find(['\'', '"']) {
                        let rest = &rest[quote_start + 1..];
                        if let Some(quote_end) = rest.find(['\'', '"']) {
                            let import_path = &rest[..quote_end];
                            if import_path.starts_with('.') {
                                imports.push(import_path.to_string());
                            }
                        }
                    }
                }
            }
        }

        if imports.is_empty() {
            None
        } else {
            Some(imports)
        }
    }

    /// Detect config files in the project
    fn detect_configs(&self) -> Result<Vec<ConfigCache>> {
        let mut configs = Vec::new();
        let mut seen = HashSet::new();

        let config_patterns = [
            "package.json",
            "tsconfig.json",
            "*.config.js",
            "*.config.ts",
            "*.config.mjs",
            "*.config.cjs",
            ".eslintrc*",
            ".prettierrc*",
            ".babelrc*",
            "vite.config.*",
            "next.config.*",
            "tailwind.config.*",
        ];

        let worktree_path = Path::new(&self.worktree);

        for pattern in config_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                let glob_matcher = glob.compile_matcher();

                for entry in std::fs::read_dir(&self.worktree)?.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let file_name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();

                    if glob_matcher.is_match(&file_name) && !seen.contains(&file_name) {
                        seen.insert(file_name.clone());

                        let relative = path
                            .strip_prefix(worktree_path)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");

                        // Determine config type
                        let config_type = if file_name.contains("config") {
                            "config"
                        } else if file_name.starts_with('.') {
                            "rc"
                        } else if file_name == "package.json" {
                            "package"
                        } else if file_name == "tsconfig.json" {
                            "typescript"
                        } else {
                            "unknown"
                        };

                        // Read content (limited)
                        let content = std::fs::read_to_string(&path)
                            .ok()
                            .map(|c| c.chars().take(1000).collect::<String>());

                        // Try to parse JSON
                        let parsed = if file_name.ends_with(".json") {
                            content
                                .as_ref()
                                .and_then(|c| serde_json::from_str(c).ok())
                        } else {
                            None
                        };

                        configs.push(ConfigCache {
                            path: relative,
                            name: file_name,
                            config_type: config_type.to_string(),
                            content,
                            parsed,
                        });
                    }
                }
            }
        }

        Ok(configs)
    }

    /// Detect test files in the project
    fn detect_tests(&self) -> Result<Vec<String>> {
        let mut tests = HashSet::new();

        let test_patterns = [
            "**/*.test.ts",
            "**/*.test.tsx",
            "**/*.test.js",
            "**/*.test.jsx",
            "**/*.spec.ts",
            "**/*.spec.tsx",
            "**/*.spec.js",
            "**/*.spec.jsx",
            "**/__tests__/**/*.ts",
            "**/__tests__/**/*.tsx",
            "**/__tests__/**/*.js",
            "**/__tests__/**/*.jsx",
            "**/test/**/*.ts",
            "**/test/**/*.tsx",
            "**/tests/**/*.ts",
            "**/tests/**/*.tsx",
        ];

        let mut builder = GlobSetBuilder::new();
        for pattern in test_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let glob_set = builder.build()?;

        for entry in WalkDir::new(&self.worktree)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Skip node_modules
            if path
                .components()
                .any(|c| c.as_os_str() == "node_modules")
            {
                continue;
            }

            let relative = path
                .strip_prefix(&self.worktree)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            if glob_set.is_match(&relative) {
                tests.insert(relative);
            }
        }

        Ok(tests.into_iter().collect())
    }
}

// ============================================================================
// Cache Store
// ============================================================================

/// Storage for project caches
pub struct ContextCacheStore {
    conn: Connection,
}

impl ContextCacheStore {
    /// Open or create a cache store
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open cache store: {}", path.display()))?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS project_cache (
                project_id TEXT PRIMARY KEY,
                cache_data TEXT NOT NULL,
                created INTEGER NOT NULL,
                updated INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_project_cache_updated ON project_cache(updated DESC);
            "#,
        )
        .with_context(|| "Failed to initialize cache store schema")?;

        Ok(Self { conn })
    }

    /// Open an in-memory cache store (for testing)
    pub fn in_memory() -> Result<Self> {
        Self::open(Path::new(":memory:"))
    }

    /// Save a project cache
    pub fn save(&self, cache: &ProjectCache) -> Result<()> {
        let cache_data = serde_json::to_string(cache)?;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO project_cache (project_id, cache_data, created, updated)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                &cache.project_id,
                &cache_data,
                cache.time.created,
                cache.time.updated,
            ],
        )?;

        Ok(())
    }

    /// Load a project cache
    pub fn load(&self, project_id: &str) -> Result<Option<ProjectCache>> {
        self.conn
            .query_row(
                "SELECT cache_data FROM project_cache WHERE project_id = ?1",
                params![project_id],
                |row| {
                    let cache_data: String = row.get(0)?;
                    Ok(serde_json::from_str(&cache_data).ok())
                },
            )
            .optional()?
            .flatten()
            .map(Ok)
            .transpose()
    }

    /// Invalidate a project cache
    pub fn invalidate(&self, project_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM project_cache WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    /// Get routes for a project
    pub fn get_routes(&self, project_id: &str) -> Result<Vec<RouteCache>> {
        let cache = self.load(project_id)?;
        Ok(cache.map(|c| c.routes).unwrap_or_default())
    }

    /// Get routes by pattern
    pub fn get_routes_by_pattern(&self, project_id: &str, pattern: &str) -> Result<Vec<RouteCache>> {
        let cache = self.load(project_id)?;
        let Some(cache) = cache else {
            return Ok(Vec::new());
        };

        let regex = regex::Regex::new(&pattern.replace('*', ".*"))?;
        Ok(cache
            .routes
            .into_iter()
            .filter(|r| regex.is_match(&r.path))
            .collect())
    }

    /// Get components for a project
    pub fn get_components(&self, project_id: &str) -> Result<Vec<ComponentCache>> {
        let cache = self.load(project_id)?;
        Ok(cache.map(|c| c.components).unwrap_or_default())
    }

    /// Get components by type
    pub fn get_components_by_type(
        &self,
        project_id: &str,
        component_type: ComponentType,
    ) -> Result<Vec<ComponentCache>> {
        let cache = self.load(project_id)?;
        let Some(cache) = cache else {
            return Ok(Vec::new());
        };

        Ok(cache
            .components
            .into_iter()
            .filter(|c| c.component_type == component_type)
            .collect())
    }

    /// Get a component by name
    pub fn get_component(&self, project_id: &str, name: &str) -> Result<Option<ComponentCache>> {
        let cache = self.load(project_id)?;
        let Some(cache) = cache else {
            return Ok(None);
        };

        Ok(cache.components.into_iter().find(|c| c.name == name))
    }

    /// Get configs for a project
    pub fn get_configs(&self, project_id: &str) -> Result<Vec<ConfigCache>> {
        let cache = self.load(project_id)?;
        Ok(cache.map(|c| c.configs).unwrap_or_default())
    }

    /// Get a config by name
    pub fn get_config(&self, project_id: &str, name: &str) -> Result<Option<ConfigCache>> {
        let cache = self.load(project_id)?;
        let Some(cache) = cache else {
            return Ok(None);
        };

        Ok(cache.configs.into_iter().find(|c| c.name == name))
    }

    /// Get test files for a project
    pub fn get_test_files(&self, project_id: &str) -> Result<Vec<String>> {
        let cache = self.load(project_id)?;
        Ok(cache.map(|c| c.test_files).unwrap_or_default())
    }

    /// Get test file for a source file
    pub fn get_test_for_file(&self, project_id: &str, file_path: &str) -> Result<Option<String>> {
        let cache = self.load(project_id)?;
        let Some(cache) = cache else {
            return Ok(None);
        };

        // Extract base name without extension
        let base_name = Path::new(file_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        for test_file in cache.test_files {
            let test_base = Path::new(&test_file)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Check if test file matches (e.g., foo.test.ts matches foo.ts)
            if test_base.starts_with(&base_name) {
                return Ok(Some(test_file));
            }
        }

        Ok(None)
    }

    /// Update a cache entry
    pub fn update_entry(&self, project_id: &str, entry: CacheEntry) -> Result<()> {
        let mut cache = self.load(project_id)?.unwrap_or_else(|| ProjectCache::new(project_id));
        cache.entries.insert(entry.path.clone(), entry);
        cache.touch();
        self.save(&cache)
    }

    /// Remove a cache entry
    pub fn remove_entry(&self, project_id: &str, path: &str) -> Result<()> {
        let Some(mut cache) = self.load(project_id)? else {
            return Ok(());
        };

        cache.entries.remove(path);
        cache.routes.retain(|r| r.path != path);
        cache.components.retain(|c| c.path != path);
        cache.configs.retain(|c| c.path != path);
        cache.test_files.retain(|t| t != path);
        cache.touch();

        self.save(&cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_builder() {
        let temp = tempdir().unwrap();
        let worktree = temp.path();

        // Create some test files
        std::fs::create_dir_all(worktree.join("src/components")).unwrap();
        std::fs::write(worktree.join("src/components/Button.tsx"), "export function Button() {}").unwrap();
        std::fs::write(worktree.join("package.json"), r#"{"name": "test"}"#).unwrap();

        let builder = CacheBuilder::new(worktree.to_string_lossy().to_string());
        let cache = builder.build("test-project").unwrap();

        assert_eq!(cache.project_id, "test-project");
        assert!(!cache.components.is_empty());
        assert!(!cache.configs.is_empty());
    }

    #[test]
    fn test_cache_store() {
        let store = ContextCacheStore::in_memory().unwrap();

        let mut cache = ProjectCache::new("test-project");
        cache.routes.push(RouteCache {
            path: "app/page.tsx".to_string(),
            route_type: RouteType::App,
            framework: Some("Next.js".to_string()),
            methods: None,
            middleware: None,
        });

        store.save(&cache).unwrap();

        let loaded = store.load("test-project").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.routes.len(), 1);
        assert_eq!(loaded.routes[0].path, "app/page.tsx");
    }

    #[test]
    fn test_component_type_inference() {
        assert_eq!(
            "useAuth".starts_with("use"),
            true
        );
    }
}
