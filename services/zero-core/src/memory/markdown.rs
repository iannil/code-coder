//! Markdown-based dual-layer memory system
//!
//! Transparent memory architecture with two layers:
//! - **Flow layer (daily)**: Chronological daily notes in `./memory/daily/{YYYY-MM-DD}.md`
//! - **Sediment layer (long-term)**: Consolidated knowledge in `./memory/MEMORY.md`
//!
//! Design principles:
//! - Human-readable markdown files
//! - Git-friendly storage
//! - No complex embedding retrieval
//! - Immutable daily notes (append-only)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Daily note entry types for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DailyEntryType {
    Decision,
    Action,
    Output,
    Error,
    Solution,
}

impl std::fmt::Display for DailyEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DailyEntryType::Decision => write!(f, "decision"),
            DailyEntryType::Action => write!(f, "action"),
            DailyEntryType::Output => write!(f, "output"),
            DailyEntryType::Error => write!(f, "error"),
            DailyEntryType::Solution => write!(f, "solution"),
        }
    }
}

/// Single entry in daily notes (flow layer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Entry type for categorization
    pub entry_type: DailyEntryType,
    /// Entry content
    pub content: String,
    /// Optional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Long-term memory categories (sediment layer)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryCategory {
    #[serde(rename = "用户偏好")]
    UserPreferences,
    #[serde(rename = "项目上下文")]
    ProjectContext,
    #[serde(rename = "关键决策")]
    KeyDecisions,
    #[serde(rename = "经验教训")]
    LessonsLearned,
    #[serde(rename = "成功方案")]
    SuccessfulSolutions,
}

impl MemoryCategory {
    /// Get the Chinese display name
    pub fn display_name(&self) -> &'static str {
        match self {
            MemoryCategory::UserPreferences => "用户偏好",
            MemoryCategory::ProjectContext => "项目上下文",
            MemoryCategory::KeyDecisions => "关键决策",
            MemoryCategory::LessonsLearned => "经验教训",
            MemoryCategory::SuccessfulSolutions => "成功方案",
        }
    }

    /// Get all categories
    pub fn all() -> &'static [MemoryCategory] {
        &[
            MemoryCategory::UserPreferences,
            MemoryCategory::ProjectContext,
            MemoryCategory::KeyDecisions,
            MemoryCategory::LessonsLearned,
            MemoryCategory::SuccessfulSolutions,
        ]
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Long-term memory section structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    pub category: MemoryCategory,
    pub content: String,
    pub last_updated: String,
}

/// Combined memory context result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    /// Long-term memory content
    pub long_term: String,
    /// Recent daily notes
    pub daily: Vec<String>,
    /// Combined context string
    pub combined: String,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMemoryConfig {
    /// Base path for memory storage
    pub base_path: PathBuf,
    /// Project identifier
    pub project_id: String,
    /// Daily notes directory path
    pub daily_path: PathBuf,
    /// Long-term memory file path
    pub long_term_path: PathBuf,
}

impl MarkdownMemoryConfig {
    /// Create configuration with default paths
    pub fn new(base_path: impl AsRef<Path>, project_id: impl Into<String>) -> Self {
        let base = base_path.as_ref().to_path_buf();
        Self {
            daily_path: base.join("daily"),
            long_term_path: base.join("MEMORY.md"),
            base_path: base,
            project_id: project_id.into(),
        }
    }

    /// Create with default base path (current directory + memory)
    pub fn default_for_project(project_id: impl Into<String>) -> Self {
        let base = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("memory");
        Self::new(base, project_id)
    }
}

// ============================================================================
// Markdown Memory Store
// ============================================================================

/// Markdown-based memory store
pub struct MarkdownMemoryStore {
    config: MarkdownMemoryConfig,
}

impl MarkdownMemoryStore {
    /// Create a new memory store with the given configuration
    pub fn new(config: MarkdownMemoryConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration for a project
    pub fn for_project(base_path: impl AsRef<Path>, project_id: impl Into<String>) -> Self {
        Self::new(MarkdownMemoryConfig::new(base_path, project_id))
    }

    /// Get the configuration
    pub fn config(&self) -> &MarkdownMemoryConfig {
        &self.config
    }

    // ========================================================================
    // Daily Notes (Flow Layer)
    // ========================================================================

    /// Get daily note file path for a given date
    pub fn get_daily_path(&self, date: NaiveDate) -> PathBuf {
        let filename = format!("{}.md", date.format("%Y-%m-%d"));
        self.config.daily_path.join(filename)
    }

    /// Append a new entry to today's daily notes
    pub fn append_daily_note(&self, entry: &DailyEntry) -> Result<()> {
        let today = Local::now().date_naive();
        let daily_path = self.get_daily_path(today);

        // Ensure directory exists
        self.ensure_dir(&self.config.daily_path)?;

        // Format entry as markdown
        let markdown = self.format_daily_entry(entry);

        // Append or create file
        let exists = daily_path.exists();
        if exists {
            let existing = fs::read_to_string(&daily_path)
                .with_context(|| format!("Failed to read daily note: {:?}", daily_path))?;
            let updated = format!("{}\n\n{}\n", existing.trim_end(), markdown);
            fs::write(&daily_path, updated)
                .with_context(|| format!("Failed to write daily note: {:?}", daily_path))?;
        } else {
            let header = format!("# Daily Notes - {}\n\n", today.format("%Y-%m-%d"));
            let content = format!("{}{}\n", header, markdown);
            fs::write(&daily_path, content)
                .with_context(|| format!("Failed to create daily note: {:?}", daily_path))?;
        }

        Ok(())
    }

    /// Load daily notes for a specific date range
    pub fn load_daily_notes(&self, start_date: NaiveDate, days: usize) -> Result<Vec<String>> {
        let mut notes = Vec::new();

        for i in 0..days {
            let date = start_date + chrono::Duration::days(i as i64);
            let daily_path = self.get_daily_path(date);

            if daily_path.exists() {
                let content = fs::read_to_string(&daily_path)
                    .with_context(|| format!("Failed to read daily note: {:?}", daily_path))?;
                notes.push(content);
            }
        }

        Ok(notes)
    }

    /// Get content from today's daily notes
    pub fn get_today_notes(&self) -> Result<String> {
        let today = Local::now().date_naive();
        let daily_path = self.get_daily_path(today);

        if daily_path.exists() {
            fs::read_to_string(&daily_path)
                .with_context(|| format!("Failed to read today's notes: {:?}", daily_path))
        } else {
            Ok(format!(
                "# Daily Notes - {}\n\n_No entries yet._\n",
                today.format("%Y-%m-%d")
            ))
        }
    }

    /// List all available daily note dates
    pub fn list_daily_note_dates(&self) -> Result<Vec<String>> {
        if !self.config.daily_path.exists() {
            return Ok(Vec::new());
        }

        let mut dates = Vec::new();
        for entry in fs::read_dir(&self.config.daily_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                if let Some(stem) = path.file_stem() {
                    dates.push(stem.to_string_lossy().to_string());
                }
            }
        }

        dates.sort_by(|a, b| b.cmp(a)); // Reverse chronological
        Ok(dates)
    }

    /// Create a daily entry helper
    pub fn create_entry(
        entry_type: DailyEntryType,
        content: impl Into<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> DailyEntry {
        DailyEntry {
            timestamp: Utc::now().to_rfc3339(),
            entry_type,
            content: content.into(),
            metadata: metadata.unwrap_or_default(),
        }
    }

    // ========================================================================
    // Long-term Memory (Sediment Layer)
    // ========================================================================

    /// Load entire long-term memory file
    pub fn load_long_term_memory(&self) -> Result<String> {
        self.ensure_memory_file()?;

        if self.config.long_term_path.exists() {
            fs::read_to_string(&self.config.long_term_path)
                .with_context(|| "Failed to load long-term memory")
        } else {
            Ok(self.get_default_memory_content())
        }
    }

    /// Load specific category from long-term memory
    pub fn load_category(&self, category: MemoryCategory) -> Result<String> {
        let content = self.load_long_term_memory()?;
        let category_content = self.extract_category(&content, category);

        Ok(category_content.unwrap_or_else(|| {
            format!(
                "{}\n_No entries yet._\n",
                self.format_section_header(category)
            )
        }))
    }

    /// Update or create a category in long-term memory
    pub fn update_category(&self, category: MemoryCategory, content: &str) -> Result<()> {
        self.ensure_memory_file()?;

        let existing = self.load_long_term_memory()?;
        let existing_content = self.extract_category(&existing, category);

        let updated_content = if existing_content.is_some() {
            self.replace_category(&existing, category, content)
        } else {
            self.append_category(&existing, category, content)
        };

        fs::write(&self.config.long_term_path, updated_content)
            .with_context(|| "Failed to update long-term memory")?;

        Ok(())
    }

    /// Merge new content into existing category
    pub fn merge_to_category(&self, category: MemoryCategory, update: &str) -> Result<()> {
        let existing = self.load_category(category)?;
        let merged = self.smart_merge(category, &existing, update);
        self.update_category(category, &merged)
    }

    /// Get all memory sections as typed objects
    pub fn get_memory_sections(&self) -> Result<Vec<MemorySection>> {
        let content = self.load_long_term_memory()?;
        let now = Utc::now().to_rfc3339();

        Ok(MemoryCategory::all()
            .iter()
            .map(|&cat| MemorySection {
                category: cat,
                content: self.extract_category(&content, cat).unwrap_or_default(),
                last_updated: now.clone(),
            })
            .collect())
    }

    /// Add item to a category list
    pub fn add_list_item(
        &self,
        category: MemoryCategory,
        item: &str,
        subtext: Option<&str>,
    ) -> Result<()> {
        let existing = self.load_category(category)?;
        let header = self.format_section_header(category);
        let item_entry = match subtext {
            Some(sub) => format!("- **{}**: {}", item, sub),
            None => format!("- {}", item),
        };

        let updated = if existing.contains(&header) {
            let existing_items = existing.replace(&header, "").trim().to_string();
            format!("{}\n{}\n{}\n", header, existing_items, item_entry)
        } else {
            format!("{}\n{}\n", header, item_entry)
        };

        self.update_category(category, &updated)
    }

    /// Remove item from a category
    pub fn remove_list_item(&self, category: MemoryCategory, item_pattern: &str) -> Result<()> {
        let existing = self.load_category(category)?;
        let filtered: Vec<&str> = existing
            .lines()
            .filter(|line| {
                !line.contains(item_pattern) || line.starts_with("## ") || line.trim() == "─".repeat(40)
            })
            .collect();

        self.update_category(category, &filtered.join("\n"))
    }

    // ========================================================================
    // Context Loading
    // ========================================================================

    /// Load combined memory context
    pub fn load_context(&self, include_days: usize) -> Result<MemoryContext> {
        let long_term = self.load_long_term_memory()?;

        let today = Local::now().date_naive();
        let start = today - chrono::Duration::days((include_days - 1) as i64);
        let daily = self.load_daily_notes(start, include_days)?;

        let combined = format!(
            "# Long-term Memory\n\n{}\n\n# Recent Daily Notes\n\n{}",
            long_term,
            daily.join("\n\n---\n\n")
        );

        Ok(MemoryContext {
            long_term,
            daily,
            combined,
        })
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    fn ensure_dir(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)
                .with_context(|| format!("Failed to create directory: {:?}", path))?;
        }
        Ok(())
    }

    fn ensure_memory_file(&self) -> Result<()> {
        self.ensure_dir(&self.config.base_path)?;

        if !self.config.long_term_path.exists() {
            let content = self.get_default_memory_content();
            fs::write(&self.config.long_term_path, content)
                .with_context(|| "Failed to create MEMORY.md")?;
        }

        Ok(())
    }

    fn format_daily_entry(&self, entry: &DailyEntry) -> String {
        let timestamp = DateTime::parse_from_rfc3339(&entry.timestamp)
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|_| entry.timestamp.clone());

        let type_badge = match entry.entry_type {
            DailyEntryType::Decision => "🎯",
            DailyEntryType::Action => "⚡",
            DailyEntryType::Output => "📤",
            DailyEntryType::Error => "❌",
            DailyEntryType::Solution => "✅",
        };

        let mut result = format!(
            "### {} [{}] {}\n\n{}",
            type_badge, timestamp, entry.entry_type, entry.content
        );

        if !entry.metadata.is_empty() {
            if let Ok(meta_json) = serde_json::to_string_pretty(&entry.metadata) {
                result.push_str(&format!("\n\n<details>\n<summary>Metadata</summary>\n\n```json\n{}\n```\n</details>", meta_json));
            }
        }

        result
    }

    fn format_section_header(&self, category: MemoryCategory) -> String {
        format!(
            "## {}\n{}",
            category.display_name(),
            "─".repeat(40)
        )
    }

    fn extract_category(&self, content: &str, category: MemoryCategory) -> Option<String> {
        let header = format!("## {}", category.display_name());
        let lines: Vec<&str> = content.lines().collect();

        let mut in_category = false;
        let mut category_lines = Vec::new();

        for line in lines {
            if line.starts_with(&header) {
                in_category = true;
                category_lines.push(line);
                continue;
            }

            if in_category {
                if line.starts_with("## ") {
                    break;
                }
                category_lines.push(line);
            }
        }

        if category_lines.is_empty() {
            None
        } else {
            Some(category_lines.join("\n"))
        }
    }

    fn replace_category(&self, content: &str, category: MemoryCategory, new_content: &str) -> String {
        let header = format!("## {}", category.display_name());
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_category = false;
        let mut replaced = false;

        for line in lines {
            if line.starts_with(&header) {
                in_category = true;
                result.push(line);
                // Add new content, skipping the header line from new_content
                let new_lines: Vec<&str> = new_content
                    .lines()
                    .skip_while(|l| l.starts_with("## "))
                    .collect();
                result.extend(new_lines);
                replaced = true;
                continue;
            }

            if in_category {
                if line.starts_with("## ") {
                    in_category = false;
                    result.push(line);
                }
                continue;
            }

            result.push(line);
        }

        if !replaced {
            return self.append_category(content, category, new_content);
        }

        result.join("\n")
    }

    fn append_category(&self, content: &str, category: MemoryCategory, new_content: &str) -> String {
        format!(
            "{}\n\n{}\n{}\n",
            content.trim_end(),
            self.format_section_header(category),
            new_content.trim()
        )
    }

    fn smart_merge(&self, category: MemoryCategory, existing: &str, update: &str) -> String {
        let header = self.format_section_header(category);

        let existing_lines: std::collections::HashSet<&str> = existing
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with("## ") && !l.starts_with("─") && !l.contains("No entries"))
            .collect();

        let update_lines: Vec<&str> = update
            .lines()
            .filter(|l| !l.is_empty())
            .collect();

        let mut merged: Vec<&str> = existing_lines.into_iter().collect();
        for line in update_lines {
            if !merged.contains(&line) {
                merged.push(line);
            }
        }

        format!("{}\n{}\n", header, merged.join("\n"))
    }

    fn get_default_memory_content(&self) -> String {
        format!(
            r#"# Long-term Memory

Transparent markdown-based memory storage. Last updated: {}
Project ID: {}

## 用户偏好
────────────────────────────────────────────────

_No preferences recorded yet._

## 项目上下文
────────────────────────────────────────────────

_No project context recorded yet._

## 关键决策
────────────────────────────────────────────────

_No decisions recorded yet._

## 经验教训
────────────────────────────────────────────────

_No lessons learned yet._

## 成功方案
────────────────────────────────────────────────

_No successful solutions recorded yet._
"#,
            Utc::now().to_rfc3339(),
            self.config.project_id
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> (MarkdownMemoryStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownMemoryConfig::new(temp_dir.path(), "test-project");
        let store = MarkdownMemoryStore::new(config);
        (store, temp_dir)
    }

    #[test]
    fn test_append_daily_note() {
        let (store, _temp) = create_test_store();

        let entry = MarkdownMemoryStore::create_entry(
            DailyEntryType::Decision,
            "Test decision",
            None,
        );

        store.append_daily_note(&entry).unwrap();

        let notes = store.get_today_notes().unwrap();
        assert!(notes.contains("Test decision"));
        assert!(notes.contains("decision"));
    }

    #[test]
    fn test_long_term_memory() {
        let (store, _temp) = create_test_store();

        // Load creates default content
        let content = store.load_long_term_memory().unwrap();
        assert!(content.contains("用户偏好"));
        assert!(content.contains("项目上下文"));

        // Add to category
        store.add_list_item(
            MemoryCategory::UserPreferences,
            "Prefers TypeScript",
            Some("For all new projects"),
        ).unwrap();

        let updated = store.load_category(MemoryCategory::UserPreferences).unwrap();
        assert!(updated.contains("Prefers TypeScript"));
    }

    #[test]
    fn test_memory_context() {
        let (store, _temp) = create_test_store();

        // Add a daily note
        let entry = MarkdownMemoryStore::create_entry(
            DailyEntryType::Action,
            "Started migration",
            None,
        );
        store.append_daily_note(&entry).unwrap();

        // Load context
        let context = store.load_context(1).unwrap();
        assert!(!context.long_term.is_empty());
        assert!(!context.combined.is_empty());
    }

    #[test]
    fn test_category_extraction() {
        let (store, _temp) = create_test_store();

        let content = store.get_default_memory_content();
        let extracted = store.extract_category(&content, MemoryCategory::UserPreferences);

        assert!(extracted.is_some());
        assert!(extracted.unwrap().contains("用户偏好"));
    }

    #[test]
    fn test_list_daily_note_dates() {
        let (store, _temp) = create_test_store();

        // Create some daily notes
        let entry = MarkdownMemoryStore::create_entry(
            DailyEntryType::Action,
            "Test",
            None,
        );
        store.append_daily_note(&entry).unwrap();

        let dates = store.list_daily_note_dates().unwrap();
        assert!(!dates.is_empty());
    }
}
