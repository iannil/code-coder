//! NAPI bindings for context module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Project fingerprinting
//! - Content relevance scoring
//! - Project cache management

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::context::{
    fingerprint::{
        BuildToolInfo as RustBuildToolInfo, ConfigFile as RustConfigFile,
        Fingerprint, FingerprintInfo as RustFingerprintInfo, FrameworkInfo as RustFrameworkInfo,
        FrameworkType, PackageInfo as RustPackageInfo, PackageManager as RustPackageManager,
        ProjectLanguage as RustProjectLanguage, TestFrameworkInfo as RustTestFrameworkInfo,
        DirectoryInfo as RustDirectoryInfo,
    },
    relevance::{
        FileMetadata as RustFileMetadata, RelevanceScore as RustRelevanceScore,
        RelevanceScorer, RelevanceScorerConfig as RustRelevanceScorerConfig,
    },
    cache::{
        CacheBuilder, CacheEntry as RustCacheEntry, CacheEntryType as RustCacheEntryType,
        CacheTime as RustCacheTime, ComponentCache as RustComponentCache,
        ComponentType as RustComponentType, ConfigCache as RustConfigCache,
        ContextCacheStore as RustContextCacheStore, ProjectCache as RustProjectCache,
        RouteCache as RustRouteCache, RouteType as RustRouteType,
    },
};

// ============================================================================
// Fingerprint Types (NAPI)
// ============================================================================

/// Project language enum for NAPI
#[napi(string_enum)]
pub enum NapiProjectLanguage {
    TypeScript,
    JavaScript,
    Python,
    Go,
    Rust,
    Java,
    CSharp,
    Other,
}

impl From<RustProjectLanguage> for NapiProjectLanguage {
    fn from(lang: RustProjectLanguage) -> Self {
        match lang {
            RustProjectLanguage::TypeScript => NapiProjectLanguage::TypeScript,
            RustProjectLanguage::JavaScript => NapiProjectLanguage::JavaScript,
            RustProjectLanguage::Python => NapiProjectLanguage::Python,
            RustProjectLanguage::Go => NapiProjectLanguage::Go,
            RustProjectLanguage::Rust => NapiProjectLanguage::Rust,
            RustProjectLanguage::Java => NapiProjectLanguage::Java,
            RustProjectLanguage::CSharp => NapiProjectLanguage::CSharp,
            RustProjectLanguage::Other => NapiProjectLanguage::Other,
        }
    }
}

/// Package manager enum for NAPI
#[napi(string_enum)]
pub enum NapiPackageManager {
    Npm,
    Bun,
    Yarn,
    Pnpm,
    Pip,
    Poetry,
    Cargo,
    Go,
    Maven,
    Gradle,
    Nuget,
    Unknown,
}

impl From<RustPackageManager> for NapiPackageManager {
    fn from(pm: RustPackageManager) -> Self {
        match pm {
            RustPackageManager::Npm => NapiPackageManager::Npm,
            RustPackageManager::Bun => NapiPackageManager::Bun,
            RustPackageManager::Yarn => NapiPackageManager::Yarn,
            RustPackageManager::Pnpm => NapiPackageManager::Pnpm,
            RustPackageManager::Pip => NapiPackageManager::Pip,
            RustPackageManager::Poetry => NapiPackageManager::Poetry,
            RustPackageManager::Cargo => NapiPackageManager::Cargo,
            RustPackageManager::Go => NapiPackageManager::Go,
            RustPackageManager::Maven => NapiPackageManager::Maven,
            RustPackageManager::Gradle => NapiPackageManager::Gradle,
            RustPackageManager::Nuget => NapiPackageManager::Nuget,
            RustPackageManager::Unknown => NapiPackageManager::Unknown,
        }
    }
}

/// Framework type enum for NAPI
#[napi(string_enum)]
pub enum NapiFrameworkType {
    Frontend,
    Backend,
    Fullstack,
    Mobile,
    Desktop,
    Cli,
    Library,
}

impl From<FrameworkType> for NapiFrameworkType {
    fn from(ft: FrameworkType) -> Self {
        match ft {
            FrameworkType::Frontend => NapiFrameworkType::Frontend,
            FrameworkType::Backend => NapiFrameworkType::Backend,
            FrameworkType::Fullstack => NapiFrameworkType::Fullstack,
            FrameworkType::Mobile => NapiFrameworkType::Mobile,
            FrameworkType::Desktop => NapiFrameworkType::Desktop,
            FrameworkType::Cli => NapiFrameworkType::Cli,
            FrameworkType::Library => NapiFrameworkType::Library,
        }
    }
}

/// Framework info for NAPI
#[napi(object)]
pub struct NapiFrameworkInfo {
    pub name: String,
    pub version: Option<String>,
    pub framework_type: NapiFrameworkType,
}

impl From<RustFrameworkInfo> for NapiFrameworkInfo {
    fn from(info: RustFrameworkInfo) -> Self {
        Self {
            name: info.name,
            version: info.version,
            framework_type: info.framework_type.into(),
        }
    }
}

/// Build tool info for NAPI
#[napi(object)]
pub struct NapiBuildToolInfo {
    pub name: String,
    pub config: Option<String>,
    pub version: Option<String>,
}

impl From<RustBuildToolInfo> for NapiBuildToolInfo {
    fn from(info: RustBuildToolInfo) -> Self {
        Self {
            name: info.name,
            config: info.config,
            version: info.version,
        }
    }
}

/// Test framework info for NAPI
#[napi(object)]
pub struct NapiTestFrameworkInfo {
    pub name: String,
    pub config: Option<String>,
    pub runner: Option<String>,
}

impl From<RustTestFrameworkInfo> for NapiTestFrameworkInfo {
    fn from(info: RustTestFrameworkInfo) -> Self {
        Self {
            name: info.name,
            config: info.config,
            runner: info.runner,
        }
    }
}

/// Package info for NAPI
#[napi(object)]
pub struct NapiPackageInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub manager: NapiPackageManager,
}

impl From<RustPackageInfo> for NapiPackageInfo {
    fn from(info: RustPackageInfo) -> Self {
        Self {
            name: info.name,
            version: info.version,
            manager: info.manager.into(),
        }
    }
}

/// Config file for NAPI
#[napi(object)]
pub struct NapiConfigFile {
    pub path: String,
    pub name: String,
}

impl From<RustConfigFile> for NapiConfigFile {
    fn from(info: RustConfigFile) -> Self {
        Self {
            path: info.path,
            name: info.name,
        }
    }
}

/// Directory info for NAPI
#[napi(object)]
pub struct NapiDirectoryInfo {
    pub src: Option<String>,
    pub components: Option<String>,
    pub pages: Option<String>,
    pub routes: Option<String>,
    pub tests: Vec<String>,
    pub lib: Option<String>,
    pub dist: Option<String>,
    pub build: Option<String>,
    pub public: Option<String>,
}

impl From<RustDirectoryInfo> for NapiDirectoryInfo {
    fn from(info: RustDirectoryInfo) -> Self {
        Self {
            src: info.src,
            components: info.components,
            pages: info.pages,
            routes: info.routes,
            tests: info.tests,
            lib: info.lib,
            dist: info.dist,
            build: info.build,
            public: info.public,
        }
    }
}

/// Full fingerprint info for NAPI
#[napi(object)]
pub struct NapiFingerprintInfo {
    pub project_id: String,
    pub frameworks: Vec<NapiFrameworkInfo>,
    pub build_tools: Vec<NapiBuildToolInfo>,
    pub test_frameworks: Vec<NapiTestFrameworkInfo>,
    pub package: NapiPackageInfo,
    pub configs: Vec<NapiConfigFile>,
    pub language: NapiProjectLanguage,
    pub has_typescript: bool,
    pub language_version: Option<String>,
    pub directories: NapiDirectoryInfo,
    pub hash: String,
}

impl From<RustFingerprintInfo> for NapiFingerprintInfo {
    fn from(info: RustFingerprintInfo) -> Self {
        Self {
            project_id: info.project_id,
            frameworks: info.frameworks.into_iter().map(|f| f.into()).collect(),
            build_tools: info.build_tools.into_iter().map(|t| t.into()).collect(),
            test_frameworks: info.test_frameworks.into_iter().map(|t| t.into()).collect(),
            package: info.package.into(),
            configs: info.configs.into_iter().map(|c| c.into()).collect(),
            language: info.language.into(),
            has_typescript: info.has_typescript,
            language_version: info.language_version,
            directories: info.directories.into(),
            hash: info.hash,
        }
    }
}

// ============================================================================
// Relevance Types (NAPI)
// ============================================================================

/// Relevance score for NAPI
#[napi(object)]
pub struct NapiRelevanceScore {
    pub score: f64,
    pub keyword_score: f64,
    pub structural_score: f64,
    pub recency_score: f64,
    pub matched_keywords: Vec<String>,
}

impl From<RustRelevanceScore> for NapiRelevanceScore {
    fn from(score: RustRelevanceScore) -> Self {
        Self {
            score: score.score,
            keyword_score: score.keyword_score,
            structural_score: score.structural_score,
            recency_score: score.recency_score,
            matched_keywords: score.matched_keywords,
        }
    }
}

/// Relevance scorer config for NAPI
#[napi(object)]
pub struct NapiRelevanceScorerConfig {
    pub keyword_weight: f64,
    pub structural_weight: f64,
    pub recency_weight: f64,
    pub min_score: f64,
    pub case_insensitive: bool,
}

impl Default for NapiRelevanceScorerConfig {
    fn default() -> Self {
        Self {
            keyword_weight: 0.5,
            structural_weight: 0.3,
            recency_weight: 0.2,
            min_score: 0.1,
            case_insensitive: true,
        }
    }
}

impl From<NapiRelevanceScorerConfig> for RustRelevanceScorerConfig {
    fn from(config: NapiRelevanceScorerConfig) -> Self {
        Self {
            keyword_weight: config.keyword_weight,
            structural_weight: config.structural_weight,
            recency_weight: config.recency_weight,
            min_score: config.min_score,
            case_insensitive: config.case_insensitive,
        }
    }
}

/// File metadata for scoring
#[napi(object)]
pub struct NapiFileMetadata {
    pub path: String,
    pub content: String,
    pub modified: Option<u32>,
    pub extension: Option<String>,
}

impl From<NapiFileMetadata> for RustFileMetadata {
    fn from(meta: NapiFileMetadata) -> Self {
        Self {
            path: meta.path,
            content: meta.content,
            modified: meta.modified.map(|m| m as u64),
            extension: meta.extension,
        }
    }
}

/// Scored file result
#[napi(object)]
pub struct NapiScoredFile {
    pub path: String,
    pub score: NapiRelevanceScore,
}

// ============================================================================
// NAPI Functions
// ============================================================================

/// Generate fingerprint for a project directory
#[napi]
pub fn generate_fingerprint(root_path: String) -> Result<NapiFingerprintInfo> {
    let path = Path::new(&root_path);

    if !path.exists() {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Path does not exist: {}", root_path),
        ));
    }

    if !path.is_dir() {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Path is not a directory: {}", root_path),
        ));
    }

    Fingerprint::generate(path)
        .map(|info| info.into())
        .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to generate fingerprint: {}", e)))
}

/// Compute similarity between two fingerprints
#[napi]
pub fn fingerprint_similarity(a: NapiFingerprintInfo, b: NapiFingerprintInfo) -> f64 {
    // Convert NAPI types back to Rust types
    let rust_a = convert_to_rust_fingerprint(a);
    let rust_b = convert_to_rust_fingerprint(b);

    Fingerprint::similarity(&rust_a, &rust_b)
}

/// Generate human-readable description of a fingerprint
#[napi]
pub fn describe_fingerprint(fingerprint: NapiFingerprintInfo) -> String {
    let rust_fp = convert_to_rust_fingerprint(fingerprint);
    Fingerprint::describe(&rust_fp)
}

/// Score content relevance against a query
#[napi]
pub fn score_relevance(query: String, content: String) -> NapiRelevanceScore {
    crate::context::relevance::score_relevance(&query, &content).into()
}

/// Score content relevance with custom config
#[napi]
pub fn score_relevance_with_config(
    query: String,
    content: String,
    config: NapiRelevanceScorerConfig,
) -> NapiRelevanceScore {
    let mut scorer = RelevanceScorer::with_config(config.into());
    scorer.set_query(&query);
    scorer
        .score(&RustFileMetadata {
            path: String::new(),
            content,
            modified: None,
            extension: None,
        })
        .into()
}

/// Score multiple files and return sorted by relevance
#[napi]
pub fn score_files(query: String, files: Vec<NapiFileMetadata>) -> Vec<NapiScoredFile> {
    let mut scorer = RelevanceScorer::new();
    scorer.set_query(&query);

    let rust_files: Vec<RustFileMetadata> = files.into_iter().map(|f| f.into()).collect();
    let scored = scorer.score_files(&rust_files);

    scored
        .into_iter()
        .map(|(file, score)| NapiScoredFile {
            path: file.path,
            score: score.into(),
        })
        .collect()
}

/// Compute content hash for deduplication
#[napi]
pub fn content_hash(content: String) -> String {
    RelevanceScorer::content_hash(&content)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn convert_to_rust_fingerprint(napi: NapiFingerprintInfo) -> RustFingerprintInfo {
    RustFingerprintInfo {
        project_id: napi.project_id,
        frameworks: napi
            .frameworks
            .into_iter()
            .map(|f| RustFrameworkInfo {
                name: f.name,
                version: f.version,
                framework_type: match f.framework_type {
                    NapiFrameworkType::Frontend => FrameworkType::Frontend,
                    NapiFrameworkType::Backend => FrameworkType::Backend,
                    NapiFrameworkType::Fullstack => FrameworkType::Fullstack,
                    NapiFrameworkType::Mobile => FrameworkType::Mobile,
                    NapiFrameworkType::Desktop => FrameworkType::Desktop,
                    NapiFrameworkType::Cli => FrameworkType::Cli,
                    NapiFrameworkType::Library => FrameworkType::Library,
                },
            })
            .collect(),
        build_tools: napi
            .build_tools
            .into_iter()
            .map(|t| RustBuildToolInfo {
                name: t.name,
                config: t.config,
                version: t.version,
            })
            .collect(),
        test_frameworks: napi
            .test_frameworks
            .into_iter()
            .map(|t| RustTestFrameworkInfo {
                name: t.name,
                config: t.config,
                runner: t.runner,
            })
            .collect(),
        package: RustPackageInfo {
            name: napi.package.name,
            version: napi.package.version,
            manager: match napi.package.manager {
                NapiPackageManager::Npm => RustPackageManager::Npm,
                NapiPackageManager::Bun => RustPackageManager::Bun,
                NapiPackageManager::Yarn => RustPackageManager::Yarn,
                NapiPackageManager::Pnpm => RustPackageManager::Pnpm,
                NapiPackageManager::Pip => RustPackageManager::Pip,
                NapiPackageManager::Poetry => RustPackageManager::Poetry,
                NapiPackageManager::Cargo => RustPackageManager::Cargo,
                NapiPackageManager::Go => RustPackageManager::Go,
                NapiPackageManager::Maven => RustPackageManager::Maven,
                NapiPackageManager::Gradle => RustPackageManager::Gradle,
                NapiPackageManager::Nuget => RustPackageManager::Nuget,
                NapiPackageManager::Unknown => RustPackageManager::Unknown,
            },
        },
        configs: napi
            .configs
            .into_iter()
            .map(|c| RustConfigFile {
                path: c.path,
                name: c.name,
            })
            .collect(),
        language: match napi.language {
            NapiProjectLanguage::TypeScript => RustProjectLanguage::TypeScript,
            NapiProjectLanguage::JavaScript => RustProjectLanguage::JavaScript,
            NapiProjectLanguage::Python => RustProjectLanguage::Python,
            NapiProjectLanguage::Go => RustProjectLanguage::Go,
            NapiProjectLanguage::Rust => RustProjectLanguage::Rust,
            NapiProjectLanguage::Java => RustProjectLanguage::Java,
            NapiProjectLanguage::CSharp => RustProjectLanguage::CSharp,
            NapiProjectLanguage::Other => RustProjectLanguage::Other,
        },
        has_typescript: napi.has_typescript,
        language_version: napi.language_version,
        directories: RustDirectoryInfo {
            src: napi.directories.src,
            components: napi.directories.components,
            pages: napi.directories.pages,
            routes: napi.directories.routes,
            tests: napi.directories.tests,
            lib: napi.directories.lib,
            dist: napi.directories.dist,
            build: napi.directories.build,
            public: napi.directories.public,
        },
        hash: napi.hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_language_conversion() {
        let napi: NapiProjectLanguage = RustProjectLanguage::TypeScript.into();
        assert!(matches!(napi, NapiProjectLanguage::TypeScript));
    }

    #[test]
    fn test_package_manager_conversion() {
        let napi: NapiPackageManager = RustPackageManager::Bun.into();
        assert!(matches!(napi, NapiPackageManager::Bun));
    }

    #[test]
    fn test_score_relevance() {
        let score = score_relevance(
            "authentication login".to_string(),
            "User authentication and login handler".to_string(),
        );
        assert!(score.score > 0.0);
    }

    #[test]
    fn test_content_hash() {
        let hash1 = content_hash("hello world".to_string());
        let hash2 = content_hash("hello world".to_string());
        assert_eq!(hash1, hash2);
    }
}

// ============================================================================
// Context Cache Types (NAPI)
// ============================================================================

/// Cache entry type for NAPI
#[napi(string_enum)]
pub enum NapiCacheEntryType {
    File,
    Directory,
    Config,
    Route,
    Component,
    Test,
}

impl From<RustCacheEntryType> for NapiCacheEntryType {
    fn from(t: RustCacheEntryType) -> Self {
        match t {
            RustCacheEntryType::File => NapiCacheEntryType::File,
            RustCacheEntryType::Directory => NapiCacheEntryType::Directory,
            RustCacheEntryType::Config => NapiCacheEntryType::Config,
            RustCacheEntryType::Route => NapiCacheEntryType::Route,
            RustCacheEntryType::Component => NapiCacheEntryType::Component,
            RustCacheEntryType::Test => NapiCacheEntryType::Test,
        }
    }
}

impl From<NapiCacheEntryType> for RustCacheEntryType {
    fn from(t: NapiCacheEntryType) -> Self {
        match t {
            NapiCacheEntryType::File => RustCacheEntryType::File,
            NapiCacheEntryType::Directory => RustCacheEntryType::Directory,
            NapiCacheEntryType::Config => RustCacheEntryType::Config,
            NapiCacheEntryType::Route => RustCacheEntryType::Route,
            NapiCacheEntryType::Component => RustCacheEntryType::Component,
            NapiCacheEntryType::Test => RustCacheEntryType::Test,
        }
    }
}

/// Route type for NAPI
#[napi(string_enum)]
pub enum NapiRouteType {
    File,
    Directory,
    App,
    Pages,
    Api,
}

impl From<RustRouteType> for NapiRouteType {
    fn from(t: RustRouteType) -> Self {
        match t {
            RustRouteType::File => NapiRouteType::File,
            RustRouteType::Directory => NapiRouteType::Directory,
            RustRouteType::App => NapiRouteType::App,
            RustRouteType::Pages => NapiRouteType::Pages,
            RustRouteType::Api => NapiRouteType::Api,
        }
    }
}

/// Component type for NAPI
#[napi(string_enum)]
pub enum NapiComponentType {
    Component,
    Hook,
    Util,
    Layout,
    Page,
}

impl From<RustComponentType> for NapiComponentType {
    fn from(t: RustComponentType) -> Self {
        match t {
            RustComponentType::Component => NapiComponentType::Component,
            RustComponentType::Hook => NapiComponentType::Hook,
            RustComponentType::Util => NapiComponentType::Util,
            RustComponentType::Layout => NapiComponentType::Layout,
            RustComponentType::Page => NapiComponentType::Page,
        }
    }
}

impl From<NapiComponentType> for RustComponentType {
    fn from(t: NapiComponentType) -> Self {
        match t {
            NapiComponentType::Component => RustComponentType::Component,
            NapiComponentType::Hook => RustComponentType::Hook,
            NapiComponentType::Util => RustComponentType::Util,
            NapiComponentType::Layout => RustComponentType::Layout,
            NapiComponentType::Page => RustComponentType::Page,
        }
    }
}

/// Cache entry for NAPI
#[napi(object)]
pub struct NapiCacheEntry {
    pub path: String,
    #[napi(js_name = "type")]
    pub entry_type: String,
    pub last_modified: i64,
    pub size: i64,
    pub hash: Option<String>,
}

impl From<RustCacheEntry> for NapiCacheEntry {
    fn from(e: RustCacheEntry) -> Self {
        Self {
            path: e.path,
            entry_type: e.entry_type.to_string(),
            last_modified: e.last_modified,
            size: e.size as i64,
            hash: e.hash,
        }
    }
}

/// Route cache entry for NAPI
#[napi(object)]
pub struct NapiRouteCache {
    pub path: String,
    #[napi(js_name = "type")]
    pub route_type: String,
    pub framework: Option<String>,
    pub methods: Option<Vec<String>>,
    pub middleware: Option<String>,
}

impl From<RustRouteCache> for NapiRouteCache {
    fn from(r: RustRouteCache) -> Self {
        Self {
            path: r.path,
            route_type: r.route_type.to_string(),
            framework: r.framework,
            methods: r.methods,
            middleware: r.middleware,
        }
    }
}

/// Component cache entry for NAPI
#[napi(object)]
pub struct NapiComponentCache {
    pub path: String,
    pub name: String,
    #[napi(js_name = "type")]
    pub component_type: String,
    pub props: Option<Vec<String>>,
    pub imports: Option<Vec<String>>,
}

impl From<RustComponentCache> for NapiComponentCache {
    fn from(c: RustComponentCache) -> Self {
        Self {
            path: c.path,
            name: c.name,
            component_type: c.component_type.to_string(),
            props: c.props,
            imports: c.imports,
        }
    }
}

/// Config cache entry for NAPI
#[napi(object)]
pub struct NapiCacheConfigEntry {
    pub path: String,
    pub name: String,
    #[napi(js_name = "type")]
    pub config_type: String,
    pub content: Option<String>,
}

impl From<RustConfigCache> for NapiCacheConfigEntry {
    fn from(c: RustConfigCache) -> Self {
        Self {
            path: c.path,
            name: c.name,
            config_type: c.config_type,
            content: c.content,
        }
    }
}

/// Cache timestamps for NAPI
#[napi(object)]
pub struct NapiCacheTime {
    pub created: i64,
    pub updated: i64,
}

impl From<RustCacheTime> for NapiCacheTime {
    fn from(t: RustCacheTime) -> Self {
        Self {
            created: t.created,
            updated: t.updated,
        }
    }
}

/// Full project cache for NAPI
#[napi(object)]
pub struct NapiProjectCache {
    pub project_id: String,
    pub routes: Vec<NapiRouteCache>,
    pub components: Vec<NapiComponentCache>,
    pub configs: Vec<NapiCacheConfigEntry>,
    pub test_files: Vec<String>,
    pub time: NapiCacheTime,
}

impl From<RustProjectCache> for NapiProjectCache {
    fn from(c: RustProjectCache) -> Self {
        Self {
            project_id: c.project_id,
            routes: c.routes.into_iter().map(Into::into).collect(),
            components: c.components.into_iter().map(Into::into).collect(),
            configs: c.configs.into_iter().map(Into::into).collect(),
            test_files: c.test_files,
            time: c.time.into(),
        }
    }
}

// ============================================================================
// Context Cache Store Handle
// ============================================================================

/// Handle to a ContextCacheStore
#[napi]
pub struct ContextCacheStoreHandle {
    inner: Arc<Mutex<RustContextCacheStore>>,
}

/// Open or create a context cache store
#[napi]
pub fn open_context_cache_store(path: String) -> Result<ContextCacheStoreHandle> {
    let store = RustContextCacheStore::open(Path::new(&path))
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(ContextCacheStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

/// Create an in-memory context cache store (for testing)
#[napi]
pub fn create_memory_context_cache_store() -> Result<ContextCacheStoreHandle> {
    let store = RustContextCacheStore::in_memory()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(ContextCacheStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

/// Build a project cache by scanning the worktree
#[napi]
pub fn build_project_cache(
    worktree: String,
    project_id: String,
    framework: Option<String>,
) -> Result<NapiProjectCache> {
    let mut builder = CacheBuilder::new(&worktree);
    if let Some(fw) = framework {
        builder = builder.with_framework(fw);
    }

    let cache = builder
        .build(&project_id)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(cache.into())
}

#[napi]
impl ContextCacheStoreHandle {
    /// Save a project cache
    #[napi]
    pub fn save(&self, cache: NapiProjectCache) -> Result<()> {
        let rust_cache = convert_napi_cache_to_rust(cache);
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.save(&rust_cache).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load a project cache
    #[napi]
    pub fn load(&self, project_id: String) -> Result<Option<NapiProjectCache>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let cache = store.load(&project_id).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(cache.map(Into::into))
    }

    /// Invalidate a project cache
    #[napi]
    pub fn invalidate(&self, project_id: String) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.invalidate(&project_id).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get routes for a project
    #[napi]
    pub fn get_routes(&self, project_id: String) -> Result<Vec<NapiRouteCache>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let routes = store.get_routes(&project_id).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(routes.into_iter().map(Into::into).collect())
    }

    /// Get routes matching a pattern
    #[napi]
    pub fn get_routes_by_pattern(&self, project_id: String, pattern: String) -> Result<Vec<NapiRouteCache>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let routes = store.get_routes_by_pattern(&project_id, &pattern)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(routes.into_iter().map(Into::into).collect())
    }

    /// Get components for a project
    #[napi]
    pub fn get_components(&self, project_id: String) -> Result<Vec<NapiComponentCache>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let components = store.get_components(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(components.into_iter().map(Into::into).collect())
    }

    /// Get components by type
    #[napi]
    pub fn get_components_by_type(&self, project_id: String, component_type: String) -> Result<Vec<NapiComponentCache>> {
        let ct: RustComponentType = component_type.parse()
            .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?;
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let components = store.get_components_by_type(&project_id, ct)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(components.into_iter().map(Into::into).collect())
    }

    /// Get a component by name
    #[napi]
    pub fn get_component(&self, project_id: String, name: String) -> Result<Option<NapiComponentCache>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let component = store.get_component(&project_id, &name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(component.map(Into::into))
    }

    /// Get configs for a project
    #[napi]
    pub fn get_configs(&self, project_id: String) -> Result<Vec<NapiCacheConfigEntry>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let configs = store.get_configs(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(configs.into_iter().map(Into::into).collect())
    }

    /// Get a config by name
    #[napi]
    pub fn get_config(&self, project_id: String, name: String) -> Result<Option<NapiCacheConfigEntry>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let config = store.get_config(&project_id, &name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(config.map(Into::into))
    }

    /// Get test files for a project
    #[napi]
    pub fn get_test_files(&self, project_id: String) -> Result<Vec<String>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.get_test_files(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get test file for a source file
    #[napi]
    pub fn get_test_for_file(&self, project_id: String, file_path: String) -> Result<Option<String>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.get_test_for_file(&project_id, &file_path)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Remove a cache entry
    #[napi]
    pub fn remove_entry(&self, project_id: String, path: String) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.remove_entry(&project_id, &path)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

/// Convert NAPI cache to Rust cache
fn convert_napi_cache_to_rust(napi: NapiProjectCache) -> RustProjectCache {
    let mut cache = RustProjectCache::new(&napi.project_id);
    cache.time = RustCacheTime {
        created: napi.time.created,
        updated: napi.time.updated,
    };
    cache.routes = napi.routes.into_iter().map(|r| RustRouteCache {
        path: r.path,
        route_type: r.route_type.parse().unwrap_or(RustRouteType::File),
        framework: r.framework,
        methods: r.methods,
        middleware: r.middleware,
    }).collect();
    cache.components = napi.components.into_iter().map(|c| RustComponentCache {
        path: c.path,
        name: c.name,
        component_type: c.component_type.parse().unwrap_or(RustComponentType::Component),
        props: c.props,
        imports: c.imports,
    }).collect();
    cache.configs = napi.configs.into_iter().map(|c| RustConfigCache {
        path: c.path,
        name: c.name,
        config_type: c.config_type,
        content: c.content,
        parsed: None,
    }).collect();
    cache.test_files = napi.test_files;
    cache
}

// ============================================================================
// Context Loader Types (NAPI)
// ============================================================================

use crate::context::loader::{
    ContextLoader, DependencyGraph as RustDependencyGraph,
    DirectoryStructure as RustDirectoryStructure, FileEntry as RustFileEntry,
    FileIndex as RustFileIndex, ScanOptions as RustScanOptions,
};

/// File entry for NAPI
#[napi(object)]
pub struct NapiFileEntry {
    pub path: String,
    #[napi(js_name = "relativePath")]
    pub relative_path: String,
    pub name: String,
    pub extension: Option<String>,
    pub directory: bool,
    pub size: i64,
    #[napi(js_name = "lastModified")]
    pub last_modified: i64,
}

impl From<RustFileEntry> for NapiFileEntry {
    fn from(e: RustFileEntry) -> Self {
        Self {
            path: e.path,
            relative_path: e.relative_path,
            name: e.name,
            extension: e.extension,
            directory: e.directory,
            size: e.size as i64,
            last_modified: e.last_modified as i64,
        }
    }
}

/// Directory structure for NAPI
#[napi(object)]
pub struct NapiDirectoryStructure {
    pub path: String,
    pub name: String,
    pub files: Vec<String>,
    pub subdirectories: Vec<NapiDirectoryStructure>,
}

impl From<RustDirectoryStructure> for NapiDirectoryStructure {
    fn from(s: RustDirectoryStructure) -> Self {
        Self {
            path: s.path,
            name: s.name,
            files: s.files,
            subdirectories: s.subdirectories.into_iter().map(Into::into).collect(),
        }
    }
}

/// File index for NAPI
#[napi(object)]
pub struct NapiFileIndex {
    #[napi(js_name = "byPath")]
    pub by_path: HashMap<String, NapiFileEntry>,
    #[napi(js_name = "byExtension")]
    pub by_extension: HashMap<String, Vec<String>>,
    #[napi(js_name = "byName")]
    pub by_name: HashMap<String, Vec<String>>,
    pub routes: Vec<String>,
    pub components: Vec<String>,
    pub tests: Vec<String>,
    pub configs: Vec<String>,
}

impl From<RustFileIndex> for NapiFileIndex {
    fn from(i: RustFileIndex) -> Self {
        Self {
            by_path: i.by_path.into_iter().map(|(k, v)| (k, v.into())).collect(),
            by_extension: i.by_extension,
            by_name: i.by_name,
            routes: i.routes,
            components: i.components,
            tests: i.tests,
            configs: i.configs,
        }
    }
}

/// Dependency graph for NAPI
#[napi(object)]
pub struct NapiDependencyGraph {
    pub imports: HashMap<String, Vec<String>>,
    #[napi(js_name = "importedBy")]
    pub imported_by: HashMap<String, Vec<String>>,
}

impl From<RustDependencyGraph> for NapiDependencyGraph {
    fn from(g: RustDependencyGraph) -> Self {
        Self {
            imports: g.imports,
            imported_by: g.imported_by,
        }
    }
}

/// Scan options for NAPI
#[napi(object)]
pub struct NapiScanOptions {
    #[napi(js_name = "maxDepth")]
    pub max_depth: Option<u32>,
    #[napi(js_name = "includeHidden")]
    pub include_hidden: Option<bool>,
    #[napi(js_name = "ignorePatterns")]
    pub ignore_patterns: Option<Vec<String>>,
}

impl From<NapiScanOptions> for RustScanOptions {
    fn from(o: NapiScanOptions) -> Self {
        Self {
            max_depth: o.max_depth.unwrap_or(10),
            include_hidden: o.include_hidden.unwrap_or(false),
            ignore_patterns: o.ignore_patterns.unwrap_or_default(),
        }
    }
}

/// Scan result for NAPI
#[napi(object)]
pub struct NapiScanResult {
    pub entries: Vec<NapiFileEntry>,
    pub structure: NapiDirectoryStructure,
}

/// Context loader handle for NAPI
#[napi]
pub struct ContextLoaderHandle {
    inner: ContextLoader,
    fingerprint: Option<RustFingerprintInfo>,
}

#[napi]
impl ContextLoaderHandle {
    /// Create a new context loader for the given directory
    #[napi(constructor)]
    pub fn new(root: String, options: Option<NapiScanOptions>) -> Self {
        let opts = options.map(Into::into).unwrap_or_default();
        Self {
            inner: ContextLoader::with_options(&root, opts),
            fingerprint: None,
        }
    }

    /// Scan the directory and return file entries and structure
    #[napi]
    pub fn scan(&self) -> Result<NapiScanResult> {
        let (entries, structure) = self.inner.scan()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(NapiScanResult {
            entries: entries.into_iter().map(Into::into).collect(),
            structure: structure.into(),
        })
    }

    /// Set fingerprint for categorization
    #[napi]
    pub fn set_fingerprint(&mut self, fingerprint: NapiFingerprintInfo) {
        self.fingerprint = Some(convert_to_rust_fingerprint(fingerprint));
    }

    /// Categorize files based on fingerprint
    #[napi]
    pub fn categorize(&self, entries: Vec<NapiFileEntry>) -> Result<NapiFileIndex> {
        let fingerprint = self.fingerprint.as_ref()
            .ok_or_else(|| Error::from_reason("Fingerprint not set. Call setFingerprint first."))?;

        let rust_entries: Vec<RustFileEntry> = entries.into_iter()
            .map(|e| RustFileEntry {
                path: e.path,
                relative_path: e.relative_path,
                name: e.name,
                extension: e.extension,
                directory: e.directory,
                size: e.size as u64,
                last_modified: e.last_modified as u64,
            })
            .collect();

        let index = self.inner.categorize_files(&rust_entries, fingerprint);
        Ok(index.into())
    }

    /// Extract import dependencies from source files
    #[napi]
    pub fn extract_dependencies(&self, entries: Vec<NapiFileEntry>, language: NapiProjectLanguage) -> NapiDependencyGraph {
        let rust_entries: Vec<RustFileEntry> = entries.into_iter()
            .map(|e| RustFileEntry {
                path: e.path,
                relative_path: e.relative_path,
                name: e.name,
                extension: e.extension,
                directory: e.directory,
                size: e.size as u64,
                last_modified: e.last_modified as u64,
            })
            .collect();

        let lang = match language {
            NapiProjectLanguage::TypeScript => RustProjectLanguage::TypeScript,
            NapiProjectLanguage::JavaScript => RustProjectLanguage::JavaScript,
            NapiProjectLanguage::Python => RustProjectLanguage::Python,
            NapiProjectLanguage::Go => RustProjectLanguage::Go,
            NapiProjectLanguage::Rust => RustProjectLanguage::Rust,
            NapiProjectLanguage::Java => RustProjectLanguage::Java,
            NapiProjectLanguage::CSharp => RustProjectLanguage::CSharp,
            NapiProjectLanguage::Other => RustProjectLanguage::Other,
        };

        let graph = self.inner.extract_imports(&rust_entries, lang);
        graph.into()
    }

    /// Find files related to a given file
    #[napi]
    pub fn find_related_files(
        &self,
        file_path: String,
        index: NapiFileIndex,
        dependencies: NapiDependencyGraph,
    ) -> Vec<String> {
        let rust_index = RustFileIndex {
            by_path: index.by_path.into_iter()
                .map(|(k, v)| (k, RustFileEntry {
                    path: v.path,
                    relative_path: v.relative_path,
                    name: v.name,
                    extension: v.extension,
                    directory: v.directory,
                    size: v.size as u64,
                    last_modified: v.last_modified as u64,
                }))
                .collect(),
            by_extension: index.by_extension,
            by_name: index.by_name,
            routes: index.routes,
            components: index.components,
            tests: index.tests,
            configs: index.configs,
        };

        let rust_deps = RustDependencyGraph {
            imports: dependencies.imports,
            imported_by: dependencies.imported_by,
        };

        self.inner.find_related_files(&file_path, &rust_index, &rust_deps)
    }
}

/// Create a context loader (convenience function)
#[napi]
pub fn create_context_loader(root: String, options: Option<NapiScanOptions>) -> ContextLoaderHandle {
    ContextLoaderHandle::new(root, options)
}

/// Scan a directory and return all file entries (convenience function)
#[napi]
pub fn scan_directory(root: String, options: Option<NapiScanOptions>) -> Result<NapiScanResult> {
    let opts = options.map(Into::into).unwrap_or_default();
    let loader = ContextLoader::with_options(&root, opts);
    let (entries, structure) = loader.scan()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(NapiScanResult {
        entries: entries.into_iter().map(Into::into).collect(),
        structure: structure.into(),
    })
}

/// Extract dependencies from a directory (convenience function)
#[napi]
pub fn extract_directory_dependencies(
    root: String,
    language: NapiProjectLanguage,
    options: Option<NapiScanOptions>,
) -> Result<NapiDependencyGraph> {
    let opts = options.map(Into::into).unwrap_or_default();
    let loader = ContextLoader::with_options(&root, opts);

    let (entries, _) = loader.scan()
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let lang = match language {
        NapiProjectLanguage::TypeScript => RustProjectLanguage::TypeScript,
        NapiProjectLanguage::JavaScript => RustProjectLanguage::JavaScript,
        NapiProjectLanguage::Python => RustProjectLanguage::Python,
        NapiProjectLanguage::Go => RustProjectLanguage::Go,
        NapiProjectLanguage::Rust => RustProjectLanguage::Rust,
        NapiProjectLanguage::Java => RustProjectLanguage::Java,
        NapiProjectLanguage::CSharp => RustProjectLanguage::CSharp,
        NapiProjectLanguage::Other => RustProjectLanguage::Other,
    };

    let graph = loader.extract_imports(&entries, lang);
    Ok(graph.into())
}
