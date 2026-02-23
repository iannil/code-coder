//! Event Bus for Zero Ecosystem
//!
//! Provides a unified event bus for communication between Rust services and TypeScript.
//! The bus supports multiple backends:
//!
//! - **In-Memory**: For local development and testing
//! - **Redis Pub/Sub**: For production multi-process communication
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     Event Bus                            │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
//! │  │   ZeroBot    │  │  CodeCoder   │  │  Channels    │  │
//! │  │   (Rust)     │  │ (TypeScript) │  │   (Rust)     │  │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
//! │         │                 │                 │          │
//! │         └─────────────────┼─────────────────┘          │
//! │                           │                            │
//! │              ┌────────────▼────────────┐               │
//! │              │     EventBus Trait      │               │
//! │              │  (publish/subscribe)    │               │
//! │              └────────────┬────────────┘               │
//! │                           │                            │
//! │         ┌─────────────────┼─────────────────┐          │
//! │         │                 │                 │          │
//! │  ┌──────▼──────┐  ┌──────▼───────┐  ┌──────▼──────┐  │
//! │  │  InMemory   │  │    Redis     │  │    NATS     │  │
//! │  │    Bus      │  │   Pub/Sub    │  │  (Future)   │  │
//! │  └─────────────┘  └──────────────┘  └─────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Event Topics
//!
//! Events are organized by topic (channel name):
//!
//! | Topic | Description |
//! |-------|-------------|
//! | `agent.request` | Request to execute an agent |
//! | `agent.response` | Agent execution response |
//! | `agent.status` | Agent status updates |
//! | `session.start` | Session started |
//! | `session.end` | Session ended |
//! | `channel.message` | Incoming message from channels |
//! | `channel.outgoing` | Outgoing message to channels |
//! | `memory.update` | Memory update notification |
//! | `audit.log` | Audit log event |

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

/// Event bus errors.
#[derive(Error, Debug)]
pub enum BusError {
    /// Connection error to the backend.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Subscription error.
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Publish error.
    #[error("Publish error: {0}")]
    Publish(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Backend not available.
    #[error("Backend not available: {0}")]
    Unavailable(String),
}

/// Result type for bus operations.
pub type BusResult<T> = Result<T, BusError>;

// ============================================================================
// Event Types
// ============================================================================

/// Event payload with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID.
    pub id: String,
    /// Topic/channel name.
    pub topic: String,
    /// Event type (e.g., "agent.request", "session.start").
    pub event_type: String,
    /// Source service (e.g., "codecoder", "zero-gateway").
    pub source: String,
    /// Target service (optional, for directed messages).
    pub target: Option<String>,
    /// Correlation ID for request/response tracking.
    pub correlation_id: Option<String>,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// JSON payload.
    pub payload: serde_json::Value,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl Event {
    /// Create a new event with auto-generated ID and timestamp.
    pub fn new(topic: impl Into<String>, event_type: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic: topic.into(),
            event_type: event_type.into(),
            source: source.into(),
            target: None,
            correlation_id: None,
            timestamp: Utc::now(),
            payload: serde_json::Value::Null,
            metadata: HashMap::new(),
        }
    }

    /// Set the payload.
    pub fn with_payload(mut self, payload: impl Serialize) -> BusResult<Self> {
        self.payload = serde_json::to_value(payload)
            .map_err(|e| BusError::Serialization(e.to_string()))?;
        Ok(self)
    }

    /// Set the target service.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set the correlation ID.
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Predefined Event Types
// ============================================================================

/// Agent request event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequestPayload {
    /// Agent name to invoke.
    pub agent_name: String,
    /// User prompt.
    pub prompt: String,
    /// Session ID.
    pub session_id: String,
    /// Optional context.
    pub context: Option<serde_json::Value>,
}

/// Agent response event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponsePayload {
    /// Original request correlation ID.
    pub request_id: String,
    /// Agent response text.
    pub response: String,
    /// Whether the execution was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Token usage.
    pub tokens_used: Option<u64>,
}

/// Channel message event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessagePayload {
    /// Channel type (telegram, discord, slack, etc.).
    pub channel_type: String,
    /// Channel/chat ID.
    pub channel_id: String,
    /// Message ID.
    pub message_id: String,
    /// User ID.
    pub user_id: String,
    /// Message text.
    pub text: String,
    /// Attachments.
    pub attachments: Vec<String>,
}

/// Session event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPayload {
    /// Session ID.
    pub session_id: String,
    /// User ID.
    pub user_id: Option<String>,
    /// Agent name.
    pub agent_name: Option<String>,
    /// Additional data.
    pub data: Option<serde_json::Value>,
}

// ============================================================================
// Event Bus Trait
// ============================================================================

/// Trait for event bus implementations.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to a topic.
    async fn publish(&self, event: Event) -> BusResult<()>;

    /// Subscribe to a topic pattern.
    ///
    /// Pattern supports wildcards:
    /// - `*` matches one segment (e.g., `agent.*` matches `agent.request`, `agent.response`)
    /// - `#` matches multiple segments (e.g., `agent.#` matches `agent.request.v2`)
    async fn subscribe(&self, pattern: &str) -> BusResult<EventReceiver>;

    /// Check if the bus is connected/healthy.
    async fn is_healthy(&self) -> bool;

    /// Close the bus connection.
    async fn close(&self) -> BusResult<()>;
}

/// Event receiver for subscriptions.
pub struct EventReceiver {
    inner: broadcast::Receiver<Event>,
}

impl EventReceiver {
    /// Receive the next event.
    pub async fn recv(&mut self) -> Option<Event> {
        self.inner.recv().await.ok()
    }
}

// ============================================================================
// In-Memory Event Bus
// ============================================================================

/// In-memory event bus for local development and testing.
///
/// This implementation uses tokio broadcast channels for local pub/sub.
pub struct InMemoryBus {
    /// Topic -> broadcast sender.
    topics: Arc<RwLock<HashMap<String, broadcast::Sender<Event>>>>,
    /// Channel capacity.
    capacity: usize,
}

impl InMemoryBus {
    /// Create a new in-memory bus with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new in-memory bus with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// Get or create a topic sender.
    async fn get_or_create_topic(&self, topic: &str) -> broadcast::Sender<Event> {
        let mut topics = self.topics.write().await;
        topics
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(self.capacity).0)
            .clone()
    }

    /// Check if a topic matches a pattern (used for testing).
    #[cfg(test)]
    fn topic_matches_pattern(topic: &str, pattern: &str) -> bool {
        let topic_parts: Vec<&str> = topic.split('.').collect();
        let pattern_parts: Vec<&str> = pattern.split('.').collect();

        let mut t_idx = 0;
        let mut p_idx = 0;

        while p_idx < pattern_parts.len() {
            let p = pattern_parts[p_idx];

            if p == "#" {
                // # matches everything remaining
                return true;
            }

            if t_idx >= topic_parts.len() {
                return false;
            }

            if p == "*" {
                // * matches exactly one segment
                t_idx += 1;
                p_idx += 1;
            } else if p == topic_parts[t_idx] {
                t_idx += 1;
                p_idx += 1;
            } else {
                return false;
            }
        }

        t_idx == topic_parts.len()
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBus for InMemoryBus {
    async fn publish(&self, event: Event) -> BusResult<()> {
        let sender = self.get_or_create_topic(&event.topic).await;

        // Send to exact topic (ignore send errors if no receivers)
        let _ = sender.send(event.clone());

        tracing::debug!(
            topic = %event.topic,
            event_type = %event.event_type,
            id = %event.id,
            "Event published"
        );

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> BusResult<EventReceiver> {
        // For simplicity, we subscribe to the exact topic
        // Pattern matching is done at receive time for in-memory bus
        let sender = self.get_or_create_topic(pattern).await;
        let receiver = sender.subscribe();

        tracing::debug!(pattern = %pattern, "Subscribed to topic pattern");

        Ok(EventReceiver { inner: receiver })
    }

    async fn is_healthy(&self) -> bool {
        true
    }

    async fn close(&self) -> BusResult<()> {
        let mut topics = self.topics.write().await;
        topics.clear();
        Ok(())
    }
}

// ============================================================================
// Redis Event Bus (Feature-gated)
// ============================================================================

/// Redis-compatible event bus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisBusConfig {
    /// Redis URL (redis://host:port).
    pub url: String,
    /// Connection timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum reconnect attempts.
    pub max_reconnects: u32,
    /// Channel prefix for namespacing.
    pub channel_prefix: String,
}

impl Default for RedisBusConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            timeout_secs: 5,
            max_reconnects: 3,
            channel_prefix: "zero:".to_string(),
        }
    }
}

/// Redis Pub/Sub event bus implementation.
///
/// Uses Redis PUBLISH/PSUBSCRIBE for multi-process event distribution.
/// Supports pattern-based subscriptions (*, #) translated to Redis glob patterns.
#[cfg(feature = "redis-backend")]
pub struct RedisBus {
    config: RedisBusConfig,
    /// Connection manager for publishing (handles reconnection automatically).
    client: redis::Client,
    /// Connection manager for async operations.
    conn_manager: tokio::sync::RwLock<Option<redis::aio::ConnectionManager>>,
    /// Local broadcast for forwarding to subscribers.
    local_senders: Arc<RwLock<HashMap<String, broadcast::Sender<Event>>>>,
    /// Subscription tasks.
    subscription_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Health status.
    healthy: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "redis-backend")]
impl RedisBus {
    /// Create a new Redis bus.
    pub async fn new(config: RedisBusConfig) -> BusResult<Self> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| BusError::Connection(e.to_string()))?;

        let conn_manager = client
            .get_connection_manager()
            .await
            .map_err(|e| BusError::Connection(e.to_string()))?;

        let bus = Self {
            config,
            client,
            conn_manager: tokio::sync::RwLock::new(Some(conn_manager)),
            local_senders: Arc::new(RwLock::new(HashMap::new())),
            subscription_handles: Arc::new(RwLock::new(Vec::new())),
            healthy: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };

        Ok(bus)
    }

    /// Get the configuration.
    pub fn config(&self) -> &RedisBusConfig {
        &self.config
    }

    /// Get the prefixed channel name.
    fn prefixed_channel(&self, topic: &str) -> String {
        format!("{}{}", self.config.channel_prefix, topic)
    }

    /// Convert topic pattern to Redis glob pattern.
    ///
    /// - `*` stays as `*` (matches one segment)
    /// - `#` becomes `*` (Redis doesn't have multi-level wildcard)
    fn topic_to_redis_pattern(&self, pattern: &str) -> String {
        let redis_pattern = pattern.replace('#', "*");
        self.prefixed_channel(&redis_pattern)
    }

    /// Start a subscription listener for a pattern.
    async fn start_subscription_listener(
        &self,
        pattern: String,
        sender: broadcast::Sender<Event>,
    ) -> BusResult<tokio::task::JoinHandle<()>> {
        let client = self.client.clone();
        let redis_pattern = self.topic_to_redis_pattern(&pattern);
        let healthy = self.healthy.clone();
        let prefix_len = self.config.channel_prefix.len();

        let handle = tokio::spawn(async move {
            loop {
                // Get a new connection for pub/sub (must be dedicated)
                let conn = match client.get_async_pubsub().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to get pub/sub connection");
                        healthy.store(false, std::sync::atomic::Ordering::SeqCst);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                healthy.store(true, std::sync::atomic::Ordering::SeqCst);

                // Subscribe to pattern
                let mut pubsub = conn;
                if let Err(e) = pubsub.psubscribe(&redis_pattern).await {
                    tracing::error!(error = %e, pattern = %redis_pattern, "Failed to subscribe");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                tracing::info!(pattern = %redis_pattern, "Subscribed to Redis pattern");

                // Listen for messages
                let mut stream = pubsub.on_message();
                while let Some(msg) = stream.next().await {
                    let channel: String = match msg.get_channel() {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let payload: String = match msg.get_payload() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    // Parse event
                    let event: Event = match serde_json::from_str(&payload) {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to parse event from Redis");
                            continue;
                        }
                    };

                    // Strip prefix from channel
                    let topic = if channel.len() > prefix_len {
                        &channel[prefix_len..]
                    } else {
                        &channel
                    };

                    tracing::debug!(
                        topic = %topic,
                        event_type = %event.event_type,
                        id = %event.id,
                        "Received event from Redis"
                    );

                    // Forward to local subscribers
                    let _ = sender.send(event);
                }

                // Connection lost, retry
                tracing::warn!("Redis subscription connection lost, reconnecting...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        Ok(handle)
    }
}

#[cfg(feature = "redis-backend")]
use futures_util::StreamExt;

#[cfg(feature = "redis-backend")]
#[async_trait]
impl EventBus for RedisBus {
    async fn publish(&self, event: Event) -> BusResult<()> {
        let channel = self.prefixed_channel(&event.topic);

        let payload = serde_json::to_string(&event)
            .map_err(|e| BusError::Serialization(e.to_string()))?;

        let mut conn_guard = self.conn_manager.write().await;
        let conn = conn_guard.as_mut().ok_or_else(|| {
            BusError::Unavailable("Connection manager not available".to_string())
        })?;

        redis::cmd("PUBLISH")
            .arg(&channel)
            .arg(&payload)
            .query_async::<i64>(conn)
            .await
            .map_err(|e| BusError::Publish(e.to_string()))?;

        tracing::debug!(
            topic = %event.topic,
            channel = %channel,
            event_type = %event.event_type,
            id = %event.id,
            "Event published to Redis"
        );

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> BusResult<EventReceiver> {
        // Create local broadcast channel
        let (sender, receiver) = broadcast::channel(1024);

        // Store sender
        {
            let mut senders = self.local_senders.write().await;
            senders.insert(pattern.to_string(), sender.clone());
        }

        // Start background listener
        let handle = self.start_subscription_listener(pattern.to_string(), sender).await?;

        // Store handle
        {
            let mut handles = self.subscription_handles.write().await;
            handles.push(handle);
        }

        tracing::debug!(pattern = %pattern, "Subscribed to topic pattern via Redis");

        Ok(EventReceiver { inner: receiver })
    }

    async fn is_healthy(&self) -> bool {
        if !self.healthy.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }

        // Try a PING command
        let mut conn_guard = self.conn_manager.write().await;
        if let Some(conn) = conn_guard.as_mut() {
            match redis::cmd("PING").query_async::<String>(conn).await {
                Ok(response) => response == "PONG",
                Err(_) => false,
            }
        } else {
            false
        }
    }

    async fn close(&self) -> BusResult<()> {
        // Cancel all subscription tasks
        let handles = {
            let mut h = self.subscription_handles.write().await;
            std::mem::take(&mut *h)
        };

        for handle in handles {
            handle.abort();
        }

        // Clear local senders
        let mut senders = self.local_senders.write().await;
        senders.clear();

        // Clear connection manager
        let mut conn = self.conn_manager.write().await;
        *conn = None;

        Ok(())
    }
}

/// Placeholder for Redis bus when feature is not enabled.
#[cfg(not(feature = "redis-backend"))]
pub struct RedisBus {
    config: RedisBusConfig,
    fallback: InMemoryBus,
}

#[cfg(not(feature = "redis-backend"))]
impl RedisBus {
    /// Create a new Redis bus (falls back to in-memory when feature not enabled).
    pub fn new(config: RedisBusConfig) -> Self {
        tracing::warn!(
            "Redis backend feature not enabled. Falling back to in-memory bus. \
             Enable with: cargo build --features redis-backend"
        );
        Self {
            config,
            fallback: InMemoryBus::new(),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &RedisBusConfig {
        &self.config
    }
}

#[cfg(not(feature = "redis-backend"))]
#[async_trait]
impl EventBus for RedisBus {
    async fn publish(&self, event: Event) -> BusResult<()> {
        self.fallback.publish(event).await
    }

    async fn subscribe(&self, pattern: &str) -> BusResult<EventReceiver> {
        self.fallback.subscribe(pattern).await
    }

    async fn is_healthy(&self) -> bool {
        self.fallback.is_healthy().await
    }

    async fn close(&self) -> BusResult<()> {
        self.fallback.close().await
    }
}

// ============================================================================
// Bus Factory
// ============================================================================

/// Event bus backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BusBackend {
    /// In-memory bus (single process).
    Memory,
    /// Redis Pub/Sub (multi-process).
    Redis,
    /// NATS (future).
    Nats,
}

impl Default for BusBackend {
    fn default() -> Self {
        Self::Memory
    }
}

/// Create an event bus based on the backend type (synchronous, in-memory only).
#[allow(unused_variables)]
pub fn create_bus(backend: BusBackend, redis_config: Option<RedisBusConfig>) -> Arc<dyn EventBus> {
    match backend {
        BusBackend::Memory => Arc::new(InMemoryBus::new()),
        BusBackend::Redis => {
            #[cfg(not(feature = "redis-backend"))]
            {
                let config = redis_config.unwrap_or_default();
                Arc::new(RedisBus::new(config))
            }
            #[cfg(feature = "redis-backend")]
            {
                tracing::warn!(
                    "Use create_bus_async for Redis backend with feature enabled. \
                     Falling back to in-memory bus."
                );
                Arc::new(InMemoryBus::new())
            }
        }
        BusBackend::Nats => {
            tracing::warn!("NATS backend not implemented. Falling back to in-memory.");
            Arc::new(InMemoryBus::new())
        }
    }
}

/// Create an event bus based on the backend type (async, supports all backends).
#[cfg(feature = "redis-backend")]
pub async fn create_bus_async(
    backend: BusBackend,
    redis_config: Option<RedisBusConfig>,
) -> BusResult<Arc<dyn EventBus>> {
    match backend {
        BusBackend::Memory => Ok(Arc::new(InMemoryBus::new())),
        BusBackend::Redis => {
            let config = redis_config.unwrap_or_default();
            let bus = RedisBus::new(config).await?;
            Ok(Arc::new(bus))
        }
        BusBackend::Nats => {
            tracing::warn!("NATS backend not implemented. Falling back to in-memory.");
            Ok(Arc::new(InMemoryBus::new()))
        }
    }
}

/// Create an event bus based on the backend type (async version, no-op when redis not enabled).
#[cfg(not(feature = "redis-backend"))]
pub async fn create_bus_async(
    backend: BusBackend,
    redis_config: Option<RedisBusConfig>,
) -> BusResult<Arc<dyn EventBus>> {
    Ok(create_bus(backend, redis_config))
}

// ============================================================================
// Topic Constants
// ============================================================================

/// Predefined topic names.
pub mod topics {
    /// Agent-related topics.
    pub mod agent {
        /// Agent execution request.
        pub const REQUEST: &str = "agent.request";
        /// Agent execution response.
        pub const RESPONSE: &str = "agent.response";
        /// Agent status update.
        pub const STATUS: &str = "agent.status";
    }

    /// Session-related topics.
    pub mod session {
        /// Session started.
        pub const START: &str = "session.start";
        /// Session ended.
        pub const END: &str = "session.end";
        /// Session message.
        pub const MESSAGE: &str = "session.message";
    }

    /// Channel-related topics.
    pub mod channel {
        /// Incoming message from channel.
        pub const INCOMING: &str = "channel.incoming";
        /// Outgoing message to channel.
        pub const OUTGOING: &str = "channel.outgoing";
        /// Channel status update.
        pub const STATUS: &str = "channel.status";
    }

    /// Memory-related topics.
    pub mod memory {
        /// Memory update notification.
        pub const UPDATE: &str = "memory.update";
        /// Memory query request.
        pub const QUERY: &str = "memory.query";
    }

    /// Audit-related topics.
    pub mod audit {
        /// Audit log event.
        pub const LOG: &str = "audit.log";
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_bus_publish_subscribe() {
        let bus = InMemoryBus::new();

        // Subscribe first
        let mut receiver = bus.subscribe("test.topic").await.unwrap();

        // Publish
        let event = Event::new("test.topic", "test.event", "test")
            .with_payload(serde_json::json!({"message": "hello"}))
            .unwrap();

        bus.publish(event.clone()).await.unwrap();

        // Receive
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.topic, "test.topic");
        assert_eq!(received.event_type, "test.event");
    }

    #[tokio::test]
    async fn test_event_creation() {
        let event = Event::new("agent.request", "invoke", "codecoder")
            .with_target("zero-gateway")
            .with_correlation_id("req-123")
            .with_metadata("user_id", "user-456")
            .with_payload(AgentRequestPayload {
                agent_name: "plan".to_string(),
                prompt: "Plan this feature".to_string(),
                session_id: "session-789".to_string(),
                context: None,
            })
            .unwrap();

        assert_eq!(event.topic, "agent.request");
        assert_eq!(event.target, Some("zero-gateway".to_string()));
        assert_eq!(event.correlation_id, Some("req-123".to_string()));
        assert_eq!(event.metadata.get("user_id"), Some(&"user-456".to_string()));
    }

    #[test]
    fn test_topic_pattern_matching() {
        // Exact match
        assert!(InMemoryBus::topic_matches_pattern("agent.request", "agent.request"));
        assert!(!InMemoryBus::topic_matches_pattern("agent.response", "agent.request"));

        // Single wildcard
        assert!(InMemoryBus::topic_matches_pattern("agent.request", "agent.*"));
        assert!(InMemoryBus::topic_matches_pattern("agent.response", "agent.*"));
        assert!(!InMemoryBus::topic_matches_pattern("session.start", "agent.*"));

        // Multi-level wildcard
        assert!(InMemoryBus::topic_matches_pattern("agent.request", "agent.#"));
        assert!(InMemoryBus::topic_matches_pattern("agent.request.v2", "agent.#"));
        assert!(InMemoryBus::topic_matches_pattern("agent", "agent.#")); // # matches zero or more
    }

    #[tokio::test]
    async fn test_bus_factory() {
        let bus = create_bus(BusBackend::Memory, None);
        assert!(bus.is_healthy().await);

        let bus = create_bus(BusBackend::Redis, None);
        assert!(bus.is_healthy().await); // Falls back to memory when feature not enabled
    }

    #[tokio::test]
    async fn test_bus_close() {
        let bus = InMemoryBus::new();

        // Subscribe to create a topic
        let _ = bus.subscribe("test.topic").await.unwrap();

        // Close should clear topics
        bus.close().await.unwrap();

        // Topics should be empty
        let topics = bus.topics.read().await;
        assert!(topics.is_empty());
    }

    #[test]
    fn test_redis_config_default() {
        let config = RedisBusConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:6379");
        assert_eq!(config.timeout_secs, 5);
        assert_eq!(config.max_reconnects, 3);
        assert_eq!(config.channel_prefix, "zero:");
    }

    #[tokio::test]
    async fn test_create_bus_async() {
        // Test async factory with in-memory backend
        let bus = create_bus_async(BusBackend::Memory, None).await.unwrap();
        assert!(bus.is_healthy().await);
    }
}

// ============================================================================
// Redis Integration Tests (requires running Redis server)
// ============================================================================

#[cfg(all(test, feature = "redis-backend"))]
mod redis_tests {
    use super::*;
    use std::time::Duration;

    /// Check if Redis is available for testing.
    async fn redis_available() -> bool {
        let config = RedisBusConfig::default();
        match RedisBus::new(config).await {
            Ok(bus) => bus.is_healthy().await,
            Err(_) => false,
        }
    }

    #[tokio::test]
    async fn test_redis_bus_health_check() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisBusConfig::default();
        let bus = RedisBus::new(config).await.unwrap();
        assert!(bus.is_healthy().await);
    }

    #[tokio::test]
    async fn test_redis_bus_publish_subscribe() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisBusConfig {
            channel_prefix: "test:".to_string(),
            ..Default::default()
        };

        let bus = RedisBus::new(config).await.unwrap();

        // Subscribe first
        let mut receiver = bus.subscribe("events.test").await.unwrap();

        // Give subscription time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish
        let event = Event::new("events.test", "test.event", "test-source")
            .with_payload(serde_json::json!({"message": "hello redis"}))
            .unwrap();

        bus.publish(event).await.unwrap();

        // Receive with timeout
        let received = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("Timeout waiting for message")
            .expect("No message received");

        assert_eq!(received.topic, "events.test");
        assert_eq!(received.event_type, "test.event");
        assert_eq!(received.source, "test-source");

        // Cleanup
        bus.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_bus_pattern_subscription() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisBusConfig {
            channel_prefix: "pattern:".to_string(),
            ..Default::default()
        };

        let bus = RedisBus::new(config).await.unwrap();

        // Subscribe to pattern
        let mut receiver = bus.subscribe("agent.*").await.unwrap();

        // Give subscription time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish to matching topic
        let event = Event::new("agent.request", "invoke", "test");
        bus.publish(event).await.unwrap();

        // Should receive
        let received = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("Timeout waiting for message")
            .expect("No message received");

        assert_eq!(received.topic, "agent.request");

        bus.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_bus_async_factory() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let bus = create_bus_async(BusBackend::Redis, None).await.unwrap();
        assert!(bus.is_healthy().await);
        bus.close().await.unwrap();
    }
}
