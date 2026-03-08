//! Task Timeout Manager
//!
//! Monitors task heartbeats and handles timeouts for the event-sourcing task system.
//!
//! # Timeout Strategy
//!
//! 1. **Heartbeat-based**: Tasks send heartbeat events every 30s
//! 2. **Stale detection**: Tasks with no heartbeat for `pendingTimeoutMs` are considered stale
//! 3. **Tool timeout**: Individual tool calls have a `toolTimeoutMs` limit
//! 4. **Global timeout**: Tasks have a maximum duration of `globalTimeoutMs`
//!
//! # Usage
//!
//! ```ignore
//! let config = TimeoutConfig::default();
//! let monitor = TimeoutMonitor::new(config);
//!
//! // Check if a task has timed out
//! if let Some(reason) = monitor.check_timeout(&task_state) {
//!     // Handle timeout
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Configuration
// ============================================================================

/// Task timeout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Pending task timeout (ms). Tasks without heartbeat for this duration are stale.
    pub pending_timeout_ms: u64,
    /// Heartbeat interval (ms). Expected interval between heartbeats.
    pub heartbeat_interval_ms: u64,
    /// Per-tool execution timeout (ms).
    pub tool_timeout_ms: u64,
    /// Global task timeout (ms). Maximum task duration.
    pub global_timeout_ms: u64,
    /// Progress warning threshold (ms). Warn if no progress for this duration.
    pub progress_warning_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            pending_timeout_ms: 300_000,     // 5 minutes
            heartbeat_interval_ms: 30_000,   // 30 seconds
            tool_timeout_ms: 60_000,         // 1 minute per tool
            global_timeout_ms: 1_800_000,    // 30 minutes total
            progress_warning_ms: 60_000,     // 1 minute without progress
        }
    }
}

// ============================================================================
// Timeout Reason
// ============================================================================

/// Reason for task timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutReason {
    /// No heartbeat received within pending timeout.
    NoHeartbeat {
        last_heartbeat: Option<DateTime<Utc>>,
        elapsed_ms: u64,
    },
    /// Global task duration exceeded.
    GlobalTimeout {
        started_at: DateTime<Utc>,
        elapsed_ms: u64,
        limit_ms: u64,
    },
    /// Tool execution exceeded timeout.
    ToolTimeout {
        tool_name: String,
        elapsed_ms: u64,
        limit_ms: u64,
    },
}

impl std::fmt::Display for TimeoutReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutReason::NoHeartbeat { elapsed_ms, .. } => {
                write!(
                    f,
                    "任务心跳超时 ({}秒无响应)",
                    elapsed_ms / 1000
                )
            }
            TimeoutReason::GlobalTimeout { elapsed_ms, limit_ms, .. } => {
                write!(
                    f,
                    "任务执行超时 (已运行{}分钟，上限{}分钟)",
                    elapsed_ms / 60_000,
                    limit_ms / 60_000
                )
            }
            TimeoutReason::ToolTimeout { tool_name, elapsed_ms, limit_ms } => {
                write!(
                    f,
                    "工具 {} 执行超时 ({}秒，上限{}秒)",
                    tool_name,
                    elapsed_ms / 1000,
                    limit_ms / 1000
                )
            }
        }
    }
}

// ============================================================================
// Task Timeout State
// ============================================================================

/// State tracked for timeout monitoring.
#[derive(Debug, Clone)]
pub struct TaskTimeoutState {
    /// Task ID.
    pub task_id: String,
    /// When the task started.
    pub started_at: DateTime<Utc>,
    /// Last heartbeat received.
    pub last_heartbeat: Option<DateTime<Utc>>,
    /// Last progress update.
    pub last_progress: Option<DateTime<Utc>>,
    /// Currently executing tool (if any).
    pub current_tool: Option<ToolExecution>,
}

/// Information about current tool execution.
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// Tool name.
    pub name: String,
    /// When the tool started.
    pub started_at: DateTime<Utc>,
}

impl TaskTimeoutState {
    /// Create a new timeout state for a task.
    pub fn new(task_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            task_id: task_id.into(),
            started_at: now,
            last_heartbeat: Some(now),
            last_progress: None,
            current_tool: None,
        }
    }

    /// Update heartbeat time.
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Some(Utc::now());
    }

    /// Update progress time.
    pub fn update_progress(&mut self) {
        let now = Utc::now();
        self.last_progress = Some(now);
        self.last_heartbeat = Some(now); // Progress implies heartbeat
    }

    /// Start tool execution tracking.
    pub fn start_tool(&mut self, name: impl Into<String>) {
        self.current_tool = Some(ToolExecution {
            name: name.into(),
            started_at: Utc::now(),
        });
    }

    /// End tool execution tracking.
    pub fn end_tool(&mut self) {
        self.current_tool = None;
        self.update_progress();
    }
}

// ============================================================================
// Timeout Monitor
// ============================================================================

/// Monitors task timeouts.
pub struct TimeoutMonitor {
    config: TimeoutConfig,
}

impl TimeoutMonitor {
    /// Create a new timeout monitor.
    pub fn new(config: TimeoutConfig) -> Self {
        Self { config }
    }

    /// Check if a task has timed out.
    ///
    /// Returns `Some(TimeoutReason)` if the task has timed out, `None` otherwise.
    pub fn check_timeout(&self, state: &TaskTimeoutState) -> Option<TimeoutReason> {
        let now = Utc::now();

        // Check global timeout
        let elapsed_ms = (now - state.started_at).num_milliseconds() as u64;
        if elapsed_ms > self.config.global_timeout_ms {
            return Some(TimeoutReason::GlobalTimeout {
                started_at: state.started_at,
                elapsed_ms,
                limit_ms: self.config.global_timeout_ms,
            });
        }

        // Check heartbeat timeout
        if let Some(last_hb) = state.last_heartbeat {
            let hb_elapsed_ms = (now - last_hb).num_milliseconds() as u64;
            if hb_elapsed_ms > self.config.pending_timeout_ms {
                return Some(TimeoutReason::NoHeartbeat {
                    last_heartbeat: state.last_heartbeat,
                    elapsed_ms: hb_elapsed_ms,
                });
            }
        } else {
            // No heartbeat ever received, check from start time
            if elapsed_ms > self.config.pending_timeout_ms {
                return Some(TimeoutReason::NoHeartbeat {
                    last_heartbeat: None,
                    elapsed_ms,
                });
            }
        }

        // Check tool timeout
        if let Some(ref tool) = state.current_tool {
            let tool_elapsed_ms = (now - tool.started_at).num_milliseconds() as u64;
            if tool_elapsed_ms > self.config.tool_timeout_ms {
                return Some(TimeoutReason::ToolTimeout {
                    tool_name: tool.name.clone(),
                    elapsed_ms: tool_elapsed_ms,
                    limit_ms: self.config.tool_timeout_ms,
                });
            }
        }

        None
    }

    /// Check if a progress warning should be issued.
    ///
    /// Returns `true` if no progress has been made within the warning threshold.
    pub fn should_warn_no_progress(&self, state: &TaskTimeoutState) -> bool {
        let now = Utc::now();
        let reference = state.last_progress.unwrap_or(state.started_at);
        let elapsed_ms = (now - reference).num_milliseconds() as u64;
        elapsed_ms > self.config.progress_warning_ms
    }

    /// Get time until next expected heartbeat.
    pub fn time_until_stale(&self, state: &TaskTimeoutState) -> Duration {
        let now = Utc::now();
        let last_hb = state.last_heartbeat.unwrap_or(state.started_at);
        let elapsed_ms = (now - last_hb).num_milliseconds() as u64;

        if elapsed_ms >= self.config.pending_timeout_ms {
            Duration::ZERO
        } else {
            Duration::from_millis(self.config.pending_timeout_ms - elapsed_ms)
        }
    }

    /// Get remaining time for global timeout.
    pub fn remaining_global_time(&self, state: &TaskTimeoutState) -> Duration {
        let now = Utc::now();
        let elapsed_ms = (now - state.started_at).num_milliseconds() as u64;

        if elapsed_ms >= self.config.global_timeout_ms {
            Duration::ZERO
        } else {
            Duration::from_millis(self.config.global_timeout_ms - elapsed_ms)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_no_timeout_within_limits() {
        let config = TimeoutConfig::default();
        let monitor = TimeoutMonitor::new(config);
        let state = TaskTimeoutState::new("test-task");

        assert!(monitor.check_timeout(&state).is_none());
    }

    #[test]
    fn test_global_timeout() {
        let config = TimeoutConfig {
            global_timeout_ms: 1000, // 1 second
            ..Default::default()
        };
        let monitor = TimeoutMonitor::new(config);

        let mut state = TaskTimeoutState::new("test-task");
        state.started_at = Utc::now() - ChronoDuration::seconds(2);

        let result = monitor.check_timeout(&state);
        assert!(matches!(result, Some(TimeoutReason::GlobalTimeout { .. })));
    }

    #[test]
    fn test_heartbeat_timeout() {
        let config = TimeoutConfig {
            pending_timeout_ms: 1000, // 1 second
            ..Default::default()
        };
        let monitor = TimeoutMonitor::new(config);

        let mut state = TaskTimeoutState::new("test-task");
        state.last_heartbeat = Some(Utc::now() - ChronoDuration::seconds(2));

        let result = monitor.check_timeout(&state);
        assert!(matches!(result, Some(TimeoutReason::NoHeartbeat { .. })));
    }

    #[test]
    fn test_tool_timeout() {
        let config = TimeoutConfig {
            tool_timeout_ms: 1000, // 1 second
            ..Default::default()
        };
        let monitor = TimeoutMonitor::new(config);

        let mut state = TaskTimeoutState::new("test-task");
        state.start_tool("slow_tool");
        state.current_tool.as_mut().unwrap().started_at =
            Utc::now() - ChronoDuration::seconds(2);

        let result = monitor.check_timeout(&state);
        assert!(matches!(result, Some(TimeoutReason::ToolTimeout { .. })));
    }

    #[test]
    fn test_progress_warning() {
        let config = TimeoutConfig {
            progress_warning_ms: 1000, // 1 second
            ..Default::default()
        };
        let monitor = TimeoutMonitor::new(config);

        let mut state = TaskTimeoutState::new("test-task");
        state.started_at = Utc::now() - ChronoDuration::seconds(2);
        state.last_heartbeat = Some(Utc::now()); // Still alive

        assert!(monitor.should_warn_no_progress(&state));

        // Update progress
        state.update_progress();
        assert!(!monitor.should_warn_no_progress(&state));
    }
}
