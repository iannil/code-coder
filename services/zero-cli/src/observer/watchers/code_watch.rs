//! Code Watcher (CodeWatch)
//!
//! Observes the codebase for changes including:
//! - Git commits and changes
//! - Build status
//! - Test coverage
//! - Technical debt indicators
//! - File changes
//!
//! Uses the `git2` crate for native Git operations.

use crate::observer::types::{
    Change, CodeObservation, CodeObservationType, Impact, Observation, Severity, WatcherType,
};
use crate::observer::watchers::{
    BaseWatcherState, Watcher, WatcherMetrics, WatcherOptions, WatcherStatus,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, info, warn};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for CodeWatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeWatchConfig {
    /// Watch specific paths
    #[serde(default)]
    pub watch_paths: Vec<PathBuf>,
    /// Git repository root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_root: Option<PathBuf>,
    /// Track build status
    #[serde(default = "default_true")]
    pub track_build: bool,
    /// Track test coverage
    #[serde(default)]
    pub track_tests: bool,
    /// Enable periodic typecheck
    #[serde(default)]
    pub enable_typecheck: bool,
    /// Typecheck interval in ms (default: 60000)
    #[serde(default = "default_typecheck_interval")]
    pub typecheck_interval_ms: u64,
    /// Typecheck timeout in ms (default: 30000)
    #[serde(default = "default_typecheck_timeout")]
    pub typecheck_timeout_ms: u64,
    /// Common watcher options
    #[serde(flatten)]
    pub options: WatcherOptions,
}

fn default_true() -> bool {
    true
}

fn default_typecheck_interval() -> u64 {
    60000
}

fn default_typecheck_timeout() -> u64 {
    30000
}

impl Default for CodeWatchConfig {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            git_root: None,
            track_build: true,
            track_tests: false,
            enable_typecheck: false,
            typecheck_interval_ms: 60000,
            typecheck_timeout_ms: 30000,
            options: WatcherOptions {
                interval_ms: 30000, // Check every 30 seconds
                ..Default::default()
            },
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal Types
// ══════════════════════════════════════════════════════════════════════════════

/// Git change information.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GitChange {
    hash: String,
    message: String,
    author: String,
    files: Vec<FileChange>,
}

/// File change information.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FileChange {
    path: String,
    action: FileAction,
    additions: Option<u32>,
    deletions: Option<u32>,
}

/// File action type.
#[derive(Debug, Clone, Copy)]
enum FileAction {
    Add,
    Modify,
    Delete,
}

// ══════════════════════════════════════════════════════════════════════════════
// CodeWatch Implementation
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher that observes codebase changes.
pub struct CodeWatch {
    /// Base watcher state
    state: BaseWatcherState,
    /// Configuration
    config: CodeWatchConfig,
    /// Last known commit hash
    last_commit_hash: Option<String>,
    /// Resolved Git root path
    git_root: Option<PathBuf>,
    /// Last typecheck time
    last_typecheck: Option<Instant>,
}

impl CodeWatch {
    /// Create a new CodeWatch instance.
    pub fn new(config: CodeWatchConfig) -> Self {
        let id = config
            .options
            .id
            .clone()
            .unwrap_or_else(|| BaseWatcherState::generate_id("code"));

        Self {
            state: BaseWatcherState::new(id),
            config,
            last_commit_hash: None,
            git_root: None,
            last_typecheck: None,
        }
    }

    /// Initialize Git repository.
    async fn init_git(&mut self) -> anyhow::Result<()> {
        // Use configured git root or discover from current directory
        let git_root = if let Some(ref root) = self.config.git_root {
            root.clone()
        } else {
            self.discover_git_root().await?
        };

        self.git_root = Some(git_root);

        // Get current HEAD
        self.last_commit_hash = self.get_current_commit_hash().await.ok();

        info!(
            watcher_id = %self.state.id,
            git_root = ?self.git_root,
            last_commit = ?self.last_commit_hash,
            "CodeWatch initialized"
        );

        Ok(())
    }

    /// Discover Git repository root.
    async fn discover_git_root(&self) -> anyhow::Result<PathBuf> {
        // Use git2 to discover the repository
        let repo = git2::Repository::discover(".")?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("No working directory found"))?;
        Ok(workdir.to_path_buf())
    }

    /// Get current commit hash.
    async fn get_current_commit_hash(&self) -> anyhow::Result<String> {
        let git_root = self.git_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Git root not initialized")
        })?;

        let repo = git2::Repository::open(git_root)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Get Git changes since last known commit.
    async fn get_git_changes(&self, from_hash: &str, to_hash: &str) -> anyhow::Result<Vec<GitChange>> {
        let git_root = self.git_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Git root not initialized")
        })?;

        let repo = git2::Repository::open(git_root)?;

        // Parse commit OIDs
        let from_oid = git2::Oid::from_str(from_hash)?;
        let to_oid = git2::Oid::from_str(to_hash)?;

        let from_commit = repo.find_commit(from_oid)?;
        let to_commit = repo.find_commit(to_oid)?;

        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;

        // Get diff
        let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

        // Collect file changes
        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let path = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let action = match delta.status() {
                    git2::Delta::Added => FileAction::Add,
                    git2::Delta::Deleted => FileAction::Delete,
                    _ => FileAction::Modify,
                };

                files.push(FileChange {
                    path,
                    action,
                    additions: None,
                    deletions: None,
                });
                true
            },
            None,
            None,
            None,
        )?;

        // Create git change record
        let change = GitChange {
            hash: to_hash.to_string(),
            message: to_commit
                .message()
                .unwrap_or("No message")
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
            author: to_commit.author().name().unwrap_or("Unknown").to_string(),
            files,
        };

        Ok(vec![change])
    }

    /// Create a code observation.
    fn create_observation(
        &self,
        obs_type: CodeObservationType,
        source: impl Into<String>,
        change: Change,
    ) -> CodeObservation {
        CodeObservation::new(&self.state.id, obs_type, source).with_change(change)
    }

    /// Create git change observation.
    fn create_git_change_observation(&self, changes: &[GitChange]) -> CodeObservation {
        let latest = changes.first().expect("Changes should not be empty");

        let diff_summary = changes
            .iter()
            .map(|c| format!("{}: {}", &c.hash[..7], c.message))
            .collect::<Vec<_>>()
            .join("\n");

        let affected_files: Vec<String> = changes
            .iter()
            .flat_map(|c| c.files.iter().map(|f| f.path.clone()))
            .collect();

        let mut obs = self.create_observation(
            CodeObservationType::GitChange,
            self.git_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string()),
            Change {
                action: "modify".to_string(),
                before: self.last_commit_hash.clone().map(serde_json::Value::String),
                after: Some(serde_json::Value::String(latest.hash.clone())),
                diff: Some(diff_summary),
            },
        );

        obs.impact = Impact {
            scope: self.determine_scope(&affected_files),
            severity: Severity::Low,
            affected_files,
        };

        obs
    }

    /// Determine impact scope from affected files.
    fn determine_scope(&self, files: &[String]) -> String {
        for file in files {
            if file.contains("package.json") || file.contains("tsconfig") || file.contains("Cargo.toml") {
                return "project".to_string();
            }
        }

        // Files nested 2+ levels deep (e.g., src/utils/helper.ts) are "module"
        if files.iter().any(|f| f.matches('/').count() >= 2) {
            return "module".to_string();
        }

        if files.iter().any(|f| f.contains('/')) {
            return "package".to_string();
        }

        "file".to_string()
    }

    /// Run typecheck and create observation.
    async fn run_typecheck(&mut self) -> Option<Observation> {
        let git_root = self.git_root.as_ref()?;

        let start = Instant::now();

        // Run bun turbo typecheck
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.typecheck_timeout_ms),
            Command::new("bun")
                .args(["turbo", "typecheck", "--quiet"])
                .current_dir(git_root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        self.last_typecheck = Some(Instant::now());

        match result {
            Ok(Ok(output)) => {
                let elapsed = start.elapsed().as_millis() as u64;

                if output.status.success() {
                    // Build passing
                    let obs = self.create_build_status_observation(true, None);
                    self.state.record_observation(elapsed);
                    Some(Observation::Code(obs))
                } else {
                    // Parse type errors
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let combined = format!("{stderr}{stdout}");

                    let errors = self.parse_type_errors(&combined);
                    if !errors.is_empty() {
                        let obs = self.create_type_error_observation(errors);
                        self.state.record_observation(elapsed);
                        Some(Observation::Code(obs))
                    } else {
                        let obs = self.create_build_status_observation(false, Some(combined));
                        self.state.record_observation(elapsed);
                        Some(Observation::Code(obs))
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(
                    watcher_id = %self.state.id,
                    error = %e,
                    "Failed to run typecheck"
                );
                self.state.record_error();
                None
            }
            Err(_) => {
                warn!(
                    watcher_id = %self.state.id,
                    "Typecheck timed out"
                );
                self.state.record_error();
                None
            }
        }
    }

    /// Parse TypeScript errors from compiler output.
    fn parse_type_errors(&self, output: &str) -> Vec<TypeErrorInfo> {
        let mut errors = Vec::new();

        // Match TypeScript error formats:
        // path/file.ts(line,col): error TSxxxx: message
        // path/file.ts:line:col - error TSxxxx: message
        let patterns = [
            regex::Regex::new(r"([^:\s]+\.tsx?)\((\d+),\d+\):\s*error\s+TS\d+:\s*(.+)").ok(),
            regex::Regex::new(r"([^:\s]+\.tsx?):(\d+):\d+\s*-\s*error\s+TS\d+:\s*(.+)").ok(),
        ];

        for pattern in patterns.into_iter().flatten() {
            for caps in pattern.captures_iter(output) {
                if let (Some(file), Some(line), Some(message)) =
                    (caps.get(1), caps.get(2), caps.get(3))
                {
                    errors.push(TypeErrorInfo {
                        file: file.as_str().to_string(),
                        line: line.as_str().parse().unwrap_or(0),
                        message: message.as_str().trim().to_string(),
                    });
                }
            }
        }

        errors
    }

    /// Create build status observation.
    fn create_build_status_observation(
        &self,
        passing: bool,
        error_output: Option<String>,
    ) -> CodeObservation {
        let status = if passing { "passing" } else { "failing" };

        let mut obs = self.create_observation(
            CodeObservationType::BuildStatus,
            "build",
            Change {
                action: "modify".to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "status": status,
                    "errors": error_output,
                })),
                diff: None,
            },
        );

        obs.impact.severity = if passing {
            Severity::Low
        } else {
            Severity::High
        };
        obs.base.confidence = 1.0;

        obs
    }

    /// Create type error observation.
    fn create_type_error_observation(&self, errors: Vec<TypeErrorInfo>) -> CodeObservation {
        let affected_files: Vec<String> = errors
            .iter()
            .map(|e| e.file.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut obs = self.create_observation(
            CodeObservationType::TypeError,
            "typescript",
            Change {
                action: "modify".to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "errors": errors,
                    "count": errors.len(),
                })),
                diff: None,
            },
        );

        obs.impact.severity = if errors.len() > 10 {
            Severity::High
        } else if !errors.is_empty() {
            Severity::Medium
        } else {
            Severity::Low
        };
        obs.impact.affected_files = affected_files;
        obs.base.confidence = 1.0;

        obs
    }
}

/// Type error information.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypeErrorInfo {
    file: String,
    line: u32,
    message: String,
}

trait CodeObservationExt {
    fn with_change(self, change: Change) -> Self;
}

impl CodeObservationExt for CodeObservation {
    fn with_change(mut self, change: Change) -> Self {
        self.change = change;
        self
    }
}

#[async_trait]
impl Watcher for CodeWatch {
    fn id(&self) -> &str {
        &self.state.id
    }

    fn watcher_type(&self) -> WatcherType {
        WatcherType::Code
    }

    fn is_running(&self) -> bool {
        self.state.running
    }

    async fn start(&mut self) -> anyhow::Result<()> {
        if self.state.running {
            warn!(watcher_id = %self.state.id, "CodeWatch already running");
            return Ok(());
        }

        // Initialize Git
        self.init_git().await?;

        self.state.running = true;

        info!(
            watcher_id = %self.state.id,
            git_root = ?self.git_root,
            typecheck_enabled = %self.config.enable_typecheck,
            "CodeWatch started"
        );

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.state.running = false;
        info!(watcher_id = %self.state.id, "CodeWatch stopped");
        Ok(())
    }

    async fn observe(&mut self) -> Option<Observation> {
        if !self.state.running {
            return None;
        }

        let start = Instant::now();

        // Check for Git changes
        let current_hash = match self.get_current_commit_hash().await {
            Ok(h) => h,
            Err(e) => {
                debug!(
                    watcher_id = %self.state.id,
                    error = %e,
                    "Failed to get current commit hash"
                );
                self.state.record_error();
                return None;
            }
        };

        // If we have a previous hash and it's different, observe the change
        if let Some(ref last_hash) = self.last_commit_hash {
            if &current_hash != last_hash {
                match self.get_git_changes(last_hash, &current_hash).await {
                    Ok(changes) if !changes.is_empty() => {
                        let obs = self.create_git_change_observation(&changes);
                        self.last_commit_hash = Some(current_hash);
                        self.state
                            .record_observation(start.elapsed().as_millis() as u64);
                        return Some(Observation::Code(obs));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        debug!(
                            watcher_id = %self.state.id,
                            error = %e,
                            "Failed to get git changes"
                        );
                        self.state.record_error();
                    }
                }
            }
        }

        // Update last hash
        self.last_commit_hash = Some(current_hash);

        // Check if we should run typecheck
        if self.config.enable_typecheck {
            let should_typecheck = match self.last_typecheck {
                Some(last) => last.elapsed().as_millis() as u64 >= self.config.typecheck_interval_ms,
                None => true,
            };

            if should_typecheck {
                return self.run_typecheck().await;
            }
        }

        None
    }

    fn get_status(&self) -> WatcherStatus {
        WatcherStatus {
            id: self.state.id.clone(),
            watcher_type: WatcherType::Code,
            running: self.state.running,
            health: self.state.calculate_health(),
            last_observation: self.state.last_observation,
            observation_count: self.state.observation_count,
            error_count: self.state.error_count,
            avg_latency_ms: self.state.avg_latency(),
        }
    }

    fn get_metrics(&self) -> WatcherMetrics {
        self.state.get_metrics()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_watch_creation() {
        let config = CodeWatchConfig::default();
        let watch = CodeWatch::new(config);

        assert!(!watch.is_running());
        assert!(watch.state.id.starts_with("code_"));
    }

    #[test]
    fn test_code_watch_config_defaults() {
        let config = CodeWatchConfig::default();

        assert!(config.track_build);
        assert!(!config.track_tests);
        assert!(!config.enable_typecheck);
        assert_eq!(config.typecheck_interval_ms, 60000);
        assert_eq!(config.options.interval_ms, 30000);
    }

    #[test]
    fn test_parse_type_errors() {
        let config = CodeWatchConfig::default();
        let watch = CodeWatch::new(config);

        let output = r#"
            src/main.ts(10,5): error TS2304: Cannot find name 'foo'.
            src/utils.ts:25:3 - error TS2339: Property 'bar' does not exist.
        "#;

        let errors = watch.parse_type_errors(output);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].file, "src/main.ts");
        assert_eq!(errors[0].line, 10);
        assert_eq!(errors[1].file, "src/utils.ts");
        assert_eq!(errors[1].line, 25);
    }

    #[test]
    fn test_determine_scope() {
        let config = CodeWatchConfig::default();
        let watch = CodeWatch::new(config);

        assert_eq!(
            watch.determine_scope(&["package.json".to_string()]),
            "project"
        );
        assert_eq!(
            watch.determine_scope(&["src/utils/helper.ts".to_string()]),
            "module"
        );
        assert_eq!(
            watch.determine_scope(&["src/main.ts".to_string()]),
            "package"
        );
        assert_eq!(watch.determine_scope(&["README.md".to_string()]), "file");
    }

    #[test]
    fn test_config_serialization() {
        let config = CodeWatchConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("trackBuild"));
        assert!(json.contains("enableTypecheck"));
    }
}
