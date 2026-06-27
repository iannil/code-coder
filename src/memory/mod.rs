/// ─── Memory ────────────────────────────────────────────────────────────────
///
/// File-based key-value storage.  Each memory is a `.md` file under
/// `memory/`.  The system uses this to persist what it learns (API
/// schemas, user preferences, past discoveries, etc.).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// An individual memory entry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub path: PathBuf,
}

/// Manages the `memory/` directory as a simple key-value store.
pub struct MemoryStore {
    base_path: PathBuf,
    /// In-memory cache.
    entries: HashMap<String, MemoryEntry>,
}

impl MemoryStore {
    /// Open (or create) the memory store at the given project root.
    pub fn open(project_root: &str) -> Self {
        let base_path = Path::new(project_root).join("memory");
        let mut store = Self {
            base_path,
            entries: HashMap::new(),
        };
        let _ = store.load_all();
        store
    }

    /// Load all `.md` files from the memory directory into cache.
    pub fn load_all(&mut self) -> anyhow::Result<()> {
        self.entries.clear();

        if !self.base_path.exists() {
            std::fs::create_dir_all(&self.base_path)?;
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.base_path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let key = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let value = std::fs::read_to_string(&path)?;
                self.entries.insert(
                    key.clone(),
                    MemoryEntry {
                        key,
                        value,
                        path,
                    },
                );
            }
        }

        Ok(())
    }

    /// Get a memory entry by key.
    pub fn get(&self, key: &str) -> Option<&MemoryEntry> {
        self.entries.get(key)
    }

    /// List all memory keys.
    pub fn list(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        keys.sort();
        keys
    }

    /// Set a memory entry (writes to disk immediately).
    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let file_path = self.base_path.join(format!("{key}.md"));

        std::fs::write(&file_path, value)
            .map_err(|e| anyhow::anyhow!("cannot write memory {key}: {e}"))?;

        self.entries.insert(
            key.to_string(),
            MemoryEntry {
                key: key.to_string(),
                value: value.to_string(),
                path: file_path,
            },
        );

        Ok(())
    }

    /// Delete a memory entry.
    pub fn delete(&mut self, key: &str) -> anyhow::Result<()> {
        let file_path = self.base_path.join(format!("{key}.md"));
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }
        self.entries.remove(key);
        Ok(())
    }

    /// Number of memory entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = MemoryStore::open(dir.path().to_str().unwrap());
        store.set("test-key", "hello world").unwrap();
        let entry = store.get("test-key").unwrap();
        assert_eq!(entry.value, "hello world");
    }

    #[test]
    fn test_delete() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = MemoryStore::open(dir.path().to_str().unwrap());
        store.set("temp", "temporary").unwrap();
        assert_eq!(store.len(), 1);
        store.delete("temp").unwrap();
        assert!(store.get("temp").is_none());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_persistence() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut store = MemoryStore::open(dir.path().to_str().unwrap());
            store.set("persist", "still here").unwrap();
        }
        // Re-open — data should survive
        let store = MemoryStore::open(dir.path().to_str().unwrap());
        let entry = store.get("persist").unwrap();
        assert_eq!(entry.value, "still here");
    }

    #[test]
    fn test_list() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = MemoryStore::open(dir.path().to_str().unwrap());
        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();
        let keys = store.list();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"a"));
    }

    #[test]
    fn test_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::open(dir.path().to_str().unwrap());
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }
}
