//! Checkpoint Manager for Event Stream Resume
//!
//! Manages checkpoint persistence for event stream consumption, enabling
//! zero-channels to resume from the last processed event after reconnection.
//!
//! # Design
//!
//! Checkpoints are stored in Redis with the key pattern:
//! - `checkpoints:{channel_type}:{channel_id}` - Last processed event ID per channel
//!
//! This allows different channels to resume independently and prevents
//! duplicate processing of events.

use anyhow::Result;
use std::sync::Arc;
use zero_core::common::RedisStreamClient;

// ============================================================================
// Checkpoint Keys
// ============================================================================

/// Generate checkpoint key for a channel.
pub fn checkpoint_key(channel_type: &str, channel_id: &str) -> String {
    format!("checkpoints:{}:{}", channel_type, channel_id)
}

/// Generate checkpoint key for a specific task.
pub fn task_checkpoint_key(task_id: &str) -> String {
    format!("checkpoints:task:{}", task_id)
}

// ============================================================================
// Checkpoint Data
// ============================================================================

/// Checkpoint data for a channel subscription.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Last processed event ID (Redis Stream message ID).
    pub last_id: String,
    /// Task ID being processed (if any).
    pub task_id: Option<String>,
    /// Last update timestamp (Unix millis).
    pub updated_at: u64,
    /// Number of events processed.
    pub event_count: u64,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self {
            last_id: "0".to_string(),
            task_id: None,
            updated_at: 0,
            event_count: 0,
        }
    }
}

// ============================================================================
// Checkpoint Manager
// ============================================================================

/// Manages checkpoint persistence for event stream resume.
pub struct CheckpointManager {
    client: Arc<RedisStreamClient>,
    /// TTL for checkpoint keys in seconds (default: 24 hours)
    ttl_secs: u64,
}

impl CheckpointManager {
    /// Create a new checkpoint manager.
    pub fn new(client: Arc<RedisStreamClient>) -> Self {
        Self {
            client,
            ttl_secs: 86400, // 24 hours
        }
    }

    /// Create with custom TTL.
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    /// Load checkpoint for a task.
    pub async fn load(&self, task_id: &str) -> Result<Checkpoint> {
        let key = task_checkpoint_key(task_id);
        let fields = self.client.hgetall(&key).await?;

        if fields.is_empty() {
            return Ok(Checkpoint::default());
        }

        Ok(Checkpoint {
            last_id: fields.get("last_id").cloned().unwrap_or_else(|| "0".to_string()),
            task_id: fields.get("task_id").cloned(),
            updated_at: fields
                .get("updated_at")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            event_count: fields
                .get("event_count")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        })
    }

    /// Save checkpoint for a task.
    pub async fn save(&self, task_id: &str, checkpoint: &Checkpoint) -> Result<()> {
        let key = task_checkpoint_key(task_id);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Bind strings to variables to extend their lifetimes
        let now_str = now.to_string();
        let event_count_str = checkpoint.event_count.to_string();

        let fields = vec![
            ("last_id", checkpoint.last_id.as_str()),
            ("updated_at", now_str.as_str()),
            ("event_count", event_count_str.as_str()),
        ];

        // Add task_id if present
        let fields: Vec<(&str, &str)> = if let Some(ref tid) = checkpoint.task_id {
            let mut f = fields;
            f.push(("task_id", tid.as_str()));
            f
        } else {
            fields
        };

        self.client.hset(&key, &fields).await?;

        // Set TTL on the key
        self.client.expire(&key, self.ttl_secs).await?;

        Ok(())
    }

    /// Clear checkpoint for a task.
    pub async fn clear(&self, task_id: &str) -> Result<()> {
        let key = task_checkpoint_key(task_id);
        self.client.del(&key).await?;
        Ok(())
    }

    /// Update checkpoint with new event.
    pub async fn update(
        &self,
        task_id: &str,
        last_id: &str,
        increment_count: bool,
    ) -> Result<()> {
        let key = task_checkpoint_key(task_id);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Bind string to variable to extend its lifetime
        let now_str = now.to_string();

        // Update fields
        let fields = vec![
            ("last_id", last_id),
            ("updated_at", now_str.as_str()),
        ];

        self.client.hset(&key, &fields).await?;

        // Increment event count if requested
        if increment_count {
            self.client.hincrby(&key, "event_count", 1).await?;
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_key_generation() {
        assert_eq!(
            checkpoint_key("telegram", "123456"),
            "checkpoints:telegram:123456"
        );
        assert_eq!(
            task_checkpoint_key("task_abc123"),
            "checkpoints:task:task_abc123"
        );
    }

    #[test]
    fn test_default_checkpoint() {
        let cp = Checkpoint::default();
        assert_eq!(cp.last_id, "0");
        assert!(cp.task_id.is_none());
        assert_eq!(cp.event_count, 0);
    }
}
