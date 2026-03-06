//! Storage module - unified storage layer
//!
//! Provides ACID-compliant key-value storage using SQLite.
//!
//! # Features
//!
//! - SQLite-backed KV store with WAL mode
//! - Path-based keys (e.g., ["session", "abc123"])
//! - Automatic schema migrations
//! - Backup and restore functionality
//! - Health checks and statistics

mod kv;

pub use kv::{EntryMeta, KVStore, StoreStats};
