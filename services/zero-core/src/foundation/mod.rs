//! Foundation module - configuration, file utilities, scheduler, memory, watcher
//!
//! This module provides:
//! - **config**: Configuration loading and validation
//! - **file**: File type detection and utilities
//! - **schema**: JSON Schema validation for configuration
//! - **watcher**: File system change detection
//! - **scheduler**: Task scheduling (planned)
//! - **memory**: Memory management (planned)

pub mod config;
pub mod file;
pub mod ignore;
pub mod schema;
pub mod watcher;

// Scheduler and memory will be added later
// pub mod scheduler;
// pub mod memory;

// Re-export main types
pub use config::{Config, ConfigLoader};
pub use file::{FileInfo, FileType};
pub use ignore::{
    get_default_folders, get_default_patterns, should_ignore,
    IgnoreCheckResult, IgnoreConfig, IgnoreEngine, IgnoreStats,
};
pub use schema::{SchemaValidator, ValidationIssue};
pub use watcher::{
    FileWatcher, FileWatcherConfig, MultiWatcher, WatchEvent, WatchEventKind,
};
