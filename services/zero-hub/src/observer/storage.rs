//! Storage for observer events.
//!
//! Provides persistence for observation events and history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

use super::stream::ObserverEvent;

/// A stored event with additional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// The original event
    pub event: ObserverEvent,
    /// Storage timestamp
    pub stored_at: DateTime<Utc>,
    /// Whether the event has been processed
    pub processed: bool,
    /// Processing result (if any)
    pub result: Option<String>,
}

impl StoredEvent {
    /// Create a new stored event.
    pub fn new(event: ObserverEvent) -> Self {
        Self {
            event,
            stored_at: Utc::now(),
            processed: false,
            result: None,
        }
    }

    /// Mark as processed with a result.
    pub fn mark_processed(&mut self, result: Option<String>) {
        self.processed = true;
        self.result = result;
    }
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum events to store in memory
    pub max_memory_events: usize,
    /// Enable disk persistence
    pub persist_to_disk: bool,
    /// Retention period in hours
    pub retention_hours: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_memory_events: 10000,
            persist_to_disk: false,
            retention_hours: 24,
        }
    }
}

/// In-memory event storage.
pub struct ObserverStorage {
    config: StorageConfig,
    events: RwLock<VecDeque<StoredEvent>>,
}

impl ObserverStorage {
    /// Create a new storage with default config.
    pub fn new() -> Self {
        Self::with_config(StorageConfig::default())
    }

    /// Create a storage with custom config.
    pub fn with_config(config: StorageConfig) -> Self {
        Self {
            config,
            events: RwLock::new(VecDeque::new()),
        }
    }

    /// Store an event.
    pub fn store(&self, event: ObserverEvent) -> bool {
        let mut events = self.events.write().unwrap();

        // Check capacity
        if events.len() >= self.config.max_memory_events {
            events.pop_front();
        }

        events.push_back(StoredEvent::new(event));
        true
    }

    /// Get recent events.
    pub fn recent(&self, limit: usize) -> Vec<StoredEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by source.
    pub fn by_source(&self, source: &str) -> Vec<StoredEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.event.source == source)
            .cloned()
            .collect()
    }

    /// Get events in a time range.
    pub fn in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<StoredEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.event.timestamp >= start && e.event.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Get unprocessed events.
    pub fn unprocessed(&self) -> Vec<StoredEvent> {
        let events = self.events.read().unwrap();
        events.iter().filter(|e| !e.processed).cloned().collect()
    }

    /// Mark an event as processed by ID.
    pub fn mark_processed(&self, event_id: &str, result: Option<String>) -> bool {
        let mut events = self.events.write().unwrap();

        if let Some(event) = events.iter_mut().find(|e| e.event.id == event_id) {
            event.mark_processed(result);
            true
        } else {
            false
        }
    }

    /// Get total event count.
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Clear all events.
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }

    /// Prune old events based on retention policy.
    pub fn prune(&self) -> usize {
        let cutoff = Utc::now() - chrono::Duration::hours(self.config.retention_hours as i64);
        let mut events = self.events.write().unwrap();

        let before = events.len();
        events.retain(|e| e.event.timestamp >= cutoff);
        before - events.len()
    }

    /// Get storage statistics.
    pub fn stats(&self) -> StorageStats {
        let events = self.events.read().unwrap();
        let processed = events.iter().filter(|e| e.processed).count();

        StorageStats {
            total_events: events.len(),
            processed_events: processed,
            unprocessed_events: events.len() - processed,
            capacity: self.config.max_memory_events,
            utilization: (events.len() as f64 / self.config.max_memory_events as f64 * 100.0) as u8,
        }
    }
}

impl Default for ObserverStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total stored events
    pub total_events: usize,
    /// Processed events
    pub processed_events: usize,
    /// Unprocessed events
    pub unprocessed_events: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Utilization percentage
    pub utilization: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::stream::EventType;

    fn test_event(source: &str) -> ObserverEvent {
        ObserverEvent::new(EventType::Code, source, serde_json::json!({"test": true}))
    }

    #[test]
    fn test_storage_store_and_retrieve() {
        let storage = ObserverStorage::new();

        storage.store(test_event("source-1"));
        storage.store(test_event("source-2"));

        assert_eq!(storage.count(), 2);

        let recent = storage.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].event.source, "source-2"); // Most recent first
    }

    #[test]
    fn test_storage_capacity() {
        let config = StorageConfig {
            max_memory_events: 3,
            ..Default::default()
        };
        let storage = ObserverStorage::with_config(config);

        for i in 0..5 {
            storage.store(test_event(&format!("source-{}", i)));
        }

        assert_eq!(storage.count(), 3);

        let recent = storage.recent(10);
        // Should have sources 2, 3, 4 (0 and 1 were evicted)
        assert_eq!(recent[0].event.source, "source-4");
    }

    #[test]
    fn test_storage_by_source() {
        let storage = ObserverStorage::new();

        storage.store(test_event("a"));
        storage.store(test_event("b"));
        storage.store(test_event("a"));

        let from_a = storage.by_source("a");
        assert_eq!(from_a.len(), 2);
    }

    #[test]
    fn test_storage_mark_processed() {
        let storage = ObserverStorage::new();

        storage.store(test_event("test"));

        let recent = storage.recent(1);
        let event_id = &recent[0].event.id;

        assert!(!recent[0].processed);

        storage.mark_processed(event_id, Some("done".into()));

        let recent = storage.recent(1);
        assert!(recent[0].processed);
        assert_eq!(recent[0].result.as_deref(), Some("done"));
    }

    #[test]
    fn test_storage_unprocessed() {
        let storage = ObserverStorage::new();

        storage.store(test_event("a"));
        storage.store(test_event("b"));

        let recent = storage.recent(1);
        let event_id = recent[0].event.id.clone();
        storage.mark_processed(&event_id, None);

        let unprocessed = storage.unprocessed();
        assert_eq!(unprocessed.len(), 1);
    }

    #[test]
    fn test_storage_stats() {
        let storage = ObserverStorage::new();

        storage.store(test_event("a"));
        storage.store(test_event("b"));

        let recent = storage.recent(1);
        storage.mark_processed(&recent[0].event.id, None);

        let stats = storage.stats();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.processed_events, 1);
        assert_eq!(stats.unprocessed_events, 1);
    }

    #[test]
    fn test_storage_clear() {
        let storage = ObserverStorage::new();

        storage.store(test_event("a"));
        storage.store(test_event("b"));
        assert_eq!(storage.count(), 2);

        storage.clear();
        assert_eq!(storage.count(), 0);
    }
}
