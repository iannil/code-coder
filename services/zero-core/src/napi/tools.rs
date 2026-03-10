//! NAPI bindings for tools module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - PatchApplicatorHandle: Patch parsing and application
//! - EditorHandle: File editing with diff support
//! - Utility functions: similarity_ratio, find_best_match

use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::tools::{
    apply_patch::{
        ApplyPatchOptions as RustApplyPatchOptions, ApplyPatchResult as RustApplyPatchResult,
        PatchApplicator as RustPatchApplicator, PatchChunk as RustPatchChunk,
        PatchFileResult as RustPatchFileResult, PatchHunk as RustPatchHunk, PatchType,
    },
    edit::{
        EditOperation as RustEditOperation, EditResult as RustEditResult, Editor as RustEditor,
        find_best_match as rust_find_best_match, similarity_ratio as rust_similarity_ratio,
    },
};

// ============================================================================
// Patch Types for NAPI
// ============================================================================

/// Patch type (add, update, delete, move)
#[napi(string_enum)]
pub enum NapiPatchType {
    Add,
    Update,
    Delete,
    Move,
}

impl From<PatchType> for NapiPatchType {
    fn from(pt: PatchType) -> Self {
        match pt {
            PatchType::Add => NapiPatchType::Add,
            PatchType::Update => NapiPatchType::Update,
            PatchType::Delete => NapiPatchType::Delete,
            PatchType::Move => NapiPatchType::Move,
        }
    }
}

impl From<NapiPatchType> for PatchType {
    fn from(pt: NapiPatchType) -> Self {
        match pt {
            NapiPatchType::Add => PatchType::Add,
            NapiPatchType::Update => PatchType::Update,
            NapiPatchType::Delete => PatchType::Delete,
            NapiPatchType::Move => PatchType::Move,
        }
    }
}

/// A chunk within an update hunk
#[napi(object)]
pub struct NapiPatchChunk {
    pub context_before: Vec<String>,
    pub removals: Vec<String>,
    pub additions: Vec<String>,
    pub context_after: Vec<String>,
}

impl From<RustPatchChunk> for NapiPatchChunk {
    fn from(chunk: RustPatchChunk) -> Self {
        Self {
            context_before: chunk.context_before,
            removals: chunk.removals,
            additions: chunk.additions,
            context_after: chunk.context_after,
        }
    }
}

/// A single hunk in a patch
#[napi(object)]
pub struct NapiPatchHunk {
    pub path: String,
    pub patch_type: String, // "add" | "update" | "delete" | "move"
    pub move_path: Option<String>,
    pub content: Option<String>,
    pub chunks: Vec<NapiPatchChunk>,
}

impl From<RustPatchHunk> for NapiPatchHunk {
    fn from(hunk: RustPatchHunk) -> Self {
        Self {
            path: hunk.path,
            patch_type: match hunk.patch_type {
                PatchType::Add => "add".to_string(),
                PatchType::Update => "update".to_string(),
                PatchType::Delete => "delete".to_string(),
                PatchType::Move => "move".to_string(),
            },
            move_path: hunk.move_path,
            content: hunk.content,
            chunks: hunk.chunks.into_iter().map(|c| c.into()).collect(),
        }
    }
}

/// Result of a single file in patch application
#[napi(object)]
pub struct NapiPatchFileResult {
    pub file_path: String,
    pub relative_path: String,
    pub patch_type: String, // "add" | "update" | "delete" | "move"
    pub before: String,
    pub after: String,
    pub diff: String,
    pub additions: u32,
    pub deletions: u32,
    pub move_path: Option<String>,
}

impl From<RustPatchFileResult> for NapiPatchFileResult {
    fn from(result: RustPatchFileResult) -> Self {
        Self {
            file_path: result.file_path,
            relative_path: result.relative_path,
            patch_type: match result.patch_type {
                PatchType::Add => "add".to_string(),
                PatchType::Update => "update".to_string(),
                PatchType::Delete => "delete".to_string(),
                PatchType::Move => "move".to_string(),
            },
            before: result.before,
            after: result.after,
            diff: result.diff,
            additions: result.additions as u32,
            deletions: result.deletions as u32,
            move_path: result.move_path,
        }
    }
}

/// Result of patch application
#[napi(object)]
pub struct NapiApplyPatchResult {
    pub success: bool,
    pub files: Vec<NapiPatchFileResult>,
    pub combined_diff: String,
    pub output: String,
    pub error: Option<String>,
    pub files_changed: u32,
    pub total_additions: u32,
    pub total_deletions: u32,
}

impl From<RustApplyPatchResult> for NapiApplyPatchResult {
    fn from(result: RustApplyPatchResult) -> Self {
        Self {
            success: result.success,
            files: result.files.into_iter().map(|f| f.into()).collect(),
            combined_diff: result.combined_diff,
            output: result.output,
            error: result.error,
            files_changed: result.files_changed as u32,
            total_additions: result.total_additions as u32,
            total_deletions: result.total_deletions as u32,
        }
    }
}

/// Options for patch application
#[napi(object)]
pub struct NapiApplyPatchOptions {
    pub working_dir: Option<String>,
    pub dry_run: Option<bool>,
    pub create_backups: Option<bool>,
    pub fuzz: Option<u32>,
}

impl From<NapiApplyPatchOptions> for RustApplyPatchOptions {
    fn from(options: NapiApplyPatchOptions) -> Self {
        Self {
            working_dir: options.working_dir,
            dry_run: options.dry_run.unwrap_or(false),
            create_backups: options.create_backups.unwrap_or(false),
            fuzz: options.fuzz.unwrap_or(0) as usize,
        }
    }
}

// ============================================================================
// Edit Types for NAPI
// ============================================================================

/// Edit operation
#[napi(object)]
pub struct NapiEditOperation {
    pub old_string: String,
    pub new_string: String,
    pub replace_all: Option<bool>,
}

impl From<NapiEditOperation> for RustEditOperation {
    fn from(op: NapiEditOperation) -> Self {
        Self {
            old_string: op.old_string,
            new_string: op.new_string,
            replace_all: op.replace_all.unwrap_or(false),
        }
    }
}

/// Result of an edit operation
#[napi(object)]
pub struct NapiEditResult {
    pub success: bool,
    pub replacements: u32,
    pub diff: String,
    pub error: Option<String>,
    pub original_hash: Option<String>,
}

impl From<RustEditResult> for NapiEditResult {
    fn from(result: RustEditResult) -> Self {
        Self {
            success: result.success,
            replacements: result.replacements as u32,
            diff: result.diff,
            error: result.error,
            original_hash: result.original_hash,
        }
    }
}

/// Best match result
#[napi(object)]
pub struct NapiBestMatch {
    pub text: String,
    pub ratio: f64,
}

// ============================================================================
// PatchApplicator NAPI Handle
// ============================================================================

/// Thread-safe wrapper for PatchApplicator
#[napi]
pub struct PatchApplicatorHandle {
    inner: Arc<Mutex<RustPatchApplicator>>,
}

#[napi]
impl PatchApplicatorHandle {
    /// Create a new PatchApplicator
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustPatchApplicator::new())),
        }
    }

    /// Create with custom default options
    #[napi(factory)]
    pub fn with_defaults(options: NapiApplyPatchOptions) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustPatchApplicator::with_defaults(options.into()))),
        }
    }

    /// Parse a patch text and return hunks
    #[napi]
    pub fn parse_patch(&self, patch_text: String) -> Result<Vec<NapiPatchHunk>> {
        let applicator = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let hunks = applicator
            .parse_patch(&patch_text)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(hunks.into_iter().map(|h| h.into()).collect())
    }

    /// Apply a patch from text
    #[napi]
    pub fn apply(&self, patch_text: String, options: Option<NapiApplyPatchOptions>) -> Result<NapiApplyPatchResult> {
        let applicator = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_options = options.map(|o| o.into());
        let result = applicator
            .apply(&patch_text, rust_options.as_ref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }
}

// ============================================================================
// Editor NAPI Handle
// ============================================================================

/// Thread-safe wrapper for Editor
#[napi]
pub struct EditorHandle {
    inner: Arc<Mutex<RustEditor>>,
}

#[napi]
impl EditorHandle {
    /// Create a new Editor
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustEditor::new())),
        }
    }

    /// Edit a file by replacing old_string with new_string
    #[napi]
    pub fn edit(&self, file_path: String, operation: NapiEditOperation) -> Result<NapiEditResult> {
        let editor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let path = std::path::Path::new(&file_path);
        let rust_op: RustEditOperation = operation.into();
        let result = editor
            .edit(path, &rust_op)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Edit a file with multiple operations
    #[napi]
    pub fn edit_multiple(&self, file_path: String, operations: Vec<NapiEditOperation>) -> Result<NapiEditResult> {
        let editor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let path = std::path::Path::new(&file_path);
        let rust_ops: Vec<RustEditOperation> = operations.into_iter().map(|o| o.into()).collect();
        let result = editor
            .edit_multiple(path, &rust_ops)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Generate a unified diff between two strings
    #[napi]
    pub fn generate_diff(&self, old_content: String, new_content: String, file_path: String) -> Result<String> {
        let editor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let path = std::path::Path::new(&file_path);
        Ok(editor.generate_diff(&old_content, &new_content, path))
    }

    /// Compute a diff between two files
    #[napi]
    pub fn diff_files(&self, old_path: String, new_path: String) -> Result<String> {
        let editor = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let old = std::path::Path::new(&old_path);
        let new = std::path::Path::new(&new_path);
        editor
            .diff_files(old, new)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Compute the similarity ratio between two strings (0.0 to 1.0)
#[napi]
pub fn similarity_ratio(s1: String, s2: String) -> f64 {
    rust_similarity_ratio(&s1, &s2)
}

/// Find the best match for a string in a list of candidates
#[napi]
pub fn find_best_match(needle: String, haystack: Vec<String>) -> Option<NapiBestMatch> {
    let refs: Vec<&str> = haystack.iter().map(|s| s.as_str()).collect();
    rust_find_best_match(&needle, &refs).map(|(text, ratio)| NapiBestMatch {
        text: text.to_string(),
        ratio,
    })
}

/// Compute unified diff between two strings (standalone function)
#[napi]
pub fn compute_diff(old_content: String, new_content: String, file_path: String) -> String {
    let editor = RustEditor::new();
    let path = std::path::Path::new(&file_path);
    editor.generate_diff(&old_content, &new_content, path)
}

// ============================================================================
// DiffLines Types and Functions
// ============================================================================

/// A single change from diffLines
#[napi(object)]
pub struct NapiDiffChange {
    /// The text content of this change
    pub value: String,

    /// Number of lines in this change
    pub count: u32,

    /// True if this is an addition
    pub added: bool,

    /// True if this is a removal
    pub removed: bool,
}

/// Compute line-by-line diff between two strings
///
/// Returns an array of changes compatible with npm 'diff' package format.
/// Each change has: value (content), count (line count), added, removed
#[napi]
pub fn diff_lines(old_content: String, new_content: String) -> Vec<NapiDiffChange> {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(&old_content, &new_content);
    let mut changes = Vec::new();

    // Group consecutive operations of the same type
    let mut current_tag: Option<ChangeTag> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for change in diff.iter_all_changes() {
        let tag = change.tag();

        if Some(tag) != current_tag && !current_lines.is_empty() {
            // Flush the current group
            if let Some(prev_tag) = current_tag {
                changes.push(create_diff_change(&current_lines, prev_tag));
            }
            current_lines.clear();
        }

        current_tag = Some(tag);
        current_lines.push(change.value());
    }

    // Flush remaining
    if !current_lines.is_empty() {
        if let Some(tag) = current_tag {
            changes.push(create_diff_change(&current_lines, tag));
        }
    }

    changes
}

fn create_diff_change(lines: &[&str], tag: similar::ChangeTag) -> NapiDiffChange {
    let value = lines.join("");
    let count = lines.len() as u32;

    match tag {
        similar::ChangeTag::Insert => NapiDiffChange {
            value,
            count,
            added: true,
            removed: false,
        },
        similar::ChangeTag::Delete => NapiDiffChange {
            value,
            count,
            added: false,
            removed: true,
        },
        similar::ChangeTag::Equal => NapiDiffChange {
            value,
            count,
            added: false,
            removed: false,
        },
    }
}

/// Create a unified diff patch string (equivalent to createTwoFilesPatch from npm 'diff')
///
/// Returns a unified diff format string suitable for displaying or applying
#[napi]
pub fn create_two_files_patch(
    old_path: String,
    new_path: String,
    old_content: String,
    new_content: String,
) -> String {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(&old_content, &new_content);

    let mut output = String::new();
    output.push_str(&format!("--- {}\n", old_path));
    output.push_str(&format!("+++ {}\n", new_path));

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            output.push('\n');
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => '-',
                    ChangeTag::Insert => '+',
                    ChangeTag::Equal => ' ',
                };

                output.push(sign);
                output.push_str(change.value());
                if !change.missing_newline() {
                    // Ensure each line ends with newline in the output
                    if !change.value().ends_with('\n') {
                        output.push('\n');
                    }
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_applicator_handle() {
        let applicator = PatchApplicatorHandle::new();
        let patch = r#"*** Begin Patch
*** src/new.txt ADD
+content line 1
+content line 2
*** End Patch"#;

        let hunks = applicator.parse_patch(patch.to_string()).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].patch_type, "add");
    }

    #[test]
    fn test_similarity_ratio() {
        let ratio = similarity_ratio("hello".to_string(), "hello".to_string());
        assert!(ratio > 0.99);

        let ratio = similarity_ratio("hello".to_string(), "helo".to_string());
        assert!(ratio > 0.7);
    }

    #[test]
    fn test_find_best_match() {
        let candidates = vec![
            "apple".to_string(),
            "application".to_string(),
            "banana".to_string(),
        ];
        let result = find_best_match("appl".to_string(), candidates).unwrap();
        assert_eq!(result.text, "apple");
        assert!(result.ratio > 0.7);
    }

    #[test]
    fn test_compute_diff() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nline 3\n";
        let diff = compute_diff(old.to_string(), new.to_string(), "test.txt".to_string());
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+line 3"));
    }

    #[test]
    fn test_diff_lines() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 4\nline 3\n";
        let changes = diff_lines(old.to_string(), new.to_string());

        // Should have at least one change
        assert!(!changes.is_empty());

        // Find the additions and deletions
        let additions: u32 = changes.iter().filter(|c| c.added).map(|c| c.count).sum();
        let deletions: u32 = changes.iter().filter(|c| c.removed).map(|c| c.count).sum();

        assert!(additions > 0, "Should have additions");
        assert!(deletions > 0, "Should have deletions");
    }

    #[test]
    fn test_diff_lines_no_change() {
        let content = "line 1\nline 2\n";
        let changes = diff_lines(content.to_string(), content.to_string());

        // All changes should be equal (not added or removed)
        for change in &changes {
            assert!(!change.added);
            assert!(!change.removed);
        }
    }

    #[test]
    fn test_create_two_files_patch() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nline 3\n";
        let patch = create_two_files_patch(
            "old.txt".to_string(),
            "new.txt".to_string(),
            old.to_string(),
            new.to_string(),
        );

        assert!(patch.contains("--- old.txt"));
        assert!(patch.contains("+++ new.txt"));
        assert!(patch.contains("-line 2"));
        assert!(patch.contains("+line 3"));
    }
}
