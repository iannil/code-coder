//! Git operations module
//!
//! High-performance git operations using libgit2.
//! This module replaces the TypeScript git-ops.ts which spawned
//! child processes for each git command.

use git2::{
    BranchType, Cred, DiffFormat, DiffOptions, ErrorCode, FetchOptions,
    IndexAddOption, ObjectType, RemoteCallbacks, Repository, ResetType,
    Signature, StashFlags, Status, StatusOptions, StatusShow,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Git operation errors
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Repository not found at {0}")]
    RepoNotFound(String),

    #[error("Not a git repository: {0}")]
    NotARepo(String),

    #[error("Git operation failed: {0}")]
    Git2(#[from] git2::Error),

    #[error("No commits in repository")]
    NoCommits,

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Remote not found: {0}")]
    RemoteNotFound(String),

    #[error("Nothing to commit")]
    NothingToCommit,

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("Worktree already exists at {0}")]
    WorktreeExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for git operations
pub type GitResult<T> = Result<T, GitError>;

/// File status in git
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

impl From<Status> for FileStatusType {
    fn from(status: Status) -> Self {
        if status.is_index_new() || status.is_wt_new() {
            if status.is_wt_new() && !status.is_index_new() {
                FileStatusType::Untracked
            } else {
                FileStatusType::Added
            }
        } else if status.is_index_modified() || status.is_wt_modified() {
            FileStatusType::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            FileStatusType::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            FileStatusType::Renamed
        } else if status.is_index_typechange() || status.is_wt_typechange() {
            FileStatusType::Typechange
        } else if status.is_ignored() {
            FileStatusType::Ignored
        } else if status.is_conflicted() {
            FileStatusType::Conflicted
        } else {
            FileStatusType::Modified
        }
    }
}

/// Single file status entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: FileStatusType,
    pub staged: bool,
    pub old_path: Option<String>, // For renames
}

/// Git status result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub renamed: HashMap<String, String>, // old_path -> new_path
    pub untracked: Vec<String>,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileStatus>,
}

/// Git commit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub error: Option<String>,
}

/// Single commit info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: i64, // Unix timestamp in milliseconds
}

/// Diff file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub status: FileStatusType,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
}

/// Git diff result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffResult {
    pub files: Vec<DiffFile>,
    pub insertions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

/// Git init options
#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    pub default_branch: String,
    pub initial_commit: bool,
    pub commit_message: String,
}

impl InitOptions {
    pub fn new() -> Self {
        Self {
            default_branch: "main".to_string(),
            initial_commit: true,
            commit_message: "Initial commit".to_string(),
        }
    }
}

/// Git clone options
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    pub depth: Option<u32>,
    pub branch: Option<String>,
    pub reinitialize: bool, // Remove .git and reinit after clone
}

/// Worktree information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
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

/// Operation result for simple operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub error: Option<String>,
}

impl OperationResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(msg.into()),
        }
    }
}

/// Git operations handle
///
/// Provides high-performance git operations using libgit2.
/// Thread-safe wrapper around git2::Repository.
pub struct GitOpsHandle {
    repo: Repository,
    path: String,
}

impl GitOpsHandle {
    /// Open an existing git repository
    pub fn open(path: &str) -> GitResult<Self> {
        let repo = Repository::open(path).map_err(|e| {
            if e.code() == ErrorCode::NotFound {
                GitError::RepoNotFound(path.to_string())
            } else {
                GitError::Git2(e)
            }
        })?;

        Ok(Self {
            repo,
            path: path.to_string(),
        })
    }

    /// Check if a directory is a git repository
    pub fn is_git_repo(path: &str) -> bool {
        Repository::open(path).is_ok()
    }

    /// Initialize a new git repository
    pub fn init(path: &str, options: Option<InitOptions>) -> GitResult<Self> {
        let opts = options.unwrap_or_else(InitOptions::new);

        // Initialize the repository
        let repo = Repository::init(path)?;

        // Set the default branch name
        if !opts.default_branch.is_empty() && opts.default_branch != "master" {
            // Create initial commit first to establish HEAD
            if opts.initial_commit {
                let sig = Signature::now("CodeCoder", "codecoder@local")?;
                let tree_id = repo.index()?.write_tree()?;
                let tree = repo.find_tree(tree_id)?;
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &opts.commit_message,
                    &tree,
                    &[],
                )?;

                // Rename master to the desired branch name
                if let Ok(mut branch) = repo.find_branch("master", BranchType::Local) {
                    let _ = branch.rename(&opts.default_branch, true);
                }
            }
        }

        Ok(Self {
            repo,
            path: path.to_string(),
        })
    }

    /// Clone a repository
    pub fn clone(url: &str, path: &str, options: Option<CloneOptions>) -> GitResult<Self> {
        let opts = options.unwrap_or_default();

        let mut builder = git2::build::RepoBuilder::new();

        // Set up fetch options with depth if specified
        let mut fetch_opts = FetchOptions::new();
        if let Some(depth) = opts.depth {
            fetch_opts.depth(depth as i32);
        }
        builder.fetch_options(fetch_opts);

        // Set branch if specified
        if let Some(ref branch) = opts.branch {
            builder.branch(branch);
        }

        let repo = builder.clone(url, Path::new(path))?;

        let handle = Self {
            repo,
            path: path.to_string(),
        };

        // Reinitialize if requested (for template use)
        if opts.reinitialize {
            drop(handle);

            // Remove .git directory
            let git_dir = Path::new(path).join(".git");
            std::fs::remove_dir_all(&git_dir)?;

            // Reinitialize
            return Self::init(
                path,
                Some(InitOptions {
                    initial_commit: true,
                    commit_message: "[project-scaffold] Initial commit from template".to_string(),
                    ..Default::default()
                }),
            );
        }

        Ok(handle)
    }

    /// Get repository path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get current git status
    pub fn status(&self) -> GitResult<GitStatus> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .show(StatusShow::IndexAndWorkdir);

        let statuses = self.repo.statuses(Some(&mut opts))?;

        let mut result = GitStatus {
            branch: self.current_branch()?,
            ..Default::default()
        };

        // Get ahead/behind counts
        if let Ok((ahead, behind)) = self.ahead_behind() {
            result.ahead = ahead;
            result.behind = behind;
        }

        // Process each file status
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();
            let file_status = FileStatusType::from(status);

            let is_staged =
                status.is_index_new() || status.is_index_modified() || status.is_index_deleted();

            // Handle renames
            let old_path = entry
                .head_to_index()
                .and_then(|d| d.old_file().path())
                .map(|p| p.to_string_lossy().to_string());

            result.files.push(FileStatus {
                path: path.clone(),
                status: file_status.clone(),
                staged: is_staged,
                old_path: old_path.clone(),
            });

            // Populate legacy arrays
            match file_status {
                FileStatusType::Modified => result.modified.push(path),
                FileStatusType::Added => result.added.push(path),
                FileStatusType::Deleted => result.deleted.push(path),
                FileStatusType::Renamed => {
                    if let Some(old) = old_path {
                        result.renamed.insert(old, path);
                    }
                }
                FileStatusType::Untracked => result.untracked.push(path),
                _ => {}
            }
        }

        Ok(result)
    }

    /// Get current branch name
    pub fn current_branch(&self) -> GitResult<String> {
        match self.repo.head() {
            Ok(head) => {
                if head.is_branch() {
                    Ok(head
                        .shorthand()
                        .unwrap_or("HEAD")
                        .to_string())
                } else {
                    // Detached HEAD
                    Ok("HEAD".to_string())
                }
            }
            Err(e) if e.code() == ErrorCode::UnbornBranch => Ok("main".to_string()),
            Err(e) => Err(GitError::Git2(e)),
        }
    }

    /// Get ahead/behind counts relative to upstream
    fn ahead_behind(&self) -> GitResult<(u32, u32)> {
        let head = self.repo.head()?;
        let local = head.peel_to_commit()?.id();

        // Try to find upstream
        let branch_name = head.shorthand().unwrap_or("main");
        let upstream_ref = format!("refs/remotes/origin/{}", branch_name);

        match self.repo.find_reference(&upstream_ref) {
            Ok(upstream) => {
                let remote = upstream.peel_to_commit()?.id();
                let (ahead, behind) = self.repo.graph_ahead_behind(local, remote)?;
                Ok((ahead as u32, behind as u32))
            }
            Err(_) => Ok((0, 0)), // No upstream
        }
    }

    /// Check if repository is clean (no uncommitted changes)
    pub fn is_clean(&self) -> GitResult<bool> {
        let status = self.status()?;
        Ok(status.modified.is_empty()
            && status.added.is_empty()
            && status.deleted.is_empty()
            && status.renamed.is_empty())
    }

    /// Create a commit
    pub fn commit(
        &self,
        message: &str,
        add_all: bool,
        allow_empty: bool,
    ) -> GitResult<CommitResult> {
        // Stage all changes if requested
        if add_all {
            let mut index = self.repo.index()?;
            index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
            index.write()?;
        }

        // Check if there are changes to commit
        let status = self.status()?;
        let has_staged = status.files.iter().any(|f| f.staged);

        if !has_staged && !allow_empty {
            return Ok(CommitResult {
                success: false,
                commit_hash: None,
                error: Some("Nothing to commit".to_string()),
            });
        }

        // Create the commit
        let sig = self.default_signature()?;
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let parent_commit = match self.repo.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(e) if e.code() == ErrorCode::UnbornBranch => None,
            Err(e) => return Err(GitError::Git2(e)),
        };

        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
        let full_message = format!("[autonomous-mode] {}", message);

        let oid = self
            .repo
            .commit(Some("HEAD"), &sig, &sig, &full_message, &tree, &parents)?;

        Ok(CommitResult {
            success: true,
            commit_hash: Some(oid.to_string()),
            error: None,
        })
    }

    /// Get list of recent commits
    pub fn commits(&self, limit: usize) -> GitResult<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();

        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }

            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            commits.push(CommitInfo {
                hash: oid.to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                date: commit.time().seconds() * 1000,
            });
        }

        Ok(commits)
    }

    /// Get current HEAD commit hash
    pub fn current_commit(&self) -> GitResult<Option<String>> {
        match self.repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit()?;
                Ok(Some(commit.id().to_string()))
            }
            Err(e) if e.code() == ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(GitError::Git2(e)),
        }
    }

    /// Reset to a specific commit
    pub fn reset(&self, commit_hash: &str, hard: bool) -> GitResult<OperationResult> {
        let oid = git2::Oid::from_str(commit_hash)?;
        let commit = self.repo.find_commit(oid)?;
        let obj = commit.as_object();

        let reset_type = if hard {
            ResetType::Hard
        } else {
            ResetType::Soft
        };

        self.repo.reset(obj, reset_type, None)?;
        Ok(OperationResult::ok())
    }

    /// Get diff between two commits or working tree
    pub fn diff(&self, from: Option<&str>, to: Option<&str>) -> GitResult<DiffResult> {
        let from_tree = match from {
            Some(ref_name) => {
                let obj = self.repo.revparse_single(ref_name)?;
                Some(obj.peel_to_tree()?)
            }
            None => None,
        };

        let to_tree = match to {
            Some(ref_name) => {
                let obj = self.repo.revparse_single(ref_name)?;
                Some(obj.peel_to_tree()?)
            }
            None => None,
        };

        let mut opts = DiffOptions::new();
        let diff = self
            .repo
            .diff_tree_to_tree(from_tree.as_ref(), to_tree.as_ref(), Some(&mut opts))?;

        let stats = diff.stats()?;
        let mut result = DiffResult {
            insertions: stats.insertions() as u32,
            deletions: stats.deletions() as u32,
            files_changed: stats.files_changed() as u32,
            files: Vec::new(),
        };

        // Get per-file diffs
        let mut current_file: Option<DiffFile> = None;
        let mut patch_lines: Vec<String> = Vec::new();

        diff.print(DiffFormat::Patch, |delta, _hunk, line| {
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());

            let path = new_path.or(old_path).unwrap_or_default();

            // Check if we're starting a new file
            if current_file.as_ref().map(|f| &f.path) != Some(&path) {
                // Save previous file
                if let Some(mut file) = current_file.take() {
                    file.patch = Some(patch_lines.join(""));
                    result.files.push(file);
                }
                patch_lines.clear();

                // Start new file
                let status = match delta.status() {
                    git2::Delta::Added => FileStatusType::Added,
                    git2::Delta::Deleted => FileStatusType::Deleted,
                    git2::Delta::Modified => FileStatusType::Modified,
                    git2::Delta::Renamed => FileStatusType::Renamed,
                    git2::Delta::Copied => FileStatusType::Copied,
                    _ => FileStatusType::Modified,
                };

                current_file = Some(DiffFile {
                    path,
                    status,
                    additions: 0,
                    deletions: 0,
                    patch: None,
                });
            }

            // Count additions/deletions and collect patch
            if let Some(ref mut file) = current_file {
                let content = String::from_utf8_lossy(line.content());
                let origin = line.origin();

                match origin {
                    '+' => {
                        file.additions += 1;
                        patch_lines.push(format!("+{}", content));
                    }
                    '-' => {
                        file.deletions += 1;
                        patch_lines.push(format!("-{}", content));
                    }
                    ' ' => {
                        patch_lines.push(format!(" {}", content));
                    }
                    '@' => {
                        patch_lines.push(format!("@{}", content));
                    }
                    _ => {}
                }
            }

            true
        })?;

        // Don't forget the last file
        if let Some(mut file) = current_file.take() {
            file.patch = Some(patch_lines.join(""));
            result.files.push(file);
        }

        Ok(result)
    }

    /// Get changed files since a commit
    pub fn changed_files(&self, since_commit: Option<&str>) -> GitResult<Vec<String>> {
        let diff_result = self.diff(since_commit, None)?;
        Ok(diff_result.files.into_iter().map(|f| f.path).collect())
    }

    /// Stash current changes
    pub fn stash(&mut self, message: Option<&str>) -> GitResult<OperationResult> {
        let sig = self.default_signature()?;
        let msg = message.unwrap_or("WIP");

        self.repo
            .stash_save(&sig, msg, Some(StashFlags::DEFAULT))?;

        Ok(OperationResult::ok())
    }

    /// Pop the latest stash
    pub fn stash_pop(&mut self) -> GitResult<OperationResult> {
        self.repo.stash_pop(0, None)?;
        Ok(OperationResult::ok())
    }

    /// List stashes
    pub fn stash_list(&mut self) -> GitResult<Vec<String>> {
        let mut stashes = Vec::new();
        self.repo.stash_foreach(|index, message, _oid| {
            stashes.push(format!("stash@{{{}}}: {}", index, message));
            true
        })?;
        Ok(stashes)
    }

    /// Add a remote
    pub fn add_remote(&self, name: &str, url: &str) -> GitResult<OperationResult> {
        self.repo.remote(name, url)?;
        Ok(OperationResult::ok())
    }

    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> GitResult<OperationResult> {
        self.repo.remote_delete(name)?;
        Ok(OperationResult::ok())
    }

    /// Get remote URL
    pub fn remote_url(&self, name: &str) -> GitResult<Option<String>> {
        match self.repo.find_remote(name) {
            Ok(remote) => Ok(remote.url().map(|s| s.to_string())),
            Err(_) => Ok(None),
        }
    }

    /// List remotes
    pub fn remotes(&self) -> GitResult<Vec<String>> {
        let remotes = self.repo.remotes()?;
        Ok(remotes.iter().filter_map(|r| r.map(String::from)).collect())
    }

    /// Fetch from remote
    pub fn fetch(&self, remote: &str) -> GitResult<OperationResult> {
        let mut remote_obj = self.repo.find_remote(remote)?;
        let mut opts = FetchOptions::new();
        let mut callbacks = RemoteCallbacks::new();

        // Set up credential callback for authentication
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        opts.remote_callbacks(callbacks);
        remote_obj.fetch(&[] as &[&str], Some(&mut opts), None)?;

        Ok(OperationResult::ok())
    }

    /// Push to remote
    pub fn push(&self, remote: &str, branch: &str, set_upstream: bool) -> GitResult<OperationResult> {
        let mut remote_obj = self.repo.find_remote(remote)?;
        let mut opts = git2::PushOptions::new();
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        opts.remote_callbacks(callbacks);

        let refspec = if set_upstream {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        remote_obj.push(&[&refspec], Some(&mut opts))?;

        Ok(OperationResult::ok())
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str, from: Option<&str>) -> GitResult<OperationResult> {
        let commit = match from {
            Some(ref_name) => {
                let obj = self.repo.revparse_single(ref_name)?;
                obj.peel_to_commit()?
            }
            None => self.repo.head()?.peel_to_commit()?,
        };

        self.repo.branch(name, &commit, false)?;
        Ok(OperationResult::ok())
    }

    /// Checkout a branch or commit
    pub fn checkout(&self, ref_name: &str) -> GitResult<OperationResult> {
        // Try to find as branch first
        if let Ok(branch) = self.repo.find_branch(ref_name, BranchType::Local) {
            let obj = branch.get().peel(ObjectType::Commit)?;
            self.repo.checkout_tree(&obj, None)?;
            self.repo
                .set_head(&format!("refs/heads/{}", ref_name))?;
            return Ok(OperationResult::ok());
        }

        // Try as remote branch
        if let Ok(branch) = self.repo.find_branch(&format!("origin/{}", ref_name), BranchType::Remote) {
            let obj = branch.get().peel(ObjectType::Commit)?;
            self.repo.checkout_tree(&obj, None)?;
            // Create local tracking branch
            let commit = branch.get().peel_to_commit()?;
            self.repo.branch(ref_name, &commit, false)?;
            self.repo.set_head(&format!("refs/heads/{}", ref_name))?;
            return Ok(OperationResult::ok());
        }

        // Try as commit hash
        let obj = self.repo.revparse_single(ref_name)?;
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head_detached(obj.id())?;

        Ok(OperationResult::ok())
    }

    /// Delete a branch
    pub fn delete_branch(&self, name: &str, force: bool) -> GitResult<OperationResult> {
        let mut branch = self.repo.find_branch(name, BranchType::Local)?;

        if force || branch.is_head() {
            branch.delete()?;
        } else {
            // Check if branch is merged before deleting
            let head = self.repo.head()?.peel_to_commit()?;
            let branch_commit = branch.get().peel_to_commit()?;

            if self.repo.merge_base(head.id(), branch_commit.id()).is_ok() {
                branch.delete()?;
            } else {
                return Ok(OperationResult::err(
                    format!("Branch '{}' is not merged. Use force to delete.", name)
                ));
            }
        }

        Ok(OperationResult::ok())
    }

    /// List branches
    pub fn branches(&self, include_remote: bool) -> GitResult<Vec<String>> {
        let mut branches = Vec::new();

        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }

        if include_remote {
            for branch in self.repo.branches(Some(BranchType::Remote))? {
                let (branch, _) = branch?;
                if let Some(name) = branch.name()? {
                    branches.push(name.to_string());
                }
            }
        }

        Ok(branches)
    }

    /// Stage specific files
    pub fn stage_files(&self, paths: &[&str]) -> GitResult<OperationResult> {
        let mut index = self.repo.index()?;
        for path in paths {
            index.add_path(Path::new(path))?;
        }
        index.write()?;
        Ok(OperationResult::ok())
    }

    /// Unstage specific files
    pub fn unstage_files(&self, paths: &[&str]) -> GitResult<OperationResult> {
        let head = self.repo.head()?.peel_to_commit()?;

        for path in paths {
            self.repo.reset_default(Some(&head.as_object()), [*path].iter())?;
        }

        Ok(OperationResult::ok())
    }

    /// Get default signature
    fn default_signature(&self) -> GitResult<Signature<'static>> {
        // Try to get from git config
        if let Ok(sig) = self.repo.signature() {
            return Ok(Signature::now(
                sig.name().unwrap_or("CodeCoder"),
                sig.email().unwrap_or("codecoder@local"),
            )?);
        }

        // Fallback to default
        Ok(Signature::now("CodeCoder", "codecoder@local")?)
    }

    // ========================================================================
    // Worktree Operations
    // ========================================================================

    /// Helper to check if a worktree is locked
    fn is_worktree_locked(wt: &git2::Worktree) -> bool {
        matches!(wt.is_locked(), Ok(git2::WorktreeLockStatus::Locked(_)))
    }

    /// List all worktrees in the repository
    pub fn list_worktrees(&self) -> GitResult<Vec<WorktreeInfo>> {
        let worktrees = self.repo.worktrees()?;
        let mut result = Vec::new();

        for name in worktrees.iter() {
            if let Some(name) = name {
                if let Ok(wt) = self.repo.find_worktree(name) {
                    let path = wt.path().to_string_lossy().to_string();

                    // Try to get the branch name from the worktree
                    let branch = if let Ok(wt_repo) = Repository::open(wt.path()) {
                        wt_repo.head()
                            .ok()
                            .and_then(|h| h.shorthand().map(String::from))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let locked = Self::is_worktree_locked(&wt);
                    let prunable = wt.validate().is_err();

                    result.push(WorktreeInfo {
                        name: name.to_string(),
                        path,
                        branch,
                        locked,
                        prunable,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Add a new worktree
    ///
    /// Creates a new worktree at the specified path, optionally creating a new branch.
    /// If branch is None, creates a detached HEAD worktree.
    /// If branch is Some and doesn't exist, creates a new branch from HEAD.
    pub fn add_worktree(&self, name: &str, path: &str, branch: Option<&str>) -> GitResult<WorktreeInfo> {
        // Check if worktree already exists
        if let Ok(existing) = self.repo.find_worktree(name) {
            return Err(GitError::WorktreeExists(existing.path().to_string_lossy().to_string()));
        }

        // Ensure the parent directory exists, but NOT the worktree directory itself
        // (git2 will create it)
        let worktree_path = Path::new(path);
        if let Some(parent) = worktree_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Get HEAD commit for the worktree
        let head_commit = self.repo.head()?.peel_to_commit()?;

        // Handle branch creation/checkout
        let branch_name = if let Some(branch) = branch {
            // Check if branch exists
            if self.repo.find_branch(branch, BranchType::Local).is_ok() {
                // Branch exists, use it
                branch.to_string()
            } else {
                // Create new branch from HEAD
                self.repo.branch(branch, &head_commit, false)?;
                branch.to_string()
            }
        } else {
            // Use detached HEAD - create a temporary branch name
            let temp_name = format!("worktree/{}", name);
            // Create the branch if it doesn't exist
            if self.repo.find_branch(&temp_name, BranchType::Local).is_err() {
                self.repo.branch(&temp_name, &head_commit, false)?;
            }
            temp_name
        };

        // Get the reference for the branch
        let ref_name = format!("refs/heads/{}", branch_name);
        let reference = self.repo.find_reference(&ref_name)?;

        // Create the worktree
        let wt = self.repo.worktree(
            name,
            worktree_path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference)))
        )?;

        let locked = Self::is_worktree_locked(&wt);
        let prunable = wt.validate().is_err();

        Ok(WorktreeInfo {
            name: name.to_string(),
            path: path.to_string(),
            branch: branch_name,
            locked,
            prunable,
        })
    }

    /// Remove a worktree
    ///
    /// If force is true, removes even if there are local changes.
    pub fn remove_worktree(&self, name: &str, force: bool) -> GitResult<OperationResult> {
        let wt = self.repo.find_worktree(name).map_err(|_| {
            GitError::WorktreeNotFound(name.to_string())
        })?;

        let wt_path = wt.path().to_path_buf();

        // Check if worktree has local changes
        if !force {
            if let Ok(wt_repo) = Repository::open(&wt_path) {
                let status = wt_repo.statuses(None)?;
                if !status.is_empty() {
                    return Ok(OperationResult::err(
                        "Worktree has local changes. Use force to remove."
                    ));
                }
            }
        }

        // Unlock the worktree if it's locked (so we can prune it)
        if Self::is_worktree_locked(&wt) {
            let _ = wt.unlock();
        }

        // Prune the worktree (removes the reference)
        // Use working_tree(true), valid(true), and locked(true) to force removal
        let mut prune_opts = git2::WorktreePruneOptions::new();
        prune_opts.working_tree(true);
        prune_opts.valid(true);  // Allow pruning even if worktree is valid
        if force {
            prune_opts.locked(true);
        }
        wt.prune(Some(&mut prune_opts))?;

        // Remove the worktree directory
        if wt_path.exists() {
            std::fs::remove_dir_all(&wt_path)?;
        }

        Ok(OperationResult::ok())
    }

    /// Prune stale worktree references
    ///
    /// Removes references to worktrees that no longer exist on disk.
    pub fn prune_worktrees(&self) -> GitResult<OperationResult> {
        let worktrees = self.repo.worktrees()?;

        for name in worktrees.iter() {
            if let Some(name) = name {
                if let Ok(wt) = self.repo.find_worktree(name) {
                    // Check if worktree path exists
                    if wt.validate().is_err() {
                        // Worktree is invalid (path doesn't exist or is corrupted)
                        let mut prune_opts = git2::WorktreePruneOptions::new();
                        prune_opts.working_tree(true);
                        let _ = wt.prune(Some(&mut prune_opts));
                    }
                }
            }
        }

        Ok(OperationResult::ok())
    }

    /// Lock a worktree to prevent it from being pruned
    pub fn lock_worktree(&self, name: &str, reason: Option<&str>) -> GitResult<OperationResult> {
        let wt = self.repo.find_worktree(name).map_err(|_| {
            GitError::WorktreeNotFound(name.to_string())
        })?;

        if Self::is_worktree_locked(&wt) {
            return Ok(OperationResult::err("Worktree is already locked"));
        }

        wt.lock(reason)?;
        Ok(OperationResult::ok())
    }

    /// Unlock a worktree
    pub fn unlock_worktree(&self, name: &str) -> GitResult<OperationResult> {
        let wt = self.repo.find_worktree(name).map_err(|_| {
            GitError::WorktreeNotFound(name.to_string())
        })?;

        if !Self::is_worktree_locked(&wt) {
            return Ok(OperationResult::err("Worktree is not locked"));
        }

        wt.unlock()?;
        Ok(OperationResult::ok())
    }

    /// Get worktree info by name
    pub fn get_worktree(&self, name: &str) -> GitResult<Option<WorktreeInfo>> {
        match self.repo.find_worktree(name) {
            Ok(wt) => {
                let path = wt.path().to_string_lossy().to_string();

                // Try to get the branch name from the worktree
                let branch = if let Ok(wt_repo) = Repository::open(wt.path()) {
                    wt_repo.head()
                        .ok()
                        .and_then(|h| h.shorthand().map(String::from))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let locked = Self::is_worktree_locked(&wt);
                let prunable = wt.validate().is_err();

                Ok(Some(WorktreeInfo {
                    name: name.to_string(),
                    path,
                    branch,
                    locked,
                    prunable,
                }))
            }
            Err(_) => Ok(None),
        }
    }

    /// Check if a ref exists
    pub fn ref_exists(&self, ref_name: &str) -> bool {
        self.repo.find_reference(ref_name).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitOpsHandle) {
        let dir = TempDir::new().unwrap();
        let handle = GitOpsHandle::init(dir.path().to_str().unwrap(), None).unwrap();
        (dir, handle)
    }

    #[test]
    fn test_init_and_open() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();

        // Init
        let handle = GitOpsHandle::init(path, None).unwrap();
        assert!(GitOpsHandle::is_git_repo(path));

        // Open
        let handle2 = GitOpsHandle::open(path).unwrap();
        assert_eq!(handle.path(), handle2.path());
    }

    #[test]
    fn test_status_empty_repo() {
        let (_dir, handle) = setup_test_repo();
        let status = handle.status().unwrap();
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_commit() {
        let (dir, handle) = setup_test_repo();

        // Create a file
        std::fs::write(dir.path().join("test.txt"), "Hello").unwrap();

        // Commit with add_all
        let result = handle.commit("Test commit", true, false).unwrap();
        assert!(result.success);
        assert!(result.commit_hash.is_some());

        // Check status is clean
        let status = handle.status().unwrap();
        assert!(status.modified.is_empty());
    }

    #[test]
    fn test_branches() {
        let (_dir, handle) = setup_test_repo();

        // Create a branch
        handle.create_branch("feature", None).unwrap();

        let branches = handle.branches(false).unwrap();
        assert!(branches.contains(&"main".to_string()) || branches.contains(&"master".to_string()));
        assert!(branches.contains(&"feature".to_string()));
    }

    #[test]
    fn test_current_branch() {
        let (_dir, handle) = setup_test_repo();
        let branch = handle.current_branch().unwrap();
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_is_clean() {
        let (dir, handle) = setup_test_repo();
        assert!(handle.is_clean().unwrap());

        // Create and commit a file first
        std::fs::write(dir.path().join("tracked.txt"), "initial").unwrap();
        handle.commit("Add tracked file", true, false).unwrap();
        assert!(handle.is_clean().unwrap());

        // Modify the tracked file - now repo should be dirty
        std::fs::write(dir.path().join("tracked.txt"), "modified").unwrap();
        assert!(!handle.is_clean().unwrap());
    }

    #[test]
    fn test_list_worktrees_empty() {
        let (_dir, handle) = setup_test_repo();
        let worktrees = handle.list_worktrees().unwrap();
        // New repo should have no worktrees (only the main working directory)
        assert!(worktrees.is_empty());
    }

    #[test]
    fn test_add_and_list_worktree() {
        let (dir, handle) = setup_test_repo();

        // Create a file and commit first
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        handle.commit("Initial commit", true, false).unwrap();

        // Create a worktree
        let wt_path = dir.path().join("wt-feature");
        let result = handle.add_worktree(
            "feature",
            wt_path.to_str().unwrap(),
            Some("feature-branch")
        ).unwrap();

        assert_eq!(result.name, "feature");
        assert_eq!(result.branch, "feature-branch");
        assert!(!result.locked);
        assert!(!result.prunable);

        // List worktrees
        let worktrees = handle.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].name, "feature");
    }

    #[test]
    fn test_get_worktree() {
        let (dir, handle) = setup_test_repo();

        // Create a file and commit first
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        handle.commit("Initial commit", true, false).unwrap();

        // Create a worktree
        let wt_path = dir.path().join("wt-test");
        handle.add_worktree(
            "test-wt",
            wt_path.to_str().unwrap(),
            Some("test-branch")
        ).unwrap();

        // Get the worktree
        let wt = handle.get_worktree("test-wt").unwrap();
        assert!(wt.is_some());
        let wt = wt.unwrap();
        assert_eq!(wt.name, "test-wt");
        assert_eq!(wt.branch, "test-branch");

        // Get non-existent worktree
        let none = handle.get_worktree("nonexistent").unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn test_remove_worktree() {
        let (dir, handle) = setup_test_repo();

        // Create a file and commit first
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        handle.commit("Initial commit", true, false).unwrap();

        // Create a worktree
        let wt_path = dir.path().join("wt-remove");
        handle.add_worktree(
            "remove-wt",
            wt_path.to_str().unwrap(),
            Some("remove-branch")
        ).unwrap();

        // Verify it exists
        assert_eq!(handle.list_worktrees().unwrap().len(), 1);

        // Remove the worktree
        let result = handle.remove_worktree("remove-wt", true).unwrap();
        assert!(result.success);

        // Verify it's gone
        assert!(handle.list_worktrees().unwrap().is_empty());
    }

    #[test]
    fn test_lock_unlock_worktree() {
        let (dir, handle) = setup_test_repo();

        // Create a file and commit first
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        handle.commit("Initial commit", true, false).unwrap();

        // Create a worktree
        let wt_path = dir.path().join("wt-lock");
        handle.add_worktree(
            "lock-wt",
            wt_path.to_str().unwrap(),
            Some("lock-branch")
        ).unwrap();

        // Lock the worktree
        let lock_result = handle.lock_worktree("lock-wt", Some("Testing")).unwrap();
        assert!(lock_result.success);

        // Verify it's locked
        let wt = handle.get_worktree("lock-wt").unwrap().unwrap();
        assert!(wt.locked);

        // Unlock the worktree
        let unlock_result = handle.unlock_worktree("lock-wt").unwrap();
        assert!(unlock_result.success);

        // Verify it's unlocked
        let wt = handle.get_worktree("lock-wt").unwrap().unwrap();
        assert!(!wt.locked);
    }
}
