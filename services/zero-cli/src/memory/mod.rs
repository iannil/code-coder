//! Memory system for ZeroBot.
//!
//! This module provides the memory factory and unique hygiene functionality.
//! Core memory implementations are imported from `zero-memory`.

pub mod crystallize;
pub mod hygiene;

pub use crystallize::{CrystallizedKnowledge, Crystallizer};

// Re-export memory types from zero-memory
pub use zero_memory::{MarkdownMemory, Memory, MemoryCategory, MemoryEntry, SqliteMemory};

use crate::config::MemoryConfig;
use std::path::Path;

/// Factory: create the right memory backend from config
pub fn create_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    _api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    // Best-effort memory hygiene/retention pass (throttled by state file).
    if let Err(e) = hygiene::run_if_due(config, workspace_dir) {
        tracing::warn!("memory hygiene skipped: {e}");
    }

    match config.backend.as_str() {
        "sqlite" => {
            // Note: SqliteMemory uses FTS5 for keyword search.
            // Vector search with embeddings requires external embedding generation.
            let mem = SqliteMemory::new(workspace_dir)?;
            Ok(Box::new(mem))
        }
        "markdown" | "none" => {
            let mem = MarkdownMemory::new(workspace_dir)?;
            Ok(Box::new(mem))
        }
        other => {
            tracing::warn!("Unknown memory backend '{other}', falling back to markdown");
            let mem = MarkdownMemory::new(workspace_dir)?;
            Ok(Box::new(mem))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn factory_sqlite() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "sqlite".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn factory_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_none_falls_back_to_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "none".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn factory_unknown_falls_back_to_markdown() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "redis".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "markdown");
    }
}
