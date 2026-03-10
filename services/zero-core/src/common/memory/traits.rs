//! Core Memory trait and types for the Zero memory system.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Category of memory entry for organization and retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    /// Core facts and preferences (long-term)
    Core,
    /// Project-specific context
    Project,
    /// Conversation history (may be pruned)
    Conversation,
    /// Daily notes and logs (auto-pruned after time)
    Daily,
    /// Temporary scratch space
    Scratch,
    /// Custom category
    Custom(String),
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Project => write!(f, "project"),
            Self::Conversation => write!(f, "conversation"),
            Self::Daily => write!(f, "daily"),
            Self::Scratch => write!(f, "scratch"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for MemoryCategory {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "core" => Self::Core,
            "project" => Self::Project,
            "conversation" => Self::Conversation,
            "daily" => Self::Daily,
            "scratch" => Self::Scratch,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// A single memory entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique key for this memory
    pub key: String,
    /// The stored content
    pub content: String,
    /// Category for organization
    pub category: MemoryCategory,
    /// Creation timestamp (Unix millis)
    pub created_at: i64,
    /// Last update timestamp (Unix millis)
    pub updated_at: i64,
    /// Relevance score from search (0.0-1.0)
    #[serde(default)]
    pub score: f32,
}

impl MemoryEntry {
    /// Create a new memory entry with current timestamp.
    pub fn new(key: impl Into<String>, content: impl Into<String>, category: MemoryCategory) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            key: key.into(),
            content: content.into(),
            category,
            created_at: now,
            updated_at: now,
            score: 0.0,
        }
    }

    /// Create entry with a specific score (for search results).
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }
}

/// Trait for memory backends — store, recall, forget.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Backend name (e.g., "sqlite", "markdown")
    fn name(&self) -> &str;

    /// Store content with a unique key.
    ///
    /// If key exists, content is updated.
    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()>;

    /// Recall memories matching a query, sorted by relevance.
    ///
    /// Uses hybrid search (vector + keyword) if embeddings are available.
    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Get a specific memory by key.
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;

    /// List all memories, optionally filtered by category.
    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Forget (delete) a memory by key.
    ///
    /// Returns true if the memory existed and was deleted.
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;

    /// Count total memories, optionally filtered by category.
    async fn count(&self, category: Option<&MemoryCategory>) -> anyhow::Result<usize>;

    /// Health check — returns true if backend is operational.
    async fn health_check(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_category_display() {
        assert_eq!(MemoryCategory::Core.to_string(), "core");
        assert_eq!(MemoryCategory::Project.to_string(), "project");
        assert_eq!(MemoryCategory::Conversation.to_string(), "conversation");
        assert_eq!(MemoryCategory::Scratch.to_string(), "scratch");
        assert_eq!(MemoryCategory::Custom("custom".into()).to_string(), "custom");
    }

    #[test]
    fn memory_category_from_str() {
        assert_eq!(MemoryCategory::from("core"), MemoryCategory::Core);
        assert_eq!(MemoryCategory::from("CORE"), MemoryCategory::Core);
        assert_eq!(MemoryCategory::from("project"), MemoryCategory::Project);
        assert_eq!(MemoryCategory::from("conversation"), MemoryCategory::Conversation);
        assert_eq!(MemoryCategory::from("scratch"), MemoryCategory::Scratch);
        assert_eq!(
            MemoryCategory::from("custom"),
            MemoryCategory::Custom("custom".into())
        );
    }

    #[test]
    fn memory_entry_new() {
        let entry = MemoryEntry::new("key1", "content", MemoryCategory::Core);
        assert_eq!(entry.key, "key1");
        assert_eq!(entry.content, "content");
        assert_eq!(entry.category, MemoryCategory::Core);
        assert!(entry.created_at > 0);
        assert_eq!(entry.created_at, entry.updated_at);
        assert!((entry.score - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn memory_entry_with_score() {
        let entry = MemoryEntry::new("key", "content", MemoryCategory::Core).with_score(0.95);
        assert!((entry.score - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn memory_category_serialization() {
        let json = serde_json::to_string(&MemoryCategory::Core).unwrap();
        assert_eq!(json, "\"core\"");

        let deserialized: MemoryCategory = serde_json::from_str("\"project\"").unwrap();
        assert_eq!(deserialized, MemoryCategory::Project);
    }

    #[test]
    fn memory_entry_serialization() {
        let entry = MemoryEntry::new("test", "content", MemoryCategory::Project);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"key\":\"test\""));
        assert!(json.contains("\"category\":\"project\""));

        let deserialized: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "test");
        assert_eq!(deserialized.category, MemoryCategory::Project);
    }
}
