//! Context module - code fingerprinting, relevance scoring, and caching
//!
//! This module provides:
//! - **fingerprint**: Project fingerprinting and tech stack detection
//! - **relevance**: Content relevance scoring
//! - **cache**: Project structure caching (routes, components, configs)

pub mod cache;
pub mod fingerprint;
pub mod relevance;

// Re-export main types
pub use fingerprint::{
    BuildToolInfo, ConfigFile, Fingerprint, FingerprintInfo, FrameworkInfo,
    PackageInfo, PackageManager, ProjectLanguage, TestFrameworkInfo,
};
pub use relevance::{RelevanceScore, RelevanceScorer, RelevanceScorerConfig};
pub use cache::{
    CacheBuilder, CacheEntry, CacheEntryType, CacheTime, ComponentCache, ComponentType,
    ConfigCache, ContextCacheStore, ProjectCache, RouteCache, RouteType,
};
