//! Application state shared across handlers

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use zero_core::{
    Grep, Ls, Reader, Truncator, Writer, Editor, MultiEditor, PatchApplicator, CodeSearch, WebFetcher,
    TodoList,
};

use crate::session::store::SessionStore;
use crate::unified_api::state::UnifiedApiState;

/// Shared application state
///
/// Note: This struct is not Clone because several zero_core types don't implement Clone.
/// Use Arc<AppState> when sharing between handlers.
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
    pub todo_lists: Arc<RwLock<HashMap<String, TodoList>>>,
    /// Session store (SQLite-backed)
    pub session_store: SessionStore,
    /// Unified API state (for agent execution, observer, etc.)
    pub unified_api: Option<Arc<UnifiedApiState>>,
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
            todo_lists: Arc::new(RwLock::new(HashMap::new())),
            session_store: SessionStore::new(&data_dir.join("sessions.db"))?,
            unified_api: None,
        })
    }

    /// Create AppState with unified API state
    pub fn with_unified_api(unified_api: Arc<UnifiedApiState>) -> Result<Self> {
        let mut state = Self::new()?;
        state.unified_api = Some(unified_api);
        Ok(state)
    }

    /// Set unified API state (for lazy initialization)
    pub fn set_unified_api(&mut self, unified_api: Arc<UnifiedApiState>) {
        self.unified_api = Some(unified_api);
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
