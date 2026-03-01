//! Redis Streams wrapper for Zero Ecosystem
//!
//! Provides persistent task queue and event sourcing capabilities using Redis Streams.
//!
//! # Key Features
//!
//! - **Persistent Queue**: Tasks survive restarts, no message loss
//! - **Consumer Groups**: Multiple workers can process tasks concurrently
//! - **Acknowledgement**: Explicit ACK ensures reliable processing
//! - **Replay**: Events can be replayed from any point (checkpoint resume)
//!
//! # Stream Keys
//!
//! - `tasks:pending` - Pending task queue (entry point)
//! - `tasks:events:{task_id}` - Per-task event stream (event sourcing)
//! - `tasks:state:{task_id}` - Task state projection (Redis Hash)

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "redis-backend")]
use redis::aio::ConnectionManager;
#[cfg(feature = "redis-backend")]
use redis::{AsyncCommands, FromRedisValue, RedisResult, ToRedisArgs, Value};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum RedisStreamError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Stream operation failed: {0}")]
    StreamOperation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Consumer group error: {0}")]
    ConsumerGroup(String),

    #[error("Message not found: {0}")]
    NotFound(String),

    #[error("Redis not available")]
    Unavailable,
}

pub type StreamResult<T> = Result<T, RedisStreamError>;

// ============================================================================
// Configuration
// ============================================================================

/// Redis Streams configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisStreamConfig {
    /// Redis URL (redis://host:port).
    pub url: String,
    /// Key prefix for namespacing.
    pub key_prefix: String,
    /// Consumer group name.
    pub consumer_group: String,
    /// Consumer name (unique per worker).
    pub consumer_name: String,
    /// Pending timeout in milliseconds (for auto-claim).
    pub pending_timeout_ms: u64,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Maximum retries before moving to dead letter.
    pub max_retries: u32,
    /// Maximum stream length (MAXLEN for trimming).
    pub max_stream_length: Option<u64>,
    /// Block timeout for XREADGROUP (0 = forever).
    pub block_timeout_ms: u64,
}

impl Default for RedisStreamConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:4410".to_string(),
            key_prefix: "codecoder:".to_string(),
            consumer_group: "ccode-workers".to_string(),
            consumer_name: format!("worker-{}", uuid::Uuid::new_v4()),
            pending_timeout_ms: 300_000, // 5 minutes
            heartbeat_interval_ms: 30_000, // 30 seconds
            max_retries: 3,
            max_stream_length: Some(10_000),
            block_timeout_ms: 5_000, // 5 seconds
        }
    }
}

// ============================================================================
// Stream Message Types
// ============================================================================

/// A message read from a stream.
#[derive(Debug, Clone)]
pub struct StreamMessage {
    /// Stream message ID (e.g., "1234567890123-0").
    pub id: String,
    /// Field-value pairs from the message.
    pub fields: HashMap<String, String>,
}

impl StreamMessage {
    /// Parse the payload field as JSON.
    pub fn parse_payload<T: DeserializeOwned>(&self) -> StreamResult<T> {
        let payload = self
            .fields
            .get("payload")
            .ok_or_else(|| RedisStreamError::NotFound("payload field not found".to_string()))?;

        serde_json::from_str(payload).map_err(|e| RedisStreamError::Serialization(e.to_string()))
    }

    /// Get a field value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }
}

/// Pending message info (from XPENDING).
#[derive(Debug, Clone)]
pub struct PendingMessage {
    /// Message ID.
    pub id: String,
    /// Consumer that owns this message.
    pub consumer: String,
    /// Idle time in milliseconds.
    pub idle_ms: u64,
    /// Delivery count.
    pub delivery_count: u32,
}

// ============================================================================
// Redis Stream Client
// ============================================================================

/// Redis Streams client for task queue and event sourcing.
#[cfg(feature = "redis-backend")]
pub struct RedisStreamClient {
    config: RedisStreamConfig,
    conn: ConnectionManager,
}

#[cfg(feature = "redis-backend")]
impl RedisStreamClient {
    /// Create a new Redis Stream client.
    pub async fn new(config: RedisStreamConfig) -> StreamResult<Self> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| RedisStreamError::Connection(e.to_string()))?;

        let conn = client
            .get_connection_manager()
            .await
            .map_err(|e| RedisStreamError::Connection(e.to_string()))?;

        Ok(Self { config, conn })
    }

    /// Get the configuration.
    pub fn config(&self) -> &RedisStreamConfig {
        &self.config
    }

    /// Get prefixed key.
    fn key(&self, name: &str) -> String {
        format!("{}{}", self.config.key_prefix, name)
    }

    /// Health check.
    pub async fn is_healthy(&self) -> bool {
        let mut conn = self.conn.clone();
        match redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
        {
            Ok(response) => response == "PONG",
            Err(_) => false,
        }
    }

    // ========================================================================
    // Stream Operations
    // ========================================================================

    /// Add a message to a stream (XADD).
    ///
    /// Returns the message ID.
    pub async fn xadd<T: Serialize>(
        &self,
        stream: &str,
        payload: &T,
    ) -> StreamResult<String> {
        let key = self.key(stream);
        let payload_json =
            serde_json::to_string(payload).map_err(|e| RedisStreamError::Serialization(e.to_string()))?;

        let mut conn = self.conn.clone();

        // Build XADD command with optional MAXLEN
        let mut cmd = redis::cmd("XADD");
        cmd.arg(&key);

        if let Some(max_len) = self.config.max_stream_length {
            cmd.arg("MAXLEN").arg("~").arg(max_len);
        }

        cmd.arg("*").arg("payload").arg(&payload_json);

        let id: String = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        tracing::debug!(stream = %key, id = %id, "Added message to stream");

        Ok(id)
    }

    /// Add a message with additional fields.
    pub async fn xadd_with_fields(
        &self,
        stream: &str,
        fields: &[(&str, &str)],
    ) -> StreamResult<String> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let mut cmd = redis::cmd("XADD");
        cmd.arg(&key);

        if let Some(max_len) = self.config.max_stream_length {
            cmd.arg("MAXLEN").arg("~").arg(max_len);
        }

        cmd.arg("*");
        for (k, v) in fields {
            cmd.arg(*k).arg(*v);
        }

        let id: String = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(id)
    }

    /// Read messages from a stream (XREAD).
    ///
    /// Use `last_id` = "0" to read from beginning, "$" to read only new messages.
    pub async fn xread(
        &self,
        stream: &str,
        last_id: &str,
        count: usize,
        block_ms: Option<u64>,
    ) -> StreamResult<Vec<StreamMessage>> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let mut cmd = redis::cmd("XREAD");
        cmd.arg("COUNT").arg(count);

        if let Some(block) = block_ms {
            cmd.arg("BLOCK").arg(block);
        }

        cmd.arg("STREAMS").arg(&key).arg(last_id);

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        let messages = result
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(_, entries)| {
                entries.into_iter().map(|(id, fields)| {
                    let field_map: HashMap<String, String> = fields.into_iter().collect();
                    StreamMessage { id, fields: field_map }
                })
            })
            .collect();

        Ok(messages)
    }

    /// Ensure consumer group exists (XGROUP CREATE).
    pub async fn ensure_consumer_group(&self, stream: &str) -> StreamResult<()> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        // Create group, starting from the beginning ("0")
        // MKSTREAM creates the stream if it doesn't exist
        let result: RedisResult<String> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&key)
            .arg(&self.config.consumer_group)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        match result {
            Ok(_) => {
                tracing::info!(
                    stream = %key,
                    group = %self.config.consumer_group,
                    "Created consumer group"
                );
                Ok(())
            }
            Err(e) => {
                // BUSYGROUP means group already exists - that's fine
                if e.to_string().contains("BUSYGROUP") {
                    tracing::debug!(
                        stream = %key,
                        group = %self.config.consumer_group,
                        "Consumer group already exists"
                    );
                    Ok(())
                } else {
                    Err(RedisStreamError::ConsumerGroup(e.to_string()))
                }
            }
        }
    }

    /// Read messages as a consumer group member (XREADGROUP).
    ///
    /// Use `last_id` = ">" to read only new messages assigned to this consumer.
    /// Use "0" to re-read pending messages.
    pub async fn xreadgroup(
        &self,
        stream: &str,
        last_id: &str,
        count: usize,
    ) -> StreamResult<Vec<StreamMessage>> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP")
            .arg(&self.config.consumer_group)
            .arg(&self.config.consumer_name)
            .arg("COUNT")
            .arg(count)
            .arg("BLOCK")
            .arg(self.config.block_timeout_ms)
            .arg("STREAMS")
            .arg(&key)
            .arg(last_id);

        let result: Option<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        let messages = result
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(_, entries)| {
                entries.into_iter().map(|(id, fields)| {
                    let field_map: HashMap<String, String> = fields.into_iter().collect();
                    StreamMessage { id, fields: field_map }
                })
            })
            .collect();

        Ok(messages)
    }

    /// Acknowledge message processing (XACK).
    pub async fn xack(&self, stream: &str, message_id: &str) -> StreamResult<()> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let _: u64 = redis::cmd("XACK")
            .arg(&key)
            .arg(&self.config.consumer_group)
            .arg(message_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        tracing::debug!(stream = %key, id = %message_id, "Acknowledged message");

        Ok(())
    }

    /// Get pending messages (XPENDING).
    pub async fn xpending(
        &self,
        stream: &str,
        count: usize,
    ) -> StreamResult<Vec<PendingMessage>> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        // XPENDING stream group [start end count] [consumer]
        let result: Vec<(String, String, u64, u32)> = redis::cmd("XPENDING")
            .arg(&key)
            .arg(&self.config.consumer_group)
            .arg("-") // start
            .arg("+") // end
            .arg(count)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        let pending = result
            .into_iter()
            .map(|(id, consumer, idle_ms, delivery_count)| PendingMessage {
                id,
                consumer,
                idle_ms,
                delivery_count,
            })
            .collect();

        Ok(pending)
    }

    /// Claim pending messages that have been idle too long (XCLAIM).
    pub async fn xclaim(
        &self,
        stream: &str,
        message_ids: &[String],
        min_idle_ms: u64,
    ) -> StreamResult<Vec<StreamMessage>> {
        if message_ids.is_empty() {
            return Ok(vec![]);
        }

        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let mut cmd = redis::cmd("XCLAIM");
        cmd.arg(&key)
            .arg(&self.config.consumer_group)
            .arg(&self.config.consumer_name)
            .arg(min_idle_ms);

        for id in message_ids {
            cmd.arg(id);
        }

        let result: Vec<(String, Vec<(String, String)>)> = cmd
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        let messages = result
            .into_iter()
            .map(|(id, fields)| {
                let field_map: HashMap<String, String> = fields.into_iter().collect();
                StreamMessage { id, fields: field_map }
            })
            .collect();

        Ok(messages)
    }

    /// Auto-claim messages that have been pending too long (XAUTOCLAIM).
    /// Redis 6.2+
    pub async fn xautoclaim(
        &self,
        stream: &str,
        min_idle_ms: u64,
        count: usize,
    ) -> StreamResult<Vec<StreamMessage>> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        // XAUTOCLAIM stream group consumer min-idle-time start [COUNT count]
        let result: (String, Vec<(String, Vec<(String, String)>)>, Vec<String>) =
            redis::cmd("XAUTOCLAIM")
                .arg(&key)
                .arg(&self.config.consumer_group)
                .arg(&self.config.consumer_name)
                .arg(min_idle_ms)
                .arg("0-0") // start from beginning of PEL
                .arg("COUNT")
                .arg(count)
                .query_async(&mut conn)
                .await
                .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        let messages = result
            .1
            .into_iter()
            .map(|(id, fields)| {
                let field_map: HashMap<String, String> = fields.into_iter().collect();
                StreamMessage { id, fields: field_map }
            })
            .collect();

        Ok(messages)
    }

    // ========================================================================
    // State Projection (Hash Operations)
    // ========================================================================

    /// Set task state fields (HSET).
    pub async fn hset(
        &self,
        key: &str,
        fields: &[(&str, &str)],
    ) -> StreamResult<()> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let mut cmd = redis::cmd("HSET");
        cmd.arg(&full_key);
        for (k, v) in fields {
            cmd.arg(*k).arg(*v);
        }

        cmd.query_async::<u64>(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(())
    }

    /// Get all task state fields (HGETALL).
    pub async fn hgetall(&self, key: &str) -> StreamResult<HashMap<String, String>> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let result: HashMap<String, String> = conn
            .hgetall(&full_key)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(result)
    }

    /// Get specific field from hash (HGET).
    pub async fn hget(&self, key: &str, field: &str) -> StreamResult<Option<String>> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let result: Option<String> = conn
            .hget(&full_key, field)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(result)
    }

    /// Increment a hash field (HINCRBY).
    pub async fn hincrby(&self, key: &str, field: &str, incr: i64) -> StreamResult<i64> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let result: i64 = conn
            .hincr(&full_key, field, incr)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(result)
    }

    /// Delete a key (DEL).
    pub async fn del(&self, key: &str) -> StreamResult<bool> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let result: u64 = conn
            .del(&full_key)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(result > 0)
    }

    /// Set key expiration (EXPIRE).
    pub async fn expire(&self, key: &str, seconds: u64) -> StreamResult<bool> {
        let full_key = self.key(key);
        let mut conn = self.conn.clone();

        let result: bool = conn
            .expire(&full_key, seconds as i64)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(result)
    }

    /// Get stream length (XLEN).
    pub async fn xlen(&self, stream: &str) -> StreamResult<u64> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        let len: u64 = redis::cmd("XLEN")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        Ok(len)
    }

    /// Get stream info (XINFO STREAM).
    pub async fn xinfo_stream(&self, stream: &str) -> StreamResult<HashMap<String, String>> {
        let key = self.key(stream);
        let mut conn = self.conn.clone();

        // This returns a complex nested structure, simplified to HashMap
        let result: Vec<Value> = redis::cmd("XINFO")
            .arg("STREAM")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RedisStreamError::StreamOperation(e.to_string()))?;

        // Parse key-value pairs from the flat array
        let mut info = HashMap::new();
        let mut iter = result.into_iter();
        while let Some(key) = iter.next() {
            if let (Value::BulkString(k), Some(v)) = (key, iter.next()) {
                let key_str = String::from_utf8_lossy(&k).to_string();
                let val_str = match v {
                    Value::BulkString(b) => String::from_utf8_lossy(&b).to_string(),
                    Value::Int(i) => i.to_string(),
                    _ => continue,
                };
                info.insert(key_str, val_str);
            }
        }

        Ok(info)
    }
}

// ============================================================================
// Placeholder when Redis feature not enabled
// ============================================================================

#[cfg(not(feature = "redis-backend"))]
pub struct RedisStreamClient {
    config: RedisStreamConfig,
}

#[cfg(not(feature = "redis-backend"))]
impl RedisStreamClient {
    pub async fn new(config: RedisStreamConfig) -> StreamResult<Self> {
        tracing::warn!(
            "Redis backend feature not enabled. Stream operations will fail. \
             Enable with: cargo build --features redis-backend"
        );
        Ok(Self { config })
    }

    pub fn config(&self) -> &RedisStreamConfig {
        &self.config
    }

    pub async fn is_healthy(&self) -> bool {
        false
    }

    pub async fn xadd<T: Serialize>(&self, _stream: &str, _payload: &T) -> StreamResult<String> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xadd_with_fields(
        &self,
        _stream: &str,
        _fields: &[(&str, &str)],
    ) -> StreamResult<String> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xread(
        &self,
        _stream: &str,
        _last_id: &str,
        _count: usize,
        _block_ms: Option<u64>,
    ) -> StreamResult<Vec<StreamMessage>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn ensure_consumer_group(&self, _stream: &str) -> StreamResult<()> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xreadgroup(
        &self,
        _stream: &str,
        _last_id: &str,
        _count: usize,
    ) -> StreamResult<Vec<StreamMessage>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xack(&self, _stream: &str, _message_id: &str) -> StreamResult<()> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xpending(&self, _stream: &str, _count: usize) -> StreamResult<Vec<PendingMessage>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xclaim(
        &self,
        _stream: &str,
        _message_ids: &[String],
        _min_idle_ms: u64,
    ) -> StreamResult<Vec<StreamMessage>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xautoclaim(
        &self,
        _stream: &str,
        _min_idle_ms: u64,
        _count: usize,
    ) -> StreamResult<Vec<StreamMessage>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn hset(&self, _key: &str, _fields: &[(&str, &str)]) -> StreamResult<()> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn hgetall(&self, _key: &str) -> StreamResult<HashMap<String, String>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn hget(&self, _key: &str, _field: &str) -> StreamResult<Option<String>> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn hincrby(&self, _key: &str, _field: &str, _incr: i64) -> StreamResult<i64> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn del(&self, _key: &str) -> StreamResult<bool> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn expire(&self, _key: &str, _seconds: u64) -> StreamResult<bool> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xlen(&self, _stream: &str) -> StreamResult<u64> {
        Err(RedisStreamError::Unavailable)
    }

    pub async fn xinfo_stream(&self, _stream: &str) -> StreamResult<HashMap<String, String>> {
        Err(RedisStreamError::Unavailable)
    }
}

// ============================================================================
// Stream Keys Helper
// ============================================================================

/// Standard stream keys for task queue.
pub mod stream_keys {
    /// Pending task queue (entry point).
    pub const TASKS_PENDING: &str = "tasks:pending";

    /// Per-task event stream prefix.
    /// Full key: `tasks:events:{task_id}`
    pub fn task_events(task_id: &str) -> String {
        format!("tasks:events:{}", task_id)
    }

    /// Per-task state projection prefix.
    /// Full key: `tasks:state:{task_id}`
    pub fn task_state(task_id: &str) -> String {
        format!("tasks:state:{}", task_id)
    }

    /// Dead letter queue for failed tasks.
    pub const TASKS_DEAD_LETTER: &str = "tasks:dead_letter";
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RedisStreamConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:4410");
        assert_eq!(config.key_prefix, "codecoder:");
        assert_eq!(config.consumer_group, "ccode-workers");
        assert!(config.consumer_name.starts_with("worker-"));
        assert_eq!(config.pending_timeout_ms, 300_000);
        assert_eq!(config.heartbeat_interval_ms, 30_000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_stream_keys() {
        assert_eq!(stream_keys::TASKS_PENDING, "tasks:pending");
        assert_eq!(
            stream_keys::task_events("task-123"),
            "tasks:events:task-123"
        );
        assert_eq!(
            stream_keys::task_state("task-123"),
            "tasks:state:task-123"
        );
    }

    #[test]
    fn test_stream_message_get() {
        let mut fields = HashMap::new();
        fields.insert("foo".to_string(), "bar".to_string());
        fields.insert("payload".to_string(), r#"{"key":"value"}"#.to_string());

        let msg = StreamMessage {
            id: "1234-0".to_string(),
            fields,
        };

        assert_eq!(msg.get("foo"), Some("bar"));
        assert_eq!(msg.get("missing"), None);
    }

    #[test]
    fn test_stream_message_parse_payload() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct TestPayload {
            key: String,
        }

        let mut fields = HashMap::new();
        fields.insert("payload".to_string(), r#"{"key":"value"}"#.to_string());

        let msg = StreamMessage {
            id: "1234-0".to_string(),
            fields,
        };

        let payload: TestPayload = msg.parse_payload().unwrap();
        assert_eq!(payload.key, "value");
    }
}

#[cfg(all(test, feature = "redis-backend"))]
mod redis_tests {
    use super::*;

    async fn redis_available() -> bool {
        match RedisStreamClient::new(RedisStreamConfig::default()).await {
            Ok(client) => client.is_healthy().await,
            Err(_) => false,
        }
    }

    #[tokio::test]
    async fn test_redis_stream_xadd_xread() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisStreamConfig {
            key_prefix: "test:stream:".to_string(),
            ..Default::default()
        };

        let client = RedisStreamClient::new(config).await.unwrap();

        // Add a message
        #[derive(Serialize)]
        struct TestMsg {
            content: String,
        }

        let msg = TestMsg {
            content: "hello".to_string(),
        };

        let id = client.xadd("test-stream", &msg).await.unwrap();
        assert!(!id.is_empty());

        // Read messages
        let messages = client.xread("test-stream", "0", 10, None).await.unwrap();
        assert!(!messages.is_empty());

        // Cleanup
        client.del("test-stream").await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_consumer_group() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisStreamConfig {
            key_prefix: "test:group:".to_string(),
            consumer_group: "test-group".to_string(),
            ..Default::default()
        };

        let client = RedisStreamClient::new(config).await.unwrap();

        // Create consumer group
        client.ensure_consumer_group("test-stream").await.unwrap();

        // Should be idempotent
        client.ensure_consumer_group("test-stream").await.unwrap();

        // Cleanup
        client.del("test-stream").await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_hash_operations() {
        if !redis_available().await {
            eprintln!("Skipping Redis test: Redis not available");
            return;
        }

        let config = RedisStreamConfig {
            key_prefix: "test:hash:".to_string(),
            ..Default::default()
        };

        let client = RedisStreamClient::new(config).await.unwrap();

        // Set fields
        client
            .hset("test-key", &[("field1", "value1"), ("field2", "value2")])
            .await
            .unwrap();

        // Get single field
        let val = client.hget("test-key", "field1").await.unwrap();
        assert_eq!(val, Some("value1".to_string()));

        // Get all fields
        let all = client.hgetall("test-key").await.unwrap();
        assert_eq!(all.get("field1"), Some(&"value1".to_string()));
        assert_eq!(all.get("field2"), Some(&"value2".to_string()));

        // Increment
        let new_val = client.hincrby("test-key", "counter", 5).await.unwrap();
        assert_eq!(new_val, 5);

        // Cleanup
        client.del("test-key").await.unwrap();
    }
}
