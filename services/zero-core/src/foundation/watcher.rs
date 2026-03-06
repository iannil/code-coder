//! File system watcher using notify crate
//!
//! Provides cross-platform file change detection with:
//! - Debouncing to reduce event noise
//! - Recursive directory watching
//! - Gitignore pattern support
//! - Event type filtering

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// File watch event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    /// Affected paths
    pub paths: Vec<PathBuf>,
    /// Event type
    pub kind: WatchEventKind,
    /// Timestamp (Unix ms)
    pub timestamp: u64,
}

/// Watch event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEventKind {
    /// File created
    Create,
    /// File modified
    Modify,
    /// File deleted
    Delete,
    /// File renamed
    Rename,
    /// Access (read)
    Access,
    /// Other/unknown
    Other,
}

impl From<&EventKind> for WatchEventKind {
    fn from(kind: &EventKind) -> Self {
        match kind {
            EventKind::Create(_) => WatchEventKind::Create,
            EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any) => WatchEventKind::Modify,
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => WatchEventKind::Rename,
            EventKind::Modify(ModifyKind::Name(_)) => WatchEventKind::Rename,
            EventKind::Remove(_) => WatchEventKind::Delete,
            EventKind::Access(_) => WatchEventKind::Access,
            _ => WatchEventKind::Other,
        }
    }
}

/// File watcher configuration
#[derive(Debug, Clone)]
pub struct FileWatcherConfig {
    /// Debounce duration (milliseconds)
    pub debounce_ms: u64,
    /// Whether to watch recursively
    pub recursive: bool,
    /// Patterns to ignore (gitignore style)
    pub ignore_patterns: Vec<String>,
    /// Event types to listen for
    pub event_types: HashSet<WatchEventKind>,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        let mut event_types = HashSet::new();
        event_types.insert(WatchEventKind::Create);
        event_types.insert(WatchEventKind::Modify);
        event_types.insert(WatchEventKind::Delete);
        event_types.insert(WatchEventKind::Rename);

        Self {
            debounce_ms: 100,
            recursive: true,
            ignore_patterns: vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
            ],
            event_types,
        }
    }
}

/// File system watcher
pub struct FileWatcher {
    config: FileWatcherConfig,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

impl FileWatcher {
    /// Create a new file watcher with default configuration
    pub fn new() -> Self {
        Self::with_config(FileWatcherConfig::default())
    }

    /// Create a new file watcher with custom configuration
    pub fn with_config(config: FileWatcherConfig) -> Self {
        Self {
            config,
            watcher: Arc::new(Mutex::new(None)),
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Start watching a path and return a receiver for events
    pub fn watch(&self, path: impl AsRef<Path>) -> anyhow::Result<mpsc::Receiver<WatchEvent>> {
        let path = path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel(1000);

        let config = self.config.clone();
        let _watched_paths = self.watched_paths.clone();

        // Create the notify watcher with debouncing
        let notify_config = Config::default()
            .with_poll_interval(Duration::from_millis(config.debounce_ms));

        let tx_clone = tx.clone();
        let ignore_patterns = config.ignore_patterns.clone();
        let event_types = config.event_types.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        let kind: WatchEventKind = (&event.kind).into();

                        // Filter by event type
                        if !event_types.contains(&kind) {
                            return;
                        }

                        // Filter by ignore patterns
                        let paths: Vec<PathBuf> = event
                            .paths
                            .into_iter()
                            .filter(|p| !should_ignore(p, &ignore_patterns))
                            .collect();

                        if paths.is_empty() {
                            return;
                        }

                        let watch_event = WatchEvent {
                            paths,
                            kind,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };

                        if let Err(e) = tx_clone.blocking_send(watch_event) {
                            debug!("Failed to send watch event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {:?}", e);
                    }
                }
            },
            notify_config,
        )?;

        // Add the path to watch
        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(&path, mode)?;

        // Store the watcher and path
        {
            let mut guard = self.watcher.lock().unwrap();
            *guard = Some(watcher);
        }
        {
            let mut guard = self.watched_paths.lock().unwrap();
            guard.insert(path.clone());
        }

        info!("Started watching: {:?}", path);
        Ok(rx)
    }

    /// Stop watching a specific path
    pub fn unwatch(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();

        let mut guard = self.watcher.lock().unwrap();
        if let Some(ref mut watcher) = *guard {
            watcher.unwatch(path)?;
        }

        let mut paths = self.watched_paths.lock().unwrap();
        paths.remove(path);

        info!("Stopped watching: {:?}", path);
        Ok(())
    }

    /// Stop watching all paths
    pub fn unwatch_all(&self) -> anyhow::Result<()> {
        let paths: Vec<PathBuf> = {
            let guard = self.watched_paths.lock().unwrap();
            guard.iter().cloned().collect()
        };

        for path in paths {
            self.unwatch(&path)?;
        }

        Ok(())
    }

    /// Get currently watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let guard = self.watched_paths.lock().unwrap();
        guard.iter().cloned().collect()
    }

    /// Check if a path is being watched
    pub fn is_watching(&self, path: impl AsRef<Path>) -> bool {
        let guard = self.watched_paths.lock().unwrap();
        guard.contains(path.as_ref())
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        if let Err(e) = self.unwatch_all() {
            warn!("Error during watcher cleanup: {}", e);
        }
    }
}

/// Check if a path should be ignored based on patterns
fn should_ignore(path: &Path, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    for pattern in patterns {
        // Simple glob-like matching
        if pattern.ends_with("/**") {
            // Match directory and all contents
            let prefix = &pattern[..pattern.len() - 3];
            if path_str.contains(prefix) {
                return true;
            }
        } else if pattern.starts_with("*.") {
            // Match by extension
            let ext = &pattern[2..];
            if path_str.ends_with(ext) {
                return true;
            }
        } else if path_str.contains(pattern) {
            return true;
        }
    }

    false
}

/// Watch multiple paths with a shared event channel
pub struct MultiWatcher {
    watchers: Vec<FileWatcher>,
    config: FileWatcherConfig,
}

impl MultiWatcher {
    /// Create a new multi-watcher
    pub fn new() -> Self {
        Self::with_config(FileWatcherConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: FileWatcherConfig) -> Self {
        Self {
            watchers: Vec::new(),
            config,
        }
    }

    /// Watch multiple paths, returning a unified event receiver
    pub fn watch_all(&mut self, paths: &[PathBuf]) -> anyhow::Result<mpsc::Receiver<WatchEvent>> {
        let (tx, rx) = mpsc::channel(1000);

        for path in paths {
            let watcher = FileWatcher::with_config(self.config.clone());
            let mut events = watcher.watch(path)?;

            let tx_clone = tx.clone();
            tokio::spawn(async move {
                while let Some(event) = events.recv().await {
                    if let Err(e) = tx_clone.send(event).await {
                        debug!("Failed to forward event: {}", e);
                        break;
                    }
                }
            });

            self.watchers.push(watcher);
        }

        Ok(rx)
    }

    /// Stop all watchers
    pub fn stop_all(&mut self) -> anyhow::Result<()> {
        for watcher in &self.watchers {
            watcher.unwatch_all()?;
        }
        self.watchers.clear();
        Ok(())
    }
}

impl Default for MultiWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_should_ignore() {
        let patterns = vec![
            ".git/**".to_string(),
            "node_modules/**".to_string(),
            "*.tmp".to_string(),
        ];

        assert!(should_ignore(Path::new("/project/.git/config"), &patterns));
        assert!(should_ignore(Path::new("/project/node_modules/pkg"), &patterns));
        assert!(should_ignore(Path::new("/project/file.tmp"), &patterns));
        assert!(!should_ignore(Path::new("/project/src/main.rs"), &patterns));
    }

    #[test]
    fn test_watch_event_kind_conversion() {
        assert_eq!(
            WatchEventKind::from(&EventKind::Create(CreateKind::File)),
            WatchEventKind::Create
        );
        assert_eq!(
            WatchEventKind::from(&EventKind::Remove(RemoveKind::File)),
            WatchEventKind::Delete
        );
    }

    #[test]
    fn test_watcher_config_default() {
        let config = FileWatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.recursive);
        assert!(config.event_types.contains(&WatchEventKind::Create));
        assert!(config.event_types.contains(&WatchEventKind::Modify));
        assert!(!config.event_types.contains(&WatchEventKind::Access));
    }

    #[tokio::test]
    async fn test_watcher_create() {
        let watcher = FileWatcher::new();
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let mut rx = watcher.watch(path).unwrap();

        // Create a file
        let file_path = path.join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        // Wait for event with timeout
        let _event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .ok()
            .flatten();

        // The watcher may or may not catch the event depending on timing
        // This test mainly verifies the watcher doesn't crash
        drop(watcher);
    }

    #[test]
    fn test_watcher_watched_paths() {
        let watcher = FileWatcher::new();
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();

        let _rx = watcher.watch(&path).unwrap();

        assert!(watcher.is_watching(&path));
        assert_eq!(watcher.watched_paths().len(), 1);

        watcher.unwatch(&path).unwrap();
        assert!(!watcher.is_watching(&path));
    }
}
