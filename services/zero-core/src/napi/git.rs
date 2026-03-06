//! NAPI bindings for git module
//!
//! Exposes GitOpsHandle to Node.js/TypeScript for high-performance git operations.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::git::{
    CloneOptions as RustCloneOptions, CommitInfo as RustCommitInfo,
    CommitResult as RustCommitResult, DiffFile as RustDiffFile, DiffResult as RustDiffResult,
    FileStatus as RustFileStatus, FileStatusType as RustFileStatusType,
    GitOpsHandle as RustGitOpsHandle, GitStatus as RustGitStatus, InitOptions as RustInitOptions,
    OperationResult as RustOperationResult, WorktreeInfo as RustWorktreeInfo,
};

// ============================================================================
// Type Definitions
// ============================================================================

/// File status type
#[napi(string_enum)]
pub enum FileStatusType {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Typechange,
    Conflicted,
}

impl From<RustFileStatusType> for FileStatusType {
    fn from(s: RustFileStatusType) -> Self {
        match s {
            RustFileStatusType::Modified => FileStatusType::Modified,
            RustFileStatusType::Added => FileStatusType::Added,
            RustFileStatusType::Deleted => FileStatusType::Deleted,
            RustFileStatusType::Renamed => FileStatusType::Renamed,
            RustFileStatusType::Copied => FileStatusType::Copied,
            RustFileStatusType::Untracked => FileStatusType::Untracked,
            RustFileStatusType::Ignored => FileStatusType::Ignored,
            RustFileStatusType::Typechange => FileStatusType::Typechange,
            RustFileStatusType::Conflicted => FileStatusType::Conflicted,
        }
    }
}

/// Single file status
#[napi(object)]
pub struct NapiFileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub old_path: Option<String>,
}

impl From<RustFileStatus> for NapiFileStatus {
    fn from(s: RustFileStatus) -> Self {
        Self {
            path: s.path,
            status: format!("{:?}", s.status).to_lowercase(),
            staged: s.staged,
            old_path: s.old_path,
        }
    }
}

/// Git status result
#[napi(object)]
pub struct NapiGitStatus {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub renamed: HashMap<String, String>,
    pub untracked: Vec<String>,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<NapiFileStatus>,
}

impl From<RustGitStatus> for NapiGitStatus {
    fn from(s: RustGitStatus) -> Self {
        Self {
            modified: s.modified,
            added: s.added,
            deleted: s.deleted,
            renamed: s.renamed,
            untracked: s.untracked,
            branch: s.branch,
            ahead: s.ahead,
            behind: s.behind,
            files: s.files.into_iter().map(Into::into).collect(),
        }
    }
}

/// Git commit result
#[napi(object)]
pub struct NapiCommitResult {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub error: Option<String>,
}

impl From<RustCommitResult> for NapiCommitResult {
    fn from(r: RustCommitResult) -> Self {
        Self {
            success: r.success,
            commit_hash: r.commit_hash,
            error: r.error,
        }
    }
}

/// Commit info
#[napi(object)]
pub struct NapiCommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: i64,
}

impl From<RustCommitInfo> for NapiCommitInfo {
    fn from(c: RustCommitInfo) -> Self {
        Self {
            hash: c.hash,
            message: c.message,
            author: c.author,
            date: c.date,
        }
    }
}

/// Diff file entry
#[napi(object)]
pub struct NapiDiffFile {
    pub path: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
}

impl From<RustDiffFile> for NapiDiffFile {
    fn from(f: RustDiffFile) -> Self {
        Self {
            path: f.path,
            status: format!("{:?}", f.status).to_lowercase(),
            additions: f.additions,
            deletions: f.deletions,
            patch: f.patch,
        }
    }
}

/// Diff result
#[napi(object)]
pub struct NapiDiffResult {
    pub files: Vec<NapiDiffFile>,
    pub insertions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

impl From<RustDiffResult> for NapiDiffResult {
    fn from(r: RustDiffResult) -> Self {
        Self {
            files: r.files.into_iter().map(Into::into).collect(),
            insertions: r.insertions,
            deletions: r.deletions,
            files_changed: r.files_changed,
        }
    }
}

/// Operation result
#[napi(object)]
pub struct NapiOperationResult {
    pub success: bool,
    pub error: Option<String>,
}

impl From<RustOperationResult> for NapiOperationResult {
    fn from(r: RustOperationResult) -> Self {
        Self {
            success: r.success,
            error: r.error,
        }
    }
}

/// Init options
#[napi(object)]
pub struct NapiInitOptions {
    pub default_branch: Option<String>,
    pub initial_commit: Option<bool>,
    pub commit_message: Option<String>,
}

impl From<NapiInitOptions> for RustInitOptions {
    fn from(o: NapiInitOptions) -> Self {
        Self {
            default_branch: o.default_branch.unwrap_or_else(|| "main".to_string()),
            initial_commit: o.initial_commit.unwrap_or(true),
            commit_message: o.commit_message.unwrap_or_else(|| "Initial commit".to_string()),
        }
    }
}

/// Clone options
#[napi(object)]
pub struct NapiCloneOptions {
    pub depth: Option<u32>,
    pub branch: Option<String>,
    pub reinitialize: Option<bool>,
}

impl From<NapiCloneOptions> for RustCloneOptions {
    fn from(o: NapiCloneOptions) -> Self {
        Self {
            depth: o.depth,
            branch: o.branch,
            reinitialize: o.reinitialize.unwrap_or(true),
        }
    }
}

/// Worktree information
#[napi(object)]
pub struct NapiWorktreeInfo {
    /// The name of the worktree (used to identify it in git)
    pub name: String,
    /// The filesystem path to the worktree
    pub path: String,
    /// The branch checked out in the worktree
    pub branch: String,
    /// Whether the worktree is locked
    pub locked: bool,
    /// Whether the worktree is prunable (invalid/missing)
    pub prunable: bool,
}

impl From<RustWorktreeInfo> for NapiWorktreeInfo {
    fn from(w: RustWorktreeInfo) -> Self {
        Self {
            name: w.name,
            path: w.path,
            branch: w.branch,
            locked: w.locked,
            prunable: w.prunable,
        }
    }
}

// ============================================================================
// GitOpsHandle
// ============================================================================

/// Handle to a git repository for high-performance operations
#[napi]
pub struct GitOpsHandle {
    inner: Arc<Mutex<RustGitOpsHandle>>,
}

/// Check if a path is a git repository
#[napi]
pub fn is_git_repo(path: String) -> bool {
    RustGitOpsHandle::is_git_repo(&path)
}

/// Open an existing git repository
#[napi]
pub fn open_git_repo(path: String) -> Result<GitOpsHandle> {
    let handle = RustGitOpsHandle::open(&path).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(GitOpsHandle {
        inner: Arc::new(Mutex::new(handle)),
    })
}

/// Initialize a new git repository
#[napi]
pub fn init_git_repo(path: String, options: Option<NapiInitOptions>) -> Result<GitOpsHandle> {
    let opts = options.map(Into::into);
    let handle =
        RustGitOpsHandle::init(&path, opts).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(GitOpsHandle {
        inner: Arc::new(Mutex::new(handle)),
    })
}

/// Clone a git repository
#[napi]
pub fn clone_git_repo(
    url: String,
    path: String,
    options: Option<NapiCloneOptions>,
) -> Result<GitOpsHandle> {
    let opts = options.map(Into::into);
    let handle =
        RustGitOpsHandle::clone(&url, &path, opts).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(GitOpsHandle {
        inner: Arc::new(Mutex::new(handle)),
    })
}

#[napi]
impl GitOpsHandle {
    /// Get repository path
    #[napi]
    pub fn path(&self) -> String {
        let handle = self.inner.lock().unwrap();
        handle.path().to_string()
    }

    /// Get current git status
    #[napi]
    pub fn status(&self) -> Result<NapiGitStatus> {
        let handle = self.inner.lock().unwrap();
        let status = handle
            .status()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(status.into())
    }

    /// Get current branch name
    #[napi]
    pub fn current_branch(&self) -> Result<String> {
        let handle = self.inner.lock().unwrap();
        handle
            .current_branch()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if repository is clean
    #[napi]
    pub fn is_clean(&self) -> Result<bool> {
        let handle = self.inner.lock().unwrap();
        handle
            .is_clean()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Create a commit
    #[napi]
    pub fn commit(
        &self,
        message: String,
        add_all: Option<bool>,
        allow_empty: Option<bool>,
    ) -> Result<NapiCommitResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .commit(&message, add_all.unwrap_or(false), allow_empty.unwrap_or(false))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get list of commits
    #[napi]
    pub fn commits(&self, limit: Option<u32>) -> Result<Vec<NapiCommitInfo>> {
        let handle = self.inner.lock().unwrap();
        let commits = handle
            .commits(limit.unwrap_or(10) as usize)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(commits.into_iter().map(Into::into).collect())
    }

    /// Get current HEAD commit hash
    #[napi]
    pub fn current_commit(&self) -> Result<Option<String>> {
        let handle = self.inner.lock().unwrap();
        handle
            .current_commit()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Reset to a specific commit
    #[napi]
    pub fn reset(&self, commit_hash: String, hard: Option<bool>) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .reset(&commit_hash, hard.unwrap_or(true))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get diff between references
    #[napi]
    pub fn diff(
        &self,
        from: Option<String>,
        to: Option<String>,
    ) -> Result<NapiDiffResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .diff(from.as_deref(), to.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get changed files since a commit
    #[napi]
    pub fn changed_files(&self, since_commit: Option<String>) -> Result<Vec<String>> {
        let handle = self.inner.lock().unwrap();
        handle
            .changed_files(since_commit.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stash current changes
    #[napi]
    pub fn stash(&self, message: Option<String>) -> Result<NapiOperationResult> {
        let mut handle = self.inner.lock().unwrap();
        let result = handle
            .stash(message.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Pop the latest stash
    #[napi]
    pub fn stash_pop(&self) -> Result<NapiOperationResult> {
        let mut handle = self.inner.lock().unwrap();
        let result = handle
            .stash_pop()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// List stashes
    #[napi]
    pub fn stash_list(&self) -> Result<Vec<String>> {
        let mut handle = self.inner.lock().unwrap();
        handle
            .stash_list()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Add a remote
    #[napi]
    pub fn add_remote(&self, name: String, url: String) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .add_remote(&name, &url)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Remove a remote
    #[napi]
    pub fn remove_remote(&self, name: String) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .remove_remote(&name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get remote URL
    #[napi]
    pub fn remote_url(&self, name: String) -> Result<Option<String>> {
        let handle = self.inner.lock().unwrap();
        handle
            .remote_url(&name)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List remotes
    #[napi]
    pub fn remotes(&self) -> Result<Vec<String>> {
        let handle = self.inner.lock().unwrap();
        handle
            .remotes()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fetch from remote
    #[napi]
    pub fn fetch(&self, remote: Option<String>) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .fetch(&remote.unwrap_or_else(|| "origin".to_string()))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Push to remote
    #[napi]
    pub fn push(
        &self,
        remote: Option<String>,
        branch: Option<String>,
        set_upstream: Option<bool>,
    ) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let remote_name = remote.unwrap_or_else(|| "origin".to_string());
        let branch_name = branch.unwrap_or_else(|| {
            handle.current_branch().unwrap_or_else(|_| "main".to_string())
        });
        let result = handle
            .push(&remote_name, &branch_name, set_upstream.unwrap_or(true))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Create a new branch
    #[napi]
    pub fn create_branch(
        &self,
        name: String,
        from: Option<String>,
    ) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .create_branch(&name, from.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Checkout a branch or commit
    #[napi]
    pub fn checkout(&self, ref_name: String) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .checkout(&ref_name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Delete a branch
    #[napi]
    pub fn delete_branch(
        &self,
        name: String,
        force: Option<bool>,
    ) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .delete_branch(&name, force.unwrap_or(false))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// List branches
    #[napi]
    pub fn branches(&self, include_remote: Option<bool>) -> Result<Vec<String>> {
        let handle = self.inner.lock().unwrap();
        handle
            .branches(include_remote.unwrap_or(false))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stage specific files
    #[napi]
    pub fn stage_files(&self, paths: Vec<String>) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let result = handle
            .stage_files(&path_refs)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Unstage specific files
    #[napi]
    pub fn unstage_files(&self, paths: Vec<String>) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let result = handle
            .unstage_files(&path_refs)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    // ========================================================================
    // Worktree Operations
    // ========================================================================

    /// List all worktrees in the repository
    #[napi]
    pub fn list_worktrees(&self) -> Result<Vec<NapiWorktreeInfo>> {
        let handle = self.inner.lock().unwrap();
        let worktrees = handle
            .list_worktrees()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(worktrees.into_iter().map(Into::into).collect())
    }

    /// Add a new worktree
    ///
    /// Creates a worktree at the specified path with the given name.
    /// If branch is provided, the worktree will be checked out on that branch.
    /// If the branch doesn't exist, it will be created from HEAD.
    #[napi]
    pub fn add_worktree(
        &self,
        name: String,
        path: String,
        branch: Option<String>,
    ) -> Result<NapiWorktreeInfo> {
        let handle = self.inner.lock().unwrap();
        let worktree = handle
            .add_worktree(&name, &path, branch.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(worktree.into())
    }

    /// Remove a worktree
    ///
    /// If force is true, removes even if there are local changes.
    #[napi]
    pub fn remove_worktree(
        &self,
        name: String,
        force: Option<bool>,
    ) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .remove_worktree(&name, force.unwrap_or(false))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Prune stale worktree references
    ///
    /// Removes references to worktrees that no longer exist on disk.
    #[napi]
    pub fn prune_worktrees(&self) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .prune_worktrees()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Lock a worktree to prevent it from being pruned
    #[napi]
    pub fn lock_worktree(
        &self,
        name: String,
        reason: Option<String>,
    ) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .lock_worktree(&name, reason.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Unlock a worktree
    #[napi]
    pub fn unlock_worktree(&self, name: String) -> Result<NapiOperationResult> {
        let handle = self.inner.lock().unwrap();
        let result = handle
            .unlock_worktree(&name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get worktree info by name
    #[napi]
    pub fn get_worktree(&self, name: String) -> Result<Option<NapiWorktreeInfo>> {
        let handle = self.inner.lock().unwrap();
        let worktree = handle
            .get_worktree(&name)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(worktree.map(Into::into))
    }

    /// Check if a ref exists
    #[napi]
    pub fn ref_exists(&self, ref_name: String) -> bool {
        let handle = self.inner.lock().unwrap();
        handle.ref_exists(&ref_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_type_conversion() {
        let rust_type = RustFileStatusType::Modified;
        let napi_type: FileStatusType = rust_type.into();
        assert!(matches!(napi_type, FileStatusType::Modified));
    }
}
