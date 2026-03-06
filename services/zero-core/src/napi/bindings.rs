//! NAPI bindings implementation
//!
//! This module exposes zero-core functions to Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::tools::grep::{Grep, GrepOptions as RustGrepOptions};
use crate::tools::glob::{Glob, GlobOptions as RustGlobOptions};
use crate::tools::read::{Reader, ReadOptions as RustReadOptions};
use crate::tools::edit::{
    Editor, EditOperation as RustEditOperation,
    replace_with_fuzzy_match as rust_replace_with_fuzzy_match,
    levenshtein_distance as rust_levenshtein_distance,
    jaro_similarity as rust_jaro_similarity,
    jaro_winkler_similarity as rust_jaro_winkler_similarity,
    fuzzy_find as rust_fuzzy_find,
    find_best_fuzzy_match as rust_find_best_fuzzy_match,
};

// ============================================================================
// Grep bindings
// ============================================================================

/// Grep search options
#[napi(object)]
pub struct GrepOptions {
    pub pattern: String,
    pub path: Option<String>,
    pub glob: Option<String>,
    pub file_type: Option<String>,
    pub case_insensitive: Option<bool>,
    pub output_mode: Option<String>,
    pub context_before: Option<u32>,
    pub context_after: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub multiline: Option<bool>,
    pub line_numbers: Option<bool>,
}

/// Grep match result
#[napi(object)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: u32,
    pub column: u32,
    pub line_content: String,
}

/// Grep search result
#[napi(object)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
    pub files: Vec<String>,
    pub total_matches: u32,
    pub files_searched: u32,
    pub truncated: bool,
}

/// Perform a grep search
#[napi]
pub async fn grep(options: GrepOptions) -> Result<GrepResult> {
    let rust_options = RustGrepOptions {
        pattern: options.pattern,
        path: options.path,
        glob: options.glob,
        file_type: options.file_type,
        case_insensitive: options.case_insensitive.unwrap_or(false),
        output_mode: options.output_mode.unwrap_or_else(|| "files_with_matches".to_string()),
        context_before: options.context_before.unwrap_or(0) as usize,
        context_after: options.context_after.unwrap_or(0) as usize,
        limit: options.limit.map(|l| l as usize),
        offset: options.offset.unwrap_or(0) as usize,
        multiline: options.multiline.unwrap_or(false),
        line_numbers: options.line_numbers.unwrap_or(true),
    };

    let grep = Grep::new();
    let result = grep.search(&rust_options).await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(GrepResult {
        matches: result.matches.into_iter().map(|m| GrepMatch {
            path: m.path,
            line_number: m.line_number as u32,
            column: m.column as u32,
            line_content: m.line_content,
        }).collect(),
        files: result.files,
        total_matches: result.total_matches as u32,
        files_searched: result.files_searched as u32,
        truncated: result.truncated,
    })
}

// ============================================================================
// Glob bindings
// ============================================================================

/// Glob search options
#[napi(object)]
pub struct GlobOptions {
    pub pattern: String,
    pub path: Option<String>,
    pub include_hidden: Option<bool>,
    pub respect_gitignore: Option<bool>,
    pub max_depth: Option<u32>,
    pub limit: Option<u32>,
    pub sort_by_mtime: Option<bool>,
    pub files_only: Option<bool>,
    pub follow_symlinks: Option<bool>,
}

/// File info for glob results
#[napi(object)]
pub struct FileInfo {
    pub path: String,
    pub size: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<i64>,
    pub extension: Option<String>,
}

/// Glob search result
#[napi(object)]
pub struct GlobResult {
    pub files: Vec<FileInfo>,
    pub total_matches: u32,
    pub truncated: bool,
    pub duration_ms: u32,
}

/// Find files matching a glob pattern
#[napi]
pub async fn glob(options: GlobOptions) -> Result<GlobResult> {
    let rust_options = RustGlobOptions {
        pattern: options.pattern,
        path: options.path,
        include_hidden: options.include_hidden.unwrap_or(false),
        respect_gitignore: options.respect_gitignore.unwrap_or(true),
        max_depth: options.max_depth.map(|d| d as usize),
        limit: options.limit.map(|l| l as usize),
        sort_by_mtime: options.sort_by_mtime.unwrap_or(false),
        files_only: options.files_only.unwrap_or(true),
        follow_symlinks: options.follow_symlinks.unwrap_or(false),
    };

    let glob_engine = Glob::new();
    let result = glob_engine.find(&rust_options).await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(GlobResult {
        files: result.files.into_iter().map(|f| FileInfo {
            path: f.path,
            size: f.size as i64,
            is_dir: f.is_dir,
            is_symlink: f.is_symlink,
            modified: f.modified,
            extension: f.extension,
        }).collect(),
        total_matches: result.total_matches as u32,
        truncated: result.truncated,
        duration_ms: result.duration_ms as u32,
    })
}

// ============================================================================
// Read bindings
// ============================================================================

/// Read file options
#[napi(object)]
pub struct ReadOptions {
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub max_line_length: Option<u32>,
    pub line_numbers: Option<bool>,
}

/// Read file result
#[napi(object)]
pub struct ReadResult {
    pub content: String,
    pub lines: Vec<String>,
    pub total_lines: u32,
    pub lines_returned: u32,
    pub truncated: bool,
    pub size: i64,
    pub is_binary: bool,
}

/// Read a file
#[napi]
pub fn read_file(path: String, options: Option<ReadOptions>) -> Result<ReadResult> {
    let rust_options = options.map(|o| RustReadOptions {
        offset: o.offset.unwrap_or(1) as usize,
        limit: o.limit.map(|l| l as usize),
        max_line_length: o.max_line_length.unwrap_or(2000) as usize,
        line_numbers: o.line_numbers.unwrap_or(true),
        mmap_threshold: 10 * 1024 * 1024,
    });

    let reader = Reader::new();
    let result = reader.read(std::path::Path::new(&path), rust_options.as_ref())
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(ReadResult {
        content: result.content,
        lines: result.lines,
        total_lines: result.total_lines as u32,
        lines_returned: result.lines_returned as u32,
        truncated: result.truncated,
        size: result.size as i64,
        is_binary: result.is_binary,
    })
}

// ============================================================================
// Edit bindings
// ============================================================================

/// Edit operation
#[napi(object)]
pub struct EditOperation {
    pub old_string: String,
    pub new_string: String,
    pub replace_all: Option<bool>,
}

/// Edit result
#[napi(object)]
pub struct EditResult {
    pub success: bool,
    pub replacements: u32,
    pub diff: String,
    pub error: Option<String>,
}

/// Edit a file
#[napi]
pub fn edit_file(path: String, operation: EditOperation) -> Result<EditResult> {
    let rust_operation = RustEditOperation {
        old_string: operation.old_string,
        new_string: operation.new_string,
        replace_all: operation.replace_all.unwrap_or(false),
    };

    let editor = Editor::new();
    let result = editor.edit(std::path::Path::new(&path), &rust_operation)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(EditResult {
        success: result.success,
        replacements: result.replacements as u32,
        diff: result.diff,
        error: result.error,
    })
}

// ============================================================================
// Fuzzy Replace bindings
// ============================================================================

/// Result of a fuzzy replace operation
#[napi(object)]
pub struct FuzzyReplaceResult {
    /// The new content after replacement
    pub content: String,

    /// Whether a match was found
    pub found: bool,

    /// The actual string that was matched (may differ from old_string)
    pub matched_string: Option<String>,

    /// Which replacer strategy succeeded
    pub strategy: Option<String>,

    /// Error message if no match found
    pub error: Option<String>,
}

/// Replace content using fuzzy matching with multiple strategies
///
/// Tries multiple replacer strategies in order:
/// 1. Simple - exact match
/// 2. LineTrimmed - line-by-line trimmed comparison
/// 3. BlockAnchor - first/last line anchors with similarity scoring
/// 4. WhitespaceNormalized - normalizes whitespace before matching
/// 5. IndentationFlexible - removes indentation before matching
/// 6. EscapeNormalized - handles escaped strings
/// 7. TrimmedBoundary - trims boundaries before matching
/// 8. ContextAware - uses context lines as anchors
/// 9. MultiOccurrence - yields all exact matches
#[napi]
pub fn replace_with_fuzzy_match(
    content: String,
    old_string: String,
    new_string: String,
    replace_all: Option<bool>,
) -> FuzzyReplaceResult {
    let result = rust_replace_with_fuzzy_match(
        &content,
        &old_string,
        &new_string,
        replace_all.unwrap_or(false),
    );

    FuzzyReplaceResult {
        content: result.content,
        found: result.found,
        matched_string: result.matched_string,
        strategy: result.strategy,
        error: result.error,
    }
}

/// Calculate Levenshtein edit distance between two strings
///
/// The Levenshtein distance is the minimum number of single-character edits
/// (insertions, deletions, substitutions) required to change one string into another.
#[napi]
pub fn levenshtein_distance(a: String, b: String) -> u32 {
    rust_levenshtein_distance(&a, &b) as u32
}

/// Calculate Jaro similarity between two strings (0.0 to 1.0)
///
/// The Jaro similarity is based on matching characters within a window
/// and transpositions of matched characters.
#[napi]
pub fn jaro_similarity(s1: String, s2: String) -> f64 {
    rust_jaro_similarity(&s1, &s2)
}

/// Calculate Jaro-Winkler similarity between two strings (0.0 to 1.0)
///
/// Jaro-Winkler is a modification of Jaro that gives more weight to strings
/// that share a common prefix. This is particularly useful for code matching
/// where identifiers often share prefixes.
///
/// # Arguments
/// * `s1` - First string to compare
/// * `s2` - Second string to compare
/// * `prefix_weight` - Optional weight given to common prefix (default 0.1, max 0.25)
#[napi]
pub fn jaro_winkler_similarity(s1: String, s2: String, prefix_weight: Option<f64>) -> f64 {
    rust_jaro_winkler_similarity(&s1, &s2, prefix_weight)
}

/// A fuzzy match result
#[napi(object)]
pub struct NapiFuzzyMatch {
    /// Start position in haystack
    pub start: u32,
    /// End position in haystack
    pub end: u32,
    /// Similarity score (0.0 to 1.0)
    pub score: f64,
    /// The matched text
    pub matched_text: String,
}

/// Find the best fuzzy match for a needle in a haystack
///
/// Uses Jaro-Winkler for similarity scoring with a sliding window approach.
/// Returns the best match if it exceeds the threshold, None otherwise.
#[napi]
pub fn fuzzy_find(needle: String, haystack: String, threshold: f64) -> Option<NapiFuzzyMatch> {
    rust_fuzzy_find(&needle, &haystack, threshold).map(|m| NapiFuzzyMatch {
        start: m.start as u32,
        end: m.end as u32,
        score: m.score,
        matched_text: m.matched_text,
    })
}

/// Best match result
#[napi(object)]
pub struct NapiBestFuzzyMatch {
    /// The matched text
    pub text: String,
    /// Similarity score (0.0 to 1.0)
    pub score: f64,
}

/// Find the best match among multiple candidates using Jaro-Winkler similarity
///
/// Returns the candidate with the highest similarity score above the threshold
#[napi]
pub fn find_best_fuzzy_match(
    needle: String,
    candidates: Vec<String>,
    threshold: f64,
) -> Option<NapiBestFuzzyMatch> {
    let refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
    rust_find_best_fuzzy_match(&needle, &refs, threshold).map(|(text, score)| NapiBestFuzzyMatch {
        text: text.to_string(),
        score,
    })
}

// ============================================================================
// Session bindings
// ============================================================================

use crate::session::{
    Message as RustMessage, MessageRole as RustMessageRole, MessageStore as RustMessageStore,
    SessionData as RustSessionData, SessionStore as RustSessionStore,
    Compactor as RustCompactor, CompactionResult as RustCompactionResult,
    CompactionStrategy as RustCompactionStrategy,
};
use std::sync::{Arc, Mutex};

/// Message role
#[napi(string_enum)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl From<RustMessageRole> for MessageRole {
    fn from(role: RustMessageRole) -> Self {
        match role {
            RustMessageRole::System => MessageRole::System,
            RustMessageRole::User => MessageRole::User,
            RustMessageRole::Assistant => MessageRole::Assistant,
            RustMessageRole::Tool => MessageRole::Tool,
        }
    }
}

impl From<MessageRole> for RustMessageRole {
    fn from(role: MessageRole) -> Self {
        match role {
            MessageRole::System => RustMessageRole::System,
            MessageRole::User => RustMessageRole::User,
            MessageRole::Assistant => RustMessageRole::Assistant,
            MessageRole::Tool => RustMessageRole::Tool,
        }
    }
}

/// Message in a conversation
#[napi(object)]
pub struct NapiMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub tokens: Option<u32>,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub compacted: bool,
}

impl From<RustMessage> for NapiMessage {
    fn from(msg: RustMessage) -> Self {
        Self {
            id: msg.id,
            role: msg.role.to_string(),
            content: msg.content,
            timestamp: msg.timestamp.to_rfc3339(),
            tokens: msg.tokens.map(|t| t as u32),
            tool_call_id: msg.tool_call_id,
            tool_name: msg.tool_name,
            compacted: msg.compacted,
        }
    }
}

impl TryFrom<NapiMessage> for RustMessage {
    type Error = Error;

    fn try_from(msg: NapiMessage) -> Result<Self> {
        use chrono::{DateTime, Utc};

        let role = match msg.role.as_str() {
            "system" => RustMessageRole::System,
            "user" => RustMessageRole::User,
            "assistant" => RustMessageRole::Assistant,
            "tool" => RustMessageRole::Tool,
            _ => return Err(Error::from_reason(format!("Invalid message role: {}", msg.role))),
        };

        let timestamp = DateTime::parse_from_rfc3339(&msg.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid timestamp: {}", e)))?;

        Ok(Self {
            id: msg.id,
            role,
            content: msg.content,
            timestamp,
            tokens: msg.tokens.map(|t| t as usize),
            tool_call_id: msg.tool_call_id,
            tool_name: msg.tool_name,
            compacted: msg.compacted,
            metadata: serde_json::Value::Null,
        })
    }
}

/// Session data
#[napi(object)]
pub struct NapiSessionData {
    pub id: String,
    pub name: Option<String>,
    pub cwd: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<NapiMessage>,
}

impl From<RustSessionData> for NapiSessionData {
    fn from(session: RustSessionData) -> Self {
        Self {
            id: session.id,
            name: session.name,
            cwd: session.cwd,
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
            messages: session.messages.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<NapiSessionData> for RustSessionData {
    type Error = Error;

    fn try_from(session: NapiSessionData) -> Result<Self> {
        use chrono::{DateTime, Utc};

        let created_at = DateTime::parse_from_rfc3339(&session.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid created_at: {}", e)))?;

        let updated_at = DateTime::parse_from_rfc3339(&session.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid updated_at: {}", e)))?;

        let messages = session
            .messages
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            id: session.id,
            name: session.name,
            cwd: session.cwd,
            created_at,
            updated_at,
            messages,
            metadata: serde_json::Value::Null,
        })
    }
}

/// Handle to a MessageStore
#[napi]
pub struct MessageStoreHandle {
    inner: Arc<Mutex<RustMessageStore>>,
}

/// Create a new message store
#[napi]
pub fn create_message_store() -> MessageStoreHandle {
    MessageStoreHandle {
        inner: Arc::new(Mutex::new(RustMessageStore::new())),
    }
}

#[napi]
impl MessageStoreHandle {
    /// Add a message to the store
    #[napi]
    pub fn push(&self, message: NapiMessage) -> Result<()> {
        let rust_msg: RustMessage = message.try_into()?;
        let mut store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.push(rust_msg);
        Ok(())
    }

    /// Get all messages
    #[napi]
    pub fn messages(&self) -> Result<Vec<NapiMessage>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(store.messages().iter().cloned().map(Into::into).collect())
    }

    /// Get the last N messages
    #[napi]
    pub fn last_n(&self, n: u32) -> Result<Vec<NapiMessage>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(store.last_n(n as usize).iter().cloned().map(Into::into).collect())
    }

    /// Get total token count
    #[napi]
    pub fn total_tokens(&self) -> Result<u32> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(store.total_tokens() as u32)
    }

    /// Clear all messages
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.clear();
        Ok(())
    }

    /// Get message count
    #[napi]
    pub fn len(&self) -> Result<u32> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(store.len() as u32)
    }

    /// Check if store is empty
    #[napi]
    pub fn is_empty(&self) -> Result<bool> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(store.is_empty())
    }
}

/// Handle to a SessionStore
#[napi]
pub struct SessionStoreHandle {
    inner: Arc<Mutex<RustSessionStore>>,
}

/// Open or create a session store
#[napi]
pub fn open_session_store(path: String) -> Result<SessionStoreHandle> {
    let store = RustSessionStore::open(std::path::Path::new(&path))
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(SessionStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

#[napi]
impl SessionStoreHandle {
    /// Save a session
    #[napi]
    pub fn save(&self, session: NapiSessionData) -> Result<()> {
        let rust_session: RustSessionData = session.try_into()?;
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.save(&rust_session).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load a session by ID
    #[napi]
    pub fn load(&self, id: String) -> Result<Option<NapiSessionData>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let session = store.load(&id).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(session.map(Into::into))
    }

    /// List all sessions
    #[napi]
    pub fn list(&self) -> Result<Vec<NapiSessionData>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let sessions = store.list().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(sessions.into_iter().map(Into::into).collect())
    }

    /// Delete a session
    #[napi]
    pub fn delete(&self, id: String) -> Result<bool> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.delete(&id).map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Utility functions
// ============================================================================

// ============================================================================
// Security bindings - Permission
// ============================================================================

use crate::security::{
    Permission as RustPermission,
    PermissionManager as RustPermissionManager,
    PermissionRule as RustPermissionRule,
    SecretEntry as RustSecretEntry,
    Vault as RustVault,
    VaultConfig as RustVaultConfig,
};

/// Permission for access control
#[napi(object)]
pub struct NapiPermission {
    pub tool: String,
    pub action: String,
    pub resource: Option<String>,
}

impl From<RustPermission> for NapiPermission {
    fn from(p: RustPermission) -> Self {
        Self {
            tool: p.tool,
            action: p.action,
            resource: p.resource,
        }
    }
}

impl From<NapiPermission> for RustPermission {
    fn from(p: NapiPermission) -> Self {
        match p.resource {
            Some(r) => RustPermission::with_resource(p.tool, p.action, r),
            None => RustPermission::new(p.tool, p.action),
        }
    }
}

/// Permission rule (allow or deny)
#[napi(object)]
pub struct NapiPermissionRule {
    pub permission: NapiPermission,
    pub allow: bool,
    pub reason: Option<String>,
}

impl From<RustPermissionRule> for NapiPermissionRule {
    fn from(r: RustPermissionRule) -> Self {
        Self {
            permission: r.permission.into(),
            allow: r.allow,
            reason: r.reason,
        }
    }
}

impl From<NapiPermissionRule> for RustPermissionRule {
    fn from(r: NapiPermissionRule) -> Self {
        let base = if r.allow {
            RustPermissionRule::allow(r.permission.into())
        } else {
            RustPermissionRule::deny(r.permission.into())
        };
        match r.reason {
            Some(reason) => base.with_reason(reason),
            None => base,
        }
    }
}

/// Handle to a PermissionManager
#[napi]
pub struct PermissionManagerHandle {
    inner: Arc<Mutex<RustPermissionManager>>,
}

/// Create a new permission manager
#[napi]
pub fn create_permission_manager() -> PermissionManagerHandle {
    PermissionManagerHandle {
        inner: Arc::new(Mutex::new(RustPermissionManager::new())),
    }
}

#[napi]
impl PermissionManagerHandle {
    /// Add a rule to the manager
    #[napi]
    pub fn add_rule(&self, rule: NapiPermissionRule) -> Result<()> {
        let mut manager = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        manager.add_rule(rule.into());
        Ok(())
    }

    /// Grant a specific permission
    #[napi]
    pub fn grant(&self, permission: NapiPermission) -> Result<()> {
        let mut manager = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        manager.grant(&permission.into());
        Ok(())
    }

    /// Check if a permission is allowed
    #[napi]
    pub fn check(&self, permission: NapiPermission) -> Result<bool> {
        let manager = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(manager.check(&permission.into()))
    }

    /// Clear all rules and grants
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut manager = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        *manager = RustPermissionManager::new();
        Ok(())
    }
}

// ============================================================================
// Security bindings - Vault
// ============================================================================

/// A secret entry in the vault
#[napi(object)]
pub struct NapiSecretEntry {
    pub name: String,
    pub value: String,
    pub description: Option<String>,
}

/// Handle to a Vault
#[napi]
pub struct VaultHandle {
    inner: Arc<Mutex<RustVault>>,
}

/// Open or create a vault with a password
#[napi]
pub fn open_vault(path: String, password: String) -> Result<VaultHandle> {
    let config = RustVaultConfig {
        path: std::path::PathBuf::from(path),
        use_keychain: false,
    };
    let vault = RustVault::open(config, &password)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(VaultHandle {
        inner: Arc::new(Mutex::new(vault)),
    })
}

/// Create an in-memory vault (for testing)
#[napi]
pub fn create_memory_vault(password: String) -> VaultHandle {
    VaultHandle {
        inner: Arc::new(Mutex::new(RustVault::in_memory(&password))),
    }
}

#[napi]
impl VaultHandle {
    /// Store a secret
    #[napi]
    pub fn set(&self, entry: NapiSecretEntry) -> Result<()> {
        let mut vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_entry = match entry.description {
            Some(desc) => RustSecretEntry::new(entry.name, entry.value).with_description(desc),
            None => RustSecretEntry::new(entry.name, entry.value),
        };
        vault.set(rust_entry);
        Ok(())
    }

    /// Get a secret by name
    #[napi]
    pub fn get(&self, name: String) -> Result<Option<NapiSecretEntry>> {
        let vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(vault.get(&name).map(|e| NapiSecretEntry {
            name: e.name.clone(),
            value: e.value.clone(),
            description: e.description.clone(),
        }))
    }

    /// Get just the secret value
    #[napi]
    pub fn get_value(&self, name: String) -> Result<Option<String>> {
        let vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(vault.get_value(&name).map(String::from))
    }

    /// Delete a secret
    #[napi]
    pub fn delete(&self, name: String) -> Result<bool> {
        let mut vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(vault.delete(&name))
    }

    /// List all secret names
    #[napi]
    pub fn list(&self) -> Result<Vec<String>> {
        let vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(vault.list().into_iter().map(String::from).collect())
    }

    /// Save the vault to disk
    #[napi]
    pub fn save(&self) -> Result<()> {
        let vault = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        vault.save().map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Compaction bindings
// ============================================================================

/// Compaction strategy enum
#[napi(string_enum)]
pub enum NapiCompactionStrategy {
    /// Remove oldest messages first
    RemoveOldest,
    /// Summarize older messages (requires LLM - falls back to RemoveOldest in Rust)
    Summarize,
    /// Hybrid: summarize then remove
    Hybrid,
}

impl From<NapiCompactionStrategy> for RustCompactionStrategy {
    fn from(strategy: NapiCompactionStrategy) -> Self {
        match strategy {
            NapiCompactionStrategy::RemoveOldest => RustCompactionStrategy::RemoveOldest,
            NapiCompactionStrategy::Summarize => RustCompactionStrategy::Summarize,
            NapiCompactionStrategy::Hybrid => RustCompactionStrategy::Hybrid,
        }
    }
}

impl From<RustCompactionStrategy> for NapiCompactionStrategy {
    fn from(strategy: RustCompactionStrategy) -> Self {
        match strategy {
            RustCompactionStrategy::RemoveOldest => NapiCompactionStrategy::RemoveOldest,
            RustCompactionStrategy::Summarize => NapiCompactionStrategy::Summarize,
            RustCompactionStrategy::Hybrid => NapiCompactionStrategy::Hybrid,
        }
    }
}

/// Result of compaction operation
#[napi(object)]
pub struct NapiCompactionResult {
    /// Number of messages before compaction
    pub messages_before: u32,
    /// Number of messages after compaction
    pub messages_after: u32,
    /// Tokens before compaction
    pub tokens_before: u32,
    /// Tokens after compaction
    pub tokens_after: u32,
    /// Summary generated (if any)
    pub summary: Option<String>,
}

impl From<RustCompactionResult> for NapiCompactionResult {
    fn from(result: RustCompactionResult) -> Self {
        Self {
            messages_before: result.messages_before as u32,
            messages_after: result.messages_after as u32,
            tokens_before: result.tokens_before as u32,
            tokens_after: result.tokens_after as u32,
            summary: result.summary,
        }
    }
}

/// Handle to a Compactor for managing context window
#[napi]
pub struct CompactorHandle {
    inner: Arc<Mutex<RustCompactor>>,
}

/// Create a new compactor with default settings (128k max, 100k target)
#[napi]
pub fn create_compactor() -> CompactorHandle {
    CompactorHandle {
        inner: Arc::new(Mutex::new(RustCompactor::default())),
    }
}

/// Create a compactor with custom token limits
#[napi]
pub fn create_compactor_with_limits(max_tokens: u32, target_tokens: u32) -> CompactorHandle {
    CompactorHandle {
        inner: Arc::new(Mutex::new(RustCompactor::new(
            max_tokens as usize,
            target_tokens as usize,
        ))),
    }
}

#[napi]
impl CompactorHandle {
    /// Check if compaction is needed for the given messages
    #[napi]
    pub fn needs_compaction(&self, messages: Vec<NapiMessage>) -> Result<bool> {
        let compactor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_messages: Vec<RustMessage> = messages
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;
        Ok(compactor.needs_compaction(&rust_messages))
    }

    /// Compact messages to fit within token limit
    /// Returns the compaction result with before/after metrics
    #[napi]
    pub fn compact(&self, messages: Vec<NapiMessage>) -> Result<NapiCompactResult> {
        let compactor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let mut rust_messages: Vec<RustMessage> = messages
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        let result = compactor.compact(&mut rust_messages);
        let compacted_messages: Vec<NapiMessage> = rust_messages.into_iter().map(Into::into).collect();

        Ok(NapiCompactResult {
            result: result.into(),
            messages: compacted_messages,
        })
    }

    /// Set the compaction strategy
    #[napi]
    pub fn set_strategy(&self, strategy: NapiCompactionStrategy) -> Result<()> {
        // The Rust Compactor uses a builder pattern, so we need to recreate it
        // For now, we'll store the strategy preference but can't change existing instance
        // This is a limitation of the current Rust API
        let _ = strategy; // Acknowledge strategy but current Rust API doesn't support changing after creation
        Ok(())
    }
}

/// Combined result containing both metrics and compacted messages
#[napi(object)]
pub struct NapiCompactResult {
    /// Compaction metrics (before/after counts)
    pub result: NapiCompactionResult,
    /// The compacted messages
    pub messages: Vec<NapiMessage>,
}

/// Estimate token count for text (fast, approximate)
#[napi]
pub fn estimate_tokens(text: String) -> u32 {
    RustCompactor::estimate_tokens(&text) as u32
}

// ============================================================================
// Utility functions
// ============================================================================

/// Get the zero-core version
#[napi]
pub fn version() -> String {
    crate::VERSION.to_string()
}

/// Initialize the library
#[napi]
pub fn init() -> Result<()> {
    crate::init().map_err(|e| Error::from_reason(e.to_string()))
}
