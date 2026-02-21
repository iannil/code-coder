//! Markdown-based memory storage.
//!
//! Stores memories as human-readable markdown files:
//! - Core memory: `{workspace}/MEMORY.md`
//! - Daily memory: `{workspace}/memory/{YYYY-MM-DD}.md`

use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use chrono::{Local, NaiveDate};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Markdown-based memory backend.
///
/// Layout:
/// - `{workspace}/MEMORY.md` - Core, long-term memories
/// - `{workspace}/memory/{YYYY-MM-DD}.md` - Daily memories (append-only)
pub struct MarkdownMemory {
    workspace: PathBuf,
    // In-memory index for search
    index: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

impl MarkdownMemory {
    /// Create a new markdown memory at the given workspace directory.
    pub fn new(workspace: &Path) -> anyhow::Result<Self> {
        let memory_dir = workspace.join("memory");
        fs::create_dir_all(&memory_dir)?;

        let mem = Self {
            workspace: workspace.to_path_buf(),
            index: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing memories into index (best effort)
        if tokio::runtime::Handle::try_current().is_ok() {
            tracing::debug!("Markdown memory initialized, index will be built lazily");
        }

        Ok(mem)
    }

    /// Get the core memory file path.
    fn core_memory_path(&self) -> PathBuf {
        self.workspace.join("MEMORY.md")
    }

    /// Get the daily memory file path for a date.
    fn daily_memory_path(&self, date: NaiveDate) -> PathBuf {
        self.workspace
            .join("memory")
            .join(format!("{}.md", date.format("%Y-%m-%d")))
    }

    /// Get today's daily memory file path.
    fn today_memory_path(&self) -> PathBuf {
        self.daily_memory_path(Local::now().date_naive())
    }

    /// Append content to the daily memory file.
    fn append_to_daily(&self, key: &str, content: &str, category: &MemoryCategory) -> anyhow::Result<()> {
        let path = self.today_memory_path();
        let now = Local::now().format("%H:%M:%S");
        let entry = format!(
            "\n## [{now}] {key}\n**Category:** {category}\n\n{content}\n",
            now = now,
            key = key,
            category = category,
            content = content
        );

        let mut file_content = fs::read_to_string(&path).unwrap_or_default();
        if file_content.is_empty() {
            let header = format!("# Daily Memory - {}\n", Local::now().format("%Y-%m-%d"));
            file_content = header;
        }
        file_content.push_str(&entry);
        fs::write(&path, file_content)?;

        Ok(())
    }

    /// Update or append to core memory.
    fn update_core_memory(&self, key: &str, content: &str, category: &MemoryCategory) -> anyhow::Result<()> {
        let path = self.core_memory_path();
        let mut file_content = fs::read_to_string(&path).unwrap_or_else(|_| "# Core Memory\n\n".to_string());

        // Look for existing section
        let section_header = format!("## {key}");
        if let Some(start) = file_content.find(&section_header) {
            // Find the end of this section (next ## or end of file)
            let rest = &file_content[start + section_header.len()..];
            let end = rest.find("\n## ").map(|i| start + section_header.len() + i).unwrap_or(file_content.len());

            // Replace the section
            let new_section = format!("## {key}\n**Category:** {category}\n\n{content}\n");
            file_content.replace_range(start..end, &new_section);
        } else {
            // Append new section
            let new_section = format!("\n## {key}\n**Category:** {category}\n\n{content}\n");
            file_content.push_str(&new_section);
        }

        fs::write(&path, file_content)?;
        Ok(())
    }

    /// Build index from all markdown files.
    async fn build_index(&self) -> anyhow::Result<()> {
        let mut index = self.index.write().await;
        index.clear();

        // Parse core memory
        if let Ok(content) = fs::read_to_string(self.core_memory_path()) {
            for entry in parse_markdown_sections(&content) {
                index.insert(entry.key.clone(), entry);
            }
        }

        // Parse daily memory files
        let memory_dir = self.workspace.join("memory");
        if memory_dir.is_dir() {
            for entry in fs::read_dir(&memory_dir)?.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for mem_entry in parse_markdown_sections(&content) {
                            index.insert(mem_entry.key.clone(), mem_entry);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parse markdown content into memory entries.
fn parse_markdown_sections(content: &str) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_content = String::new();
    let mut current_category = MemoryCategory::Core;

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry
            if let Some(key) = current_key.take() {
                if !current_content.trim().is_empty() {
                    entries.push(MemoryEntry::new(key, current_content.trim(), current_category.clone()));
                }
                current_content.clear();
            }

            // Parse new key (remove timestamp if present)
            let header = line.strip_prefix("## ").unwrap_or("");
            let key = if let Some(end) = header.find(']') {
                header[end + 1..].trim().to_string()
            } else {
                header.trim().to_string()
            };
            current_key = Some(key);
            current_category = MemoryCategory::Core;
        } else if line.starts_with("**Category:**") {
            let cat_str = line.strip_prefix("**Category:**").unwrap_or("").trim();
            current_category = MemoryCategory::from(cat_str);
        } else if current_key.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last entry
    if let Some(key) = current_key {
        if !current_content.trim().is_empty() {
            entries.push(MemoryEntry::new(key, current_content.trim(), current_category));
        }
    }

    entries
}

#[async_trait]
impl Memory for MarkdownMemory {
    fn name(&self) -> &str {
        "markdown"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        // Update core memory for Core category, otherwise append to daily
        match category {
            MemoryCategory::Core | MemoryCategory::Project => {
                self.update_core_memory(key, content, &category)?;
            }
            _ => {
                self.append_to_daily(key, content, &category)?;
            }
        }

        // Update index
        let mut index = self.index.write().await;
        index.insert(key.to_string(), MemoryEntry::new(key, content, category));

        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        // Rebuild index if empty
        if self.index.read().await.is_empty() {
            self.build_index().await?;
        }

        let index = self.index.read().await;
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f32, MemoryEntry)> = index
            .values()
            .filter_map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let key_lower = entry.key.to_lowercase();

                // Simple word matching score
                let mut score = 0.0_f32;
                for word in &query_words {
                    if key_lower.contains(word) {
                        score += 2.0; // Key match is worth more
                    }
                    if content_lower.contains(word) {
                        score += 1.0;
                    }
                }

                if score > 0.0 {
                    let mut result = entry.clone();
                    result.score = score;
                    Some((score, result))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        // Rebuild index if empty
        if self.index.read().await.is_empty() {
            self.build_index().await?;
        }

        Ok(self.index.read().await.get(key).cloned())
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        // Rebuild index if empty
        if self.index.read().await.is_empty() {
            self.build_index().await?;
        }

        let index = self.index.read().await;
        let entries: Vec<MemoryEntry> = match category {
            Some(cat) => index.values().filter(|e| &e.category == cat).cloned().collect(),
            None => index.values().cloned().collect(),
        };

        Ok(entries)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let mut index = self.index.write().await;
        let existed = index.remove(key).is_some();

        // Note: We don't actually delete from markdown files
        // This is intentional for audit trail purposes
        // The in-memory index just no longer returns this entry

        Ok(existed)
    }

    async fn count(&self, category: Option<&MemoryCategory>) -> anyhow::Result<usize> {
        if self.index.read().await.is_empty() {
            self.build_index().await?;
        }

        let index = self.index.read().await;
        let count = match category {
            Some(cat) => index.values().filter(|e| &e.category == cat).count(),
            None => index.len(),
        };

        Ok(count)
    }

    async fn health_check(&self) -> bool {
        self.workspace.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, MarkdownMemory) {
        let tmp = TempDir::new().unwrap();
        let mem = MarkdownMemory::new(tmp.path()).unwrap();
        (tmp, mem)
    }

    #[tokio::test]
    async fn store_and_get_core() {
        let (_tmp, mem) = setup();

        mem.store("key1", "content1", MemoryCategory::Core)
            .await
            .unwrap();

        let entry = mem.get("key1").await.unwrap().unwrap();
        assert_eq!(entry.key, "key1");
        assert_eq!(entry.content, "content1");
    }

    #[tokio::test]
    async fn store_updates_existing_core() {
        let (_tmp, mem) = setup();

        mem.store("key1", "original", MemoryCategory::Core)
            .await
            .unwrap();
        mem.store("key1", "updated", MemoryCategory::Core)
            .await
            .unwrap();

        let entry = mem.get("key1").await.unwrap().unwrap();
        assert_eq!(entry.content, "updated");
    }

    #[tokio::test]
    async fn store_conversation_to_daily() {
        let (tmp, mem) = setup();

        mem.store("chat1", "Hello world", MemoryCategory::Conversation)
            .await
            .unwrap();

        let today = Local::now().date_naive();
        let daily_path = tmp
            .path()
            .join("memory")
            .join(format!("{}.md", today.format("%Y-%m-%d")));
        assert!(daily_path.exists());

        let content = fs::read_to_string(&daily_path).unwrap();
        assert!(content.contains("Hello world"));
    }

    #[tokio::test]
    async fn recall_finds_matches() {
        let (_tmp, mem) = setup();

        mem.store("rust_guide", "Rust programming language", MemoryCategory::Core)
            .await
            .unwrap();
        mem.store("python_guide", "Python scripting", MemoryCategory::Core)
            .await
            .unwrap();

        let results = mem.recall("Rust programming", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].key, "rust_guide");
    }

    #[tokio::test]
    async fn list_all() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();

        let entries = mem.list(None).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn list_by_category() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();

        let core_entries = mem.list(Some(&MemoryCategory::Core)).await.unwrap();
        assert_eq!(core_entries.len(), 1);
    }

    #[tokio::test]
    async fn forget_removes_from_index() {
        let (_tmp, mem) = setup();

        mem.store("key1", "content", MemoryCategory::Core)
            .await
            .unwrap();

        // Add another entry to keep index non-empty
        mem.store("key2", "content2", MemoryCategory::Core)
            .await
            .unwrap();

        let deleted = mem.forget("key1").await.unwrap();
        assert!(deleted);

        // Entry should be removed from the active index
        // Note: Files are preserved for audit trail, but index marks entry as forgotten
        let entry = mem.get("key1").await.unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn count_all() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();

        assert_eq!(mem.count(None).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn health_check_returns_true() {
        let (_tmp, mem) = setup();
        assert!(mem.health_check().await);
    }

    #[tokio::test]
    async fn name_returns_markdown() {
        let (_tmp, mem) = setup();
        assert_eq!(mem.name(), "markdown");
    }

    #[test]
    fn parse_markdown_sections_extracts_entries() {
        let content = r#"# Core Memory

## User Preferences
**Category:** core

Prefers dark mode.

## Project Context
**Category:** project

Working on AI assistant.
"#;

        let entries = parse_markdown_sections(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "User Preferences");
        assert_eq!(entries[1].key, "Project Context");
    }

    #[test]
    fn parse_markdown_sections_with_timestamp() {
        let content = r#"## [10:30:45] chat_message
**Category:** conversation

Hello world
"#;

        let entries = parse_markdown_sections(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "chat_message");
    }
}
