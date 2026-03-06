//! Context module - code fingerprinting, relevance scoring, caching, and loading
//!
//! This module provides:
//! - **fingerprint**: Project fingerprinting and tech stack detection
//! - **relevance**: Content relevance scoring
//! - **cache**: Project structure caching (routes, components, configs)
//! - **loader**: High-performance project scanning and analysis

pub mod cache;
pub mod fingerprint;
pub mod loader;
pub mod relevance;

// Re-export main types
pub use fingerprint::{
    BuildToolInfo, ConfigFile, DirectoryInfo, Fingerprint, FingerprintInfo, FrameworkInfo,
    PackageInfo, PackageManager, ProjectLanguage, TestFrameworkInfo,
};
pub use relevance::{RelevanceScore, RelevanceScorer, RelevanceScorerConfig};
pub use cache::{
    CacheBuilder, CacheEntry, CacheEntryType, CacheTime, ComponentCache, ComponentType,
    ConfigCache, ContextCacheStore, ProjectCache, RouteCache, RouteType,
};
pub use loader::{
    ContextLoader, DependencyGraph, DirectoryStructure, FileEntry, FileIndex,
    ImportInfo, ImportType, ScanOptions,
};
