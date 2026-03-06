//! Todo tool - task list management
//!
//! This module provides todo list management with:
//! - Create, update, delete tasks
//! - Status tracking (pending, in_progress, completed)
//! - Session-scoped task lists
//! - JSON serialization

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Todo item status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl Default for TodoStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "pending"),
            TodoStatus::InProgress => write!(f, "in_progress"),
            TodoStatus::Completed => write!(f, "completed"),
        }
    }
}

/// A single todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier
    pub id: String,

    /// Short title/subject
    pub subject: String,

    /// Detailed description (optional)
    #[serde(default)]
    pub description: Option<String>,

    /// Current status
    #[serde(default)]
    pub status: TodoStatus,

    /// Owner/assignee (optional)
    #[serde(default)]
    pub owner: Option<String>,

    /// Active form text (displayed while in progress)
    #[serde(default)]
    pub active_form: Option<String>,

    /// Items this task blocks (can't start until this completes)
    #[serde(default)]
    pub blocks: Vec<String>,

    /// Items that block this task
    #[serde(default)]
    pub blocked_by: Vec<String>,

    /// Arbitrary metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl TodoItem {
    /// Create a new todo item
    pub fn new(id: impl Into<String>, subject: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            subject: subject.into(),
            description: None,
            status: TodoStatus::Pending,
            owner: None,
            active_form: None,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Create with active form
    pub fn with_active_form(mut self, active_form: impl Into<String>) -> Self {
        self.active_form = Some(active_form.into());
        self
    }

    /// Check if this item is blocked by any incomplete items
    pub fn is_blocked(&self, todos: &[TodoItem]) -> bool {
        self.blocked_by.iter().any(|id| {
            todos
                .iter()
                .find(|t| t.id == *id)
                .map(|t| t.status != TodoStatus::Completed)
                .unwrap_or(false)
        })
    }

    /// Update the status
    pub fn set_status(&mut self, status: TodoStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Add a blocking relationship
    pub fn add_blocks(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.blocks.contains(&id) {
            self.blocks.push(id);
            self.updated_at = Utc::now();
        }
    }

    /// Add a blocked-by relationship
    pub fn add_blocked_by(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.blocked_by.contains(&id) {
            self.blocked_by.push(id);
            self.updated_at = Utc::now();
        }
    }
}

/// Todo list manager
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoList {
    /// All todo items
    items: Vec<TodoItem>,
    /// Auto-incrementing ID counter
    next_id: u64,
}

impl TodoList {
    /// Create a new empty todo list
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    /// Create from existing items
    pub fn from_items(items: Vec<TodoItem>) -> Self {
        let max_id = items
            .iter()
            .filter_map(|i| i.id.parse::<u64>().ok())
            .max()
            .unwrap_or(0);

        Self {
            items,
            next_id: max_id + 1,
        }
    }

    /// Get all items
    pub fn items(&self) -> &[TodoItem] {
        &self.items
    }

    /// Get all items (mutable)
    pub fn items_mut(&mut self) -> &mut Vec<TodoItem> {
        &mut self.items
    }

    /// Get an item by ID
    pub fn get(&self, id: &str) -> Option<&TodoItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Get an item by ID (mutable)
    pub fn get_mut(&mut self, id: &str) -> Option<&mut TodoItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Create a new todo item
    pub fn create(&mut self, subject: impl Into<String>, description: Option<String>) -> &TodoItem {
        let id = self.next_id.to_string();
        self.next_id += 1;

        let mut item = TodoItem::new(&id, subject);
        item.description = description;

        self.items.push(item);
        self.items.last().unwrap()
    }

    /// Create with active form
    pub fn create_with_active_form(
        &mut self,
        subject: impl Into<String>,
        description: Option<String>,
        active_form: impl Into<String>,
    ) -> &TodoItem {
        let id = self.next_id.to_string();
        self.next_id += 1;

        let mut item = TodoItem::new(&id, subject);
        item.description = description;
        item.active_form = Some(active_form.into());

        self.items.push(item);
        self.items.last().unwrap()
    }

    /// Update an item's status
    pub fn update_status(&mut self, id: &str, status: TodoStatus) -> Option<&TodoItem> {
        if let Some(item) = self.get_mut(id) {
            item.set_status(status);
            // Return reference to the updated item
            return self.get(id);
        }
        None
    }

    /// Delete an item
    pub fn delete(&mut self, id: &str) -> bool {
        let initial_len = self.items.len();
        self.items.retain(|i| i.id != id);

        // Remove from blocking relationships
        for item in &mut self.items {
            item.blocks.retain(|i| i != id);
            item.blocked_by.retain(|i| i != id);
        }

        self.items.len() < initial_len
    }

    /// Get pending items
    pub fn pending(&self) -> Vec<&TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Pending)
            .collect()
    }

    /// Get in-progress items
    pub fn in_progress(&self) -> Vec<&TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::InProgress)
            .collect()
    }

    /// Get completed items
    pub fn completed(&self) -> Vec<&TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Completed)
            .collect()
    }

    /// Get unblocked pending items (ready to work on)
    pub fn available(&self) -> Vec<&TodoItem> {
        self.items
            .iter()
            .filter(|i| i.status == TodoStatus::Pending && !i.is_blocked(&self.items))
            .collect()
    }

    /// Replace the entire list
    pub fn replace(&mut self, items: Vec<TodoItem>) {
        let max_id = items
            .iter()
            .filter_map(|i| i.id.parse::<u64>().ok())
            .max()
            .unwrap_or(0);

        self.items = items;
        self.next_id = max_id + 1;
    }

    /// Get summary statistics
    pub fn summary(&self) -> TodoSummary {
        TodoSummary {
            total: self.items.len(),
            pending: self.pending().len(),
            in_progress: self.in_progress().len(),
            completed: self.completed().len(),
            blocked: self.items.iter().filter(|i| i.is_blocked(&self.items)).count(),
        }
    }

    /// Format as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.items).unwrap_or_default()
    }
}

/// Summary statistics for a todo list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoSummary {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub blocked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_todo() {
        let mut list = TodoList::new();
        let item = list.create("Test task", Some("Description".to_string()));

        assert_eq!(item.id, "1");
        assert_eq!(item.subject, "Test task");
        assert_eq!(item.status, TodoStatus::Pending);
    }

    #[test]
    fn test_auto_increment_id() {
        let mut list = TodoList::new();
        list.create("Task 1", None);
        list.create("Task 2", None);
        let item = list.create("Task 3", None);

        assert_eq!(item.id, "3");
    }

    #[test]
    fn test_update_status() {
        let mut list = TodoList::new();
        list.create("Test task", None);

        list.update_status("1", TodoStatus::InProgress);
        assert_eq!(list.get("1").unwrap().status, TodoStatus::InProgress);

        list.update_status("1", TodoStatus::Completed);
        assert_eq!(list.get("1").unwrap().status, TodoStatus::Completed);
    }

    #[test]
    fn test_delete() {
        let mut list = TodoList::new();
        list.create("Task 1", None);
        list.create("Task 2", None);

        assert!(list.delete("1"));
        assert!(list.get("1").is_none());
        assert!(list.get("2").is_some());
    }

    #[test]
    fn test_blocking() {
        let mut list = TodoList::new();
        list.create("Task 1", None);
        list.create("Task 2", None);

        // Task 2 is blocked by Task 1
        list.get_mut("2").unwrap().add_blocked_by("1");
        list.get_mut("1").unwrap().add_blocks("2");

        let item2 = list.get("2").unwrap();
        assert!(item2.is_blocked(&list.items));

        // Complete task 1
        list.update_status("1", TodoStatus::Completed);

        let item2 = list.get("2").unwrap();
        assert!(!item2.is_blocked(&list.items));
    }

    #[test]
    fn test_available() {
        let mut list = TodoList::new();
        list.create("Task 1", None);
        list.create("Task 2", None);

        // Task 2 blocked by Task 1
        list.get_mut("2").unwrap().add_blocked_by("1");

        let available = list.available();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, "1");
    }

    #[test]
    fn test_summary() {
        let mut list = TodoList::new();
        list.create("Task 1", None);
        list.create("Task 2", None);
        list.create("Task 3", None);

        list.update_status("1", TodoStatus::InProgress);
        list.update_status("2", TodoStatus::Completed);

        let summary = list.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.pending, 1);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.completed, 1);
    }

    #[test]
    fn test_replace() {
        let mut list = TodoList::new();
        list.create("Old task", None);

        let new_items = vec![
            TodoItem::new("10", "New task 1"),
            TodoItem::new("20", "New task 2"),
        ];

        list.replace(new_items);

        assert_eq!(list.items().len(), 2);
        assert!(list.get("10").is_some());
        assert!(list.get("20").is_some());
        assert!(list.get("1").is_none());
    }

    #[test]
    fn test_json_serialization() {
        let mut list = TodoList::new();
        list.create("Test task", Some("Description".to_string()));

        let json = list.to_json();
        assert!(json.contains("Test task"));
        assert!(json.contains("Description"));
    }
}
