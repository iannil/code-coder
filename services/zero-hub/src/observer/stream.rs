//! Event stream for the Observer Network.
//!
//! High-performance event buffering, routing, and aggregation for observation events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

/// Observer event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Code-related observation
    Code,
    /// World/external observation
    World,
    /// Self/agent observation
    AgentSelf,
    /// Meta observation (observing observers)
    Meta,
    /// System event (gear change, dial adjustment, etc.)
    System,
}

/// Priority levels for events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// An observation event in the stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: EventType,
    /// Event priority
    pub priority: Priority,
    /// Source watcher ID
    pub source: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event payload (flexible JSON data)
    pub payload: serde_json::Value,
    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    1.0
}

impl ObserverEvent {
    /// Create a new observer event.
    pub fn new(
        event_type: EventType,
        source: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: generate_event_id(),
            event_type,
            priority: Priority::Normal,
            source: source.into(),
            timestamp: Utc::now(),
            payload,
            tags: Vec::new(),
            confidence: 1.0,
        }
    }

    /// Create a system event.
    pub fn system(source: impl Into<String>, payload: serde_json::Value) -> Self {
        Self::new(EventType::System, source, payload)
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Configuration for the event stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Maximum events to buffer
    pub max_buffer_size: usize,
    /// Enable event deduplication
    pub deduplicate: bool,
    /// Deduplication window in milliseconds
    pub dedup_window_ms: u64,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 10000,
            deduplicate: true,
            dedup_window_ms: 1000,
            batch_size: 100,
        }
    }
}

/// Statistics for the event stream.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamStats {
    /// Total events received
    pub received: u64,
    /// Total events processed
    pub processed: u64,
    /// Total events dropped (buffer full)
    pub dropped: u64,
    /// Total events deduplicated
    pub deduplicated: u64,
    /// Current buffer size
    pub buffer_size: usize,
    /// Events by type
    pub by_type: EventTypeStats,
}

/// Event counts by type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventTypeStats {
    pub code: u64,
    pub world: u64,
    pub agent_self: u64,
    pub meta: u64,
    pub system: u64,
}

/// The event stream buffer and processor.
pub struct ObserverStream {
    config: StreamConfig,
    buffer: VecDeque<ObserverEvent>,
    received: AtomicU64,
    processed: AtomicU64,
    dropped: AtomicU64,
    deduplicated: AtomicU64,
    type_counts: TypeCounts,
}

struct TypeCounts {
    code: AtomicU64,
    world: AtomicU64,
    agent_self: AtomicU64,
    meta: AtomicU64,
    system: AtomicU64,
}

impl Default for TypeCounts {
    fn default() -> Self {
        Self {
            code: AtomicU64::new(0),
            world: AtomicU64::new(0),
            agent_self: AtomicU64::new(0),
            meta: AtomicU64::new(0),
            system: AtomicU64::new(0),
        }
    }
}

impl ObserverStream {
    /// Create a new event stream.
    pub fn new(config: StreamConfig) -> Self {
        Self {
            config,
            buffer: VecDeque::new(),
            received: AtomicU64::new(0),
            processed: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            deduplicated: AtomicU64::new(0),
            type_counts: TypeCounts::default(),
        }
    }

    /// Push an event to the stream.
    pub fn push(&mut self, event: ObserverEvent) -> bool {
        self.received.fetch_add(1, Ordering::Relaxed);

        // Track by type
        match event.event_type {
            EventType::Code => self.type_counts.code.fetch_add(1, Ordering::Relaxed),
            EventType::World => self.type_counts.world.fetch_add(1, Ordering::Relaxed),
            EventType::AgentSelf => self.type_counts.agent_self.fetch_add(1, Ordering::Relaxed),
            EventType::Meta => self.type_counts.meta.fetch_add(1, Ordering::Relaxed),
            EventType::System => self.type_counts.system.fetch_add(1, Ordering::Relaxed),
        };

        // Check buffer capacity
        if self.buffer.len() >= self.config.max_buffer_size {
            // Drop oldest event
            self.buffer.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }

        self.buffer.push_back(event);
        true
    }

    /// Pop an event from the stream.
    pub fn pop(&mut self) -> Option<ObserverEvent> {
        let event = self.buffer.pop_front();
        if event.is_some() {
            self.processed.fetch_add(1, Ordering::Relaxed);
        }
        event
    }

    /// Pop a batch of events.
    pub fn pop_batch(&mut self, max_size: usize) -> Vec<ObserverEvent> {
        let size = max_size.min(self.buffer.len());
        let events: Vec<_> = self.buffer.drain(..size).collect();
        self.processed.fetch_add(events.len() as u64, Ordering::Relaxed);
        events
    }

    /// Peek at the next event without removing it.
    pub fn peek(&self) -> Option<&ObserverEvent> {
        self.buffer.front()
    }

    /// Get current buffer size.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get stream statistics.
    pub fn stats(&self) -> StreamStats {
        StreamStats {
            received: self.received.load(Ordering::Relaxed),
            processed: self.processed.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            deduplicated: self.deduplicated.load(Ordering::Relaxed),
            buffer_size: self.buffer.len(),
            by_type: EventTypeStats {
                code: self.type_counts.code.load(Ordering::Relaxed),
                world: self.type_counts.world.load(Ordering::Relaxed),
                agent_self: self.type_counts.agent_self.load(Ordering::Relaxed),
                meta: self.type_counts.meta.load(Ordering::Relaxed),
                system: self.type_counts.system.load(Ordering::Relaxed),
            },
        }
    }

    /// Filter events by type.
    pub fn filter_by_type(&self, event_type: EventType) -> Vec<&ObserverEvent> {
        self.buffer
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }

    /// Filter events by priority.
    pub fn filter_by_priority(&self, min_priority: Priority) -> Vec<&ObserverEvent> {
        self.buffer
            .iter()
            .filter(|e| e.priority >= min_priority)
            .collect()
    }

    /// Get events since a timestamp.
    pub fn events_since(&self, since: DateTime<Utc>) -> Vec<&ObserverEvent> {
        self.buffer
            .iter()
            .filter(|e| e.timestamp >= since)
            .collect()
    }
}

/// Generate a unique event ID.
fn generate_event_id() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = Utc::now().timestamp_millis();
    format!("evt_{}_{}", timestamp, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_event_new() {
        let event = ObserverEvent::new(
            EventType::Code,
            "test-watcher",
            serde_json::json!({"action": "commit"}),
        );

        assert_eq!(event.event_type, EventType::Code);
        assert_eq!(event.source, "test-watcher");
        assert_eq!(event.priority, Priority::Normal);
        assert_eq!(event.confidence, 1.0);
    }

    #[test]
    fn test_observer_event_with_priority() {
        let event = ObserverEvent::new(EventType::System, "test", serde_json::json!({}))
            .with_priority(Priority::Critical);

        assert_eq!(event.priority, Priority::Critical);
    }

    #[test]
    fn test_stream_push_pop() {
        let mut stream = ObserverStream::new(StreamConfig::default());

        let event = ObserverEvent::new(EventType::Code, "test", serde_json::json!({}));
        stream.push(event);

        assert_eq!(stream.len(), 1);

        let popped = stream.pop();
        assert!(popped.is_some());
        assert_eq!(stream.len(), 0);
    }

    #[test]
    fn test_stream_buffer_overflow() {
        let config = StreamConfig {
            max_buffer_size: 3,
            ..Default::default()
        };
        let mut stream = ObserverStream::new(config);

        for i in 0..5 {
            let event = ObserverEvent::new(
                EventType::Code,
                format!("source-{}", i),
                serde_json::json!({"index": i}),
            );
            stream.push(event);
        }

        // Buffer should be at max size
        assert_eq!(stream.len(), 3);

        // Oldest events should be dropped
        let stats = stream.stats();
        assert_eq!(stats.dropped, 2);
    }

    #[test]
    fn test_stream_pop_batch() {
        let mut stream = ObserverStream::new(StreamConfig::default());

        for i in 0..10 {
            let event = ObserverEvent::new(EventType::Code, "test", serde_json::json!({"i": i}));
            stream.push(event);
        }

        let batch = stream.pop_batch(5);
        assert_eq!(batch.len(), 5);
        assert_eq!(stream.len(), 5);
    }

    #[test]
    fn test_stream_stats() {
        let mut stream = ObserverStream::new(StreamConfig::default());

        stream.push(ObserverEvent::new(EventType::Code, "a", serde_json::json!({})));
        stream.push(ObserverEvent::new(EventType::World, "b", serde_json::json!({})));
        stream.push(ObserverEvent::new(EventType::Code, "c", serde_json::json!({})));

        let stats = stream.stats();
        assert_eq!(stats.received, 3);
        assert_eq!(stats.by_type.code, 2);
        assert_eq!(stats.by_type.world, 1);
    }

    #[test]
    fn test_filter_by_type() {
        let mut stream = ObserverStream::new(StreamConfig::default());

        stream.push(ObserverEvent::new(EventType::Code, "a", serde_json::json!({})));
        stream.push(ObserverEvent::new(EventType::World, "b", serde_json::json!({})));
        stream.push(ObserverEvent::new(EventType::Code, "c", serde_json::json!({})));

        let code_events = stream.filter_by_type(EventType::Code);
        assert_eq!(code_events.len(), 2);
    }

    #[test]
    fn test_filter_by_priority() {
        let mut stream = ObserverStream::new(StreamConfig::default());

        stream.push(
            ObserverEvent::new(EventType::Code, "a", serde_json::json!({}))
                .with_priority(Priority::Low),
        );
        stream.push(
            ObserverEvent::new(EventType::Code, "b", serde_json::json!({}))
                .with_priority(Priority::High),
        );
        stream.push(
            ObserverEvent::new(EventType::Code, "c", serde_json::json!({}))
                .with_priority(Priority::Critical),
        );

        let high_priority = stream.filter_by_priority(Priority::High);
        assert_eq!(high_priority.len(), 2); // High and Critical
    }
}
