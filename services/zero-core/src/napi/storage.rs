//! NAPI bindings for storage module
//!
//! Exposes KVStore functionality to Node.js/TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

use crate::storage::{KVStore as RustKVStore, StoreStats as RustStoreStats};

/// Entry metadata
#[napi(object)]
pub struct NapiEntryMeta {
    pub key: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub size: i64,
}

/// Store statistics
#[napi(object)]
pub struct NapiStoreStats {
    pub total_entries: u32,
    pub total_size_bytes: u32,
    pub oldest_entry_ms: Option<i64>,
    pub newest_entry_ms: Option<i64>,
}

impl From<RustStoreStats> for NapiStoreStats {
    fn from(s: RustStoreStats) -> Self {
        Self {
            total_entries: s.total_entries as u32,
            total_size_bytes: s.total_size_bytes as u32,
            oldest_entry_ms: s.oldest_entry_ms,
            newest_entry_ms: s.newest_entry_ms,
        }
    }
}

/// Handle to a KVStore
#[napi]
pub struct KVStoreHandle {
    inner: Arc<RustKVStore>,
}

/// Open or create a KV store at the given path
#[napi]
pub async fn open_kv_store(path: String) -> Result<KVStoreHandle> {
    let store = RustKVStore::open(&path)
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(KVStoreHandle {
        inner: Arc::new(store),
    })
}

/// Create an in-memory KV store (for testing)
#[napi]
pub fn create_memory_kv_store() -> Result<KVStoreHandle> {
    let store = RustKVStore::in_memory().map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(KVStoreHandle {
        inner: Arc::new(store),
    })
}

#[napi]
impl KVStoreHandle {
    /// Set a value for a key path
    #[napi]
    pub async fn set(&self, key: Vec<String>, value: String) -> Result<()> {
        let key_refs: Vec<&str> = key.iter().map(|s| s.as_str()).collect();
        self.inner
            .set(&key_refs, &value)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get a value by key path
    #[napi]
    pub async fn get(&self, key: Vec<String>) -> Result<Option<String>> {
        let key_refs: Vec<&str> = key.iter().map(|s| s.as_str()).collect();
        self.inner
            .get(&key_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete a key
    #[napi]
    pub async fn delete(&self, key: Vec<String>) -> Result<bool> {
        let key_refs: Vec<&str> = key.iter().map(|s| s.as_str()).collect();
        self.inner
            .delete(&key_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a key exists
    #[napi]
    pub async fn exists(&self, key: Vec<String>) -> Result<bool> {
        let key_refs: Vec<&str> = key.iter().map(|s| s.as_str()).collect();
        self.inner
            .exists(&key_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List keys with a prefix
    #[napi]
    pub async fn list(&self, prefix: Vec<String>) -> Result<Vec<String>> {
        let prefix_refs: Vec<&str> = prefix.iter().map(|s| s.as_str()).collect();
        self.inner
            .list(&prefix_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Count entries with a prefix
    #[napi]
    pub async fn count(&self, prefix: Vec<String>) -> Result<u32> {
        let prefix_refs: Vec<&str> = prefix.iter().map(|s| s.as_str()).collect();
        self.inner
            .count(&prefix_refs)
            .await
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete all keys with a prefix
    #[napi]
    pub async fn delete_prefix(&self, prefix: Vec<String>) -> Result<u32> {
        let prefix_refs: Vec<&str> = prefix.iter().map(|s| s.as_str()).collect();
        self.inner
            .delete_prefix(&prefix_refs)
            .await
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get store statistics
    #[napi]
    pub async fn stats(&self) -> Result<NapiStoreStats> {
        self.inner
            .stats()
            .await
            .map(Into::into)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Health check
    #[napi]
    pub async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    /// Compact the database
    #[napi]
    pub async fn compact(&self) -> Result<()> {
        self.inner
            .compact()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get the database path
    #[napi]
    pub fn path(&self) -> String {
        self.inner.path().to_string_lossy().to_string()
    }
}
