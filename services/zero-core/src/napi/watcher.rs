//! NAPI bindings for file system watcher
//!
//! Provides cross-platform file change detection using the notify crate.
//! This replaces @parcel/watcher with a native Rust implementation.

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::foundation::watcher::{
    FileWatcher as RustFileWatcher, FileWatcherConfig as RustFileWatcherConfig,
    WatchEvent as RustWatchEvent, WatchEventKind as RustWatchEventKind,
};

/// Watch event kind
#[napi(string_enum)]
#[derive(Debug, PartialEq)]
pub enum WatchEventKind {
    /// File created
    Add,
    /// File modified
    Change,
    /// File deleted
    Unlink,
    /// File renamed
    Rename,
}

impl From<RustWatchEventKind> for WatchEventKind {
    fn from(kind: RustWatchEventKind) -> Self {
        match kind {
            RustWatchEventKind::Create => WatchEventKind::Add,
            RustWatchEventKind::Modify => WatchEventKind::Change,
            RustWatchEventKind::Delete => WatchEventKind::Unlink,
            RustWatchEventKind::Rename => WatchEventKind::Rename,
            // Map other types to Change as a fallback
            RustWatchEventKind::Access => WatchEventKind::Change,
            RustWatchEventKind::Other => WatchEventKind::Change,
        }
    }
}

/// File watch event
#[napi(object)]
pub struct WatchEvent {
    /// Affected file path
    pub path: String,
    /// Event type
    pub kind: String,
    /// Timestamp (Unix ms)
    pub timestamp: i64,
}

impl From<RustWatchEvent> for WatchEvent {
    fn from(event: RustWatchEvent) -> Self {
        // Take the first path (most common case)
        let path = event
            .paths
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let kind = match event.kind {
            RustWatchEventKind::Create => "add",
            RustWatchEventKind::Modify => "change",
            RustWatchEventKind::Delete => "unlink",
            RustWatchEventKind::Rename => "rename",
            RustWatchEventKind::Access => "change",
            RustWatchEventKind::Other => "change",
        };

        WatchEvent {
            path,
            kind: kind.to_string(),
            timestamp: event.timestamp as i64,
        }
    }
}

/// File watcher configuration
#[napi(object)]
pub struct FileWatcherConfig {
    /// Debounce duration in milliseconds (default: 100)
    pub debounce_ms: Option<u32>,
    /// Whether to watch recursively (default: true)
    pub recursive: Option<bool>,
    /// Patterns to ignore (gitignore style)
    pub ignore: Option<Vec<String>>,
}

impl From<FileWatcherConfig> for RustFileWatcherConfig {
    fn from(config: FileWatcherConfig) -> Self {
        let mut event_types = HashSet::new();
        event_types.insert(RustWatchEventKind::Create);
        event_types.insert(RustWatchEventKind::Modify);
        event_types.insert(RustWatchEventKind::Delete);
        event_types.insert(RustWatchEventKind::Rename);

        RustFileWatcherConfig {
            debounce_ms: config.debounce_ms.unwrap_or(100) as u64,
            recursive: config.recursive.unwrap_or(true),
            ignore_patterns: config.ignore.unwrap_or_else(|| {
                vec![
                    ".git/**".to_string(),
                    "node_modules/**".to_string(),
                    "target/**".to_string(),
                    "*.tmp".to_string(),
                    "*.swp".to_string(),
                ]
            }),
            event_types,
        }
    }
}

/// Handle to a file watcher subscription
struct WatcherSubscription {
    watcher: RustFileWatcher,
    running: Arc<Mutex<bool>>,
}

/// Handle to a file watcher
#[napi]
pub struct FileWatcherHandle {
    config: RustFileWatcherConfig,
    subscriptions: Arc<Mutex<Vec<WatcherSubscription>>>,
}

/// Create a new file watcher with default configuration
#[napi]
pub fn create_file_watcher() -> FileWatcherHandle {
    FileWatcherHandle {
        config: RustFileWatcherConfig::default(),
        subscriptions: Arc::new(Mutex::new(Vec::new())),
    }
}

/// Create a new file watcher with custom configuration
#[napi]
pub fn create_file_watcher_with_config(config: FileWatcherConfig) -> FileWatcherHandle {
    FileWatcherHandle {
        config: config.into(),
        subscriptions: Arc::new(Mutex::new(Vec::new())),
    }
}

#[napi]
impl FileWatcherHandle {
    /// Subscribe to file changes in a directory
    /// The callback receives (path: string, event: 'add' | 'change' | 'unlink')
    #[napi]
    pub fn subscribe(
        &self,
        path: String,
        #[napi(ts_arg_type = "(path: string, event: string) => void")] callback: ThreadsafeFunction<
            (String, String),
        >,
    ) -> Result<()> {
        let watcher = RustFileWatcher::with_config(self.config.clone());
        let mut rx = watcher
            .watch(&path)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();

        // Spawn a task to forward events to JS callback
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let is_running = *running_clone.lock().unwrap();
                if !is_running {
                    break;
                }

                // Convert event kind to string
                let kind = match event.kind {
                    RustWatchEventKind::Create => "add",
                    RustWatchEventKind::Modify => "change",
                    RustWatchEventKind::Delete => "unlink",
                    RustWatchEventKind::Rename => "rename",
                    RustWatchEventKind::Access => "change",
                    RustWatchEventKind::Other => "change",
                };

                // Send each path as a separate event
                for p in event.paths {
                    let path_str = p.to_string_lossy().to_string();
                    let _ =
                        callback.call(Ok((path_str, kind.to_string())), ThreadsafeFunctionCallMode::NonBlocking);
                }
            }
        });

        // Store the subscription
        let mut subs = self.subscriptions.lock().unwrap();
        subs.push(WatcherSubscription { watcher, running });

        Ok(())
    }

    /// Unsubscribe from all watchers
    #[napi]
    pub fn unsubscribe_all(&self) -> Result<()> {
        let mut subs = self.subscriptions.lock().unwrap();

        for sub in subs.iter() {
            // Signal the task to stop
            *sub.running.lock().unwrap() = false;
            // Unwatch all paths
            let _ = sub.watcher.unwatch_all();
        }

        subs.clear();
        Ok(())
    }

    /// Get the number of active subscriptions
    #[napi]
    pub fn subscription_count(&self) -> u32 {
        let subs = self.subscriptions.lock().unwrap();
        subs.len() as u32
    }

    /// Check if currently watching any paths
    #[napi]
    pub fn is_watching(&self) -> bool {
        let subs = self.subscriptions.lock().unwrap();
        !subs.is_empty()
    }
}

impl Drop for FileWatcherHandle {
    fn drop(&mut self) {
        let _ = self.unsubscribe_all();
    }
}

/// Convenience function to watch a single path with a callback
#[napi]
pub fn watch_path(
    path: String,
    #[napi(ts_arg_type = "(path: string, event: string) => void")] callback: ThreadsafeFunction<
        (String, String),
    >,
    config: Option<FileWatcherConfig>,
) -> Result<FileWatcherHandle> {
    let handle = if let Some(cfg) = config {
        create_file_watcher_with_config(cfg)
    } else {
        create_file_watcher()
    };

    handle.subscribe(path, callback)?;
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_event_kind_conversion() {
        assert_eq!(
            WatchEventKind::from(RustWatchEventKind::Create),
            WatchEventKind::Add
        );
        assert_eq!(
            WatchEventKind::from(RustWatchEventKind::Modify),
            WatchEventKind::Change
        );
        assert_eq!(
            WatchEventKind::from(RustWatchEventKind::Delete),
            WatchEventKind::Unlink
        );
    }

    #[test]
    fn test_config_defaults() {
        let config = FileWatcherConfig {
            debounce_ms: None,
            recursive: None,
            ignore: None,
        };

        let rust_config: RustFileWatcherConfig = config.into();
        assert_eq!(rust_config.debounce_ms, 100);
        assert!(rust_config.recursive);
        assert!(!rust_config.ignore_patterns.is_empty());
    }
}
