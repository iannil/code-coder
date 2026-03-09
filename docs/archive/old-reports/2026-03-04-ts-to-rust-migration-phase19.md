# Phase 19: Unified Storage Layer with SQLite

**Date**: 2026-03-04
**Status**: Completed

## Summary

Implemented a unified KV storage layer using SQLite to replace JSON file-based storage in `packages/ccode/src/storage/storage.ts`. The new implementation provides ACID guarantees, concurrent access safety, and better reliability.

## Implementation

### Rust Module

Created `services/zero-core/src/storage/kv.rs`:

```rust
pub struct KVStore {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

// Key operations
impl KVStore {
    pub async fn set(&self, key: &[&str], value: &str) -> Result<()>;
    pub async fn get(&self, key: &[&str]) -> Result<Option<String>>;
    pub async fn delete(&self, key: &[&str]) -> Result<bool>;
    pub async fn exists(&self, key: &[&str]) -> Result<bool>;
    pub async fn list(&self, prefix: &[&str]) -> Result<Vec<String>>;
    pub async fn count(&self, prefix: &[&str]) -> Result<usize>;
    pub async fn delete_prefix(&self, prefix: &[&str]) -> Result<usize>;
    pub async fn stats(&self) -> Result<StoreStats>;
    pub async fn compact(&self) -> Result<()>;
    pub async fn backup(&self, dest_path: impl AsRef<Path>) -> Result<()>;
}
```

### Key Features

1. **WAL Mode**: Uses SQLite WAL (Write-Ahead Logging) for better concurrent access
2. **Path-based Keys**: Keys are arrays like `["session", "abc123"]` joined with `/`
3. **Automatic Schema Migrations**: Version tracking with migration system
4. **Statistics**: Entry count, total size, oldest/newest timestamps
5. **Backup**: VACUUM INTO for atomic backups

### SQLite Schema

```sql
CREATE TABLE kv_store (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_kv_store_prefix ON kv_store (key COLLATE NOCASE);
CREATE INDEX idx_kv_store_updated ON kv_store (updated_at DESC);
```

### NAPI Bindings

Created `services/zero-core/src/napi/storage.rs`:

```typescript
// TypeScript interface (via NAPI)
interface KVStoreHandle {
  set(key: string[], value: string): Promise<void>;
  get(key: string[]): Promise<string | null>;
  delete(key: string[]): Promise<boolean>;
  exists(key: string[]): Promise<boolean>;
  list(prefix: string[]): Promise<string[]>;
  count(prefix: string[]): Promise<number>;
  deletePrefix(prefix: string[]): Promise<number>;
  stats(): Promise<StoreStats>;
  healthCheck(): Promise<boolean>;
  compact(): Promise<void>;
  path(): string;
}
```

## Files Created

1. `services/zero-core/src/storage/mod.rs` - Module definition
2. `services/zero-core/src/storage/kv.rs` - KVStore implementation (400+ lines)
3. `services/zero-core/src/napi/storage.rs` - NAPI bindings (170+ lines)

## Files Modified

1. `services/zero-core/src/lib.rs` - Added storage module export
2. `services/zero-core/src/napi/mod.rs` - Added storage NAPI bindings

## Test Results

```
running 12 tests
test storage::kv::tests::test_set_and_get ... ok
test storage::kv::tests::test_get_nonexistent ... ok
test storage::kv::tests::test_delete ... ok
test storage::kv::tests::test_delete_nonexistent ... ok
test storage::kv::tests::test_list ... ok
test storage::kv::tests::test_count ... ok
test storage::kv::tests::test_delete_prefix ... ok
test storage::kv::tests::test_exists ... ok
test storage::kv::tests::test_stats ... ok
test storage::kv::tests::test_health_check ... ok
test storage::kv::tests::test_update_existing ... ok
test storage::kv::tests::test_json_values ... ok

test result: ok. 12 passed; 0 failed
```

## Benefits Over JSON Files

| Aspect | JSON Files | SQLite KVStore |
|--------|------------|----------------|
| Atomicity | File rename | ACID transactions |
| Concurrent access | File locking | WAL mode |
| Corruption recovery | Manual backup/restore | Built-in journaling |
| Query performance | File scan | Indexed lookups |
| Backup | File copy | VACUUM INTO |
| Disk usage | One file per entry | Single database |

## Migration Path

The TypeScript `Storage` namespace in `packages/ccode/src/storage/storage.ts` can be updated to:

1. Import `openKvStore` from `@codecoder-ai/core`
2. Use native storage for new sessions
3. Keep JSON fallback for existing data
4. Add migration script to move JSON data to SQLite

## Architecture

```
TypeScript (packages/ccode)
    │
    └─→ @codecoder-ai/core ─→ NAPI ─→ zero-core/src/storage/kv.rs
                                              │
                                              └─→ SQLite (rusqlite)
                                                    │
                                                    ├─→ WAL mode
                                                    ├─→ Index on key prefix
                                                    └─→ Index on updated_at
```

## Next Steps

1. Add TypeScript wrapper in `@codecoder-ai/core`
2. Update `packages/ccode/src/storage/storage.ts` to use native storage
3. Create data migration script for existing JSON files
4. Add backup rotation and cleanup
