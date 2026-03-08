//! Application state shared across handlers

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use zero_core::{
    Grep, Ls, Reader, Truncator, Writer, Editor, MultiEditor, PatchApplicator, CodeSearch, WebFetcher,
    TodoList,
};

use crate::session::store::SessionStore;

/// Shared application state
pub struct AppState {
    /// Grep engine
    pub grep: Grep,
    /// Glob engine (using Ls)
    pub ls: Ls,
    /// File reader
    pub reader: Reader,
    /// File writer
    pub writer: Writer,
    /// File editor
    pub editor: Editor,
    /// Multi-file editor
    pub multi_editor: MultiEditor,
    /// Patch applicator
    pub patch_applicator: PatchApplicator,
    /// Code search engine
    pub code_search: CodeSearch,
    /// Web fetcher
    pub web_fetcher: WebFetcher,
    /// Truncator
    pub truncator: Truncator,
    /// Session todo lists (session_id -> TodoList)
    pub todo_lists: RwLock<HashMap<String, TodoList>>,
    /// Session store (SQLite-backed)
    pub session_store: SessionStore,
}

impl AppState {
    /// Create a new AppState instance
    pub fn new() -> Result<Self> {
        let data_dir = directories::ProjectDirs::from("ai", "codecoder", "zero-api")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("zero-api"));

        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            grep: Grep::new(),
            ls: Ls::new(),
            reader: Reader::new(),
            writer: Writer::new(),
            editor: Editor::new(),
            multi_editor: MultiEditor::new(),
            patch_applicator: PatchApplicator::new(),
            code_search: CodeSearch::new(),
            web_fetcher: WebFetcher::new(),
            truncator: Truncator::new(data_dir.join("tool-output")),
            todo_lists: RwLock::new(HashMap::new()),
            session_store: SessionStore::new(&data_dir.join("sessions.db"))?,
        })
    }

    /// Get or create a todo list for a session
    pub fn get_todo_list(&self, session_id: &str) -> TodoList {
        let lists = self.todo_lists.read().unwrap();
        lists.get(session_id).cloned().unwrap_or_else(TodoList::new)
    }

    /// Update a session's todo list
    pub fn set_todo_list(&self, session_id: &str, list: TodoList) {
        let mut lists = self.todo_lists.write().unwrap();
        lists.insert(session_id.to_string(), list);
    }
}
