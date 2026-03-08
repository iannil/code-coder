//! Forum round and turn scheduling.
//!
//! This module provides scheduling logic for multi-agent forum discussions,
//! including turn order management, timeout handling, and round progression.
//!
//! ## Design Principle
//!
//! Scheduling is **deterministic** - it follows fixed rules for turn order
//! and timeout handling. No LLM reasoning is required.

use super::session::{ForumSession, ForumTurn, SessionError, SessionStatus};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the forum scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Default round timeout in seconds
    pub default_round_timeout_secs: u64,
    /// Default turn timeout in seconds
    pub default_turn_timeout_secs: u64,
    /// Grace period for late responses in seconds
    #[serde(default = "default_grace_period")]
    pub grace_period_secs: u64,
    /// Whether to skip unresponsive participants
    #[serde(default)]
    pub skip_unresponsive: bool,
    /// Maximum consecutive skips before removal
    #[serde(default = "default_max_skips")]
    pub max_consecutive_skips: u32,
}

fn default_grace_period() -> u64 {
    5
}

fn default_max_skips() -> u32 {
    2
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            default_round_timeout_secs: 300,
            default_turn_timeout_secs: 60,
            grace_period_secs: default_grace_period(),
            skip_unresponsive: true,
            max_consecutive_skips: default_max_skips(),
        }
    }
}

/// Tracks skip counts for participants.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SkipTracker {
    consecutive_skips: HashMap<String, u32>,
}

impl SkipTracker {
    /// Create a new skip tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a skip for a participant.
    pub fn record_skip(&mut self, participant: &str) {
        let count = self.consecutive_skips.entry(participant.to_string()).or_insert(0);
        *count += 1;
    }

    /// Reset skip count for a participant (when they respond).
    pub fn reset(&mut self, participant: &str) {
        self.consecutive_skips.remove(participant);
    }

    /// Get consecutive skip count for a participant.
    pub fn get_skip_count(&self, participant: &str) -> u32 {
        self.consecutive_skips.get(participant).copied().unwrap_or(0)
    }

    /// Check if participant should be removed.
    pub fn should_remove(&self, participant: &str, max_skips: u32) -> bool {
        self.get_skip_count(participant) >= max_skips
    }
}

/// Result of a scheduling decision.
#[derive(Debug, Clone)]
pub enum ScheduleAction {
    /// Wait for current speaker to respond
    WaitForSpeaker {
        speaker: String,
        deadline: DateTime<Utc>,
    },
    /// Skip current speaker due to timeout
    SkipSpeaker {
        speaker: String,
        reason: String,
    },
    /// Remove participant from session
    RemoveParticipant {
        participant: String,
        reason: String,
    },
    /// Start next round
    StartRound {
        round: u32,
    },
    /// Session is complete
    Complete,
    /// Session needs to be timed out
    Timeout,
}

/// Forum scheduler for managing turn order and timing.
pub struct ForumScheduler {
    config: SchedulerConfig,
    skip_tracker: SkipTracker,
    /// Timestamp when current turn started
    turn_started_at: Option<DateTime<Utc>>,
    /// Timestamp when current round started
    round_started_at: Option<DateTime<Utc>>,
}

impl ForumScheduler {
    /// Create a new forum scheduler with default configuration.
    pub fn new() -> Self {
        Self {
            config: SchedulerConfig::default(),
            skip_tracker: SkipTracker::new(),
            turn_started_at: None,
            round_started_at: None,
        }
    }

    /// Create a scheduler with custom configuration.
    pub fn with_config(config: SchedulerConfig) -> Self {
        Self {
            config,
            skip_tracker: SkipTracker::new(),
            turn_started_at: None,
            round_started_at: None,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    /// Start tracking a new turn.
    pub fn start_turn(&mut self) {
        self.turn_started_at = Some(Utc::now());
    }

    /// Start tracking a new round.
    pub fn start_round(&mut self) {
        self.round_started_at = Some(Utc::now());
        self.turn_started_at = None;
    }

    /// Check if turn has timed out.
    pub fn is_turn_timed_out(&self, session: &ForumSession) -> bool {
        if let Some(started) = self.turn_started_at {
            let timeout = Duration::seconds(session.config.turn_timeout_secs as i64);
            let grace = Duration::seconds(self.config.grace_period_secs as i64);
            Utc::now() > started + timeout + grace
        } else {
            false
        }
    }

    /// Check if round has timed out.
    pub fn is_round_timed_out(&self, session: &ForumSession) -> bool {
        if let Some(started) = self.round_started_at {
            let timeout = Duration::seconds(session.config.round_timeout_secs as i64);
            Utc::now() > started + timeout
        } else {
            false
        }
    }

    /// Get the next speaker for the session.
    pub fn next_speaker<'a>(&self, session: &'a ForumSession) -> Option<&'a str> {
        session.current_speaker()
    }

    /// Determine if the session is complete based on current state.
    pub fn is_complete(&self, session: &ForumSession) -> bool {
        session.is_complete()
    }

    /// Decide what action to take next.
    pub fn decide(&mut self, session: &ForumSession) -> ScheduleAction {
        // Check if session is already complete
        if session.is_complete() {
            return ScheduleAction::Complete;
        }

        // Check if session is not active
        if session.status != SessionStatus::Active {
            return ScheduleAction::Complete;
        }

        // Check round timeout
        if self.is_round_timed_out(session) {
            return ScheduleAction::Timeout;
        }

        // Get current speaker
        let speaker = match session.current_speaker() {
            Some(s) => s.to_string(),
            None => return ScheduleAction::Complete,
        };

        // Check turn timeout
        if self.is_turn_timed_out(session) {
            // Record skip
            self.skip_tracker.record_skip(&speaker);

            // Check if should remove
            if self.skip_tracker.should_remove(&speaker, self.config.max_consecutive_skips) {
                return ScheduleAction::RemoveParticipant {
                    participant: speaker,
                    reason: "Too many consecutive skips".to_string(),
                };
            }

            if self.config.skip_unresponsive {
                return ScheduleAction::SkipSpeaker {
                    speaker,
                    reason: "Turn timeout".to_string(),
                };
            }
        }

        // Normal case: wait for speaker
        let deadline = self.turn_started_at
            .unwrap_or_else(Utc::now)
            + Duration::seconds(session.config.turn_timeout_secs as i64);

        ScheduleAction::WaitForSpeaker {
            speaker,
            deadline,
        }
    }

    /// Record that a speaker has responded.
    pub fn record_response(&mut self, speaker: &str) {
        self.skip_tracker.reset(speaker);
        self.turn_started_at = None;
    }

    /// Skip the current speaker.
    pub fn skip_current(&mut self, session: &mut ForumSession) -> Result<(), SessionError> {
        if let Some(speaker) = session.current_speaker() {
            self.skip_tracker.record_skip(speaker);
        }
        // Record an empty turn to advance
        session.record_turn("[skipped]")?;
        self.turn_started_at = None;
        Ok(())
    }

    /// Get the skip tracker state for serialization.
    pub fn skip_tracker(&self) -> &SkipTracker {
        &self.skip_tracker
    }

    /// Restore skip tracker state.
    pub fn restore_skip_tracker(&mut self, tracker: SkipTracker) {
        self.skip_tracker = tracker;
    }

    /// Calculate time remaining for current turn.
    pub fn time_remaining(&self, session: &ForumSession) -> Option<Duration> {
        self.turn_started_at.map(|started| {
            let timeout = Duration::seconds(session.config.turn_timeout_secs as i64);
            let deadline = started + timeout;
            let remaining = deadline - Utc::now();
            if remaining.num_seconds() > 0 {
                remaining
            } else {
                Duration::zero()
            }
        })
    }

    /// Get timing statistics.
    pub fn timing_stats(&self) -> SchedulerStats {
        SchedulerStats {
            turn_started_at: self.turn_started_at,
            round_started_at: self.round_started_at,
        }
    }
}

impl Default for ForumScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Timing statistics from the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub turn_started_at: Option<DateTime<Utc>>,
    pub round_started_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::forum::session::SessionConfig;

    fn create_test_session() -> ForumSession {
        ForumSession::new(
            "test-1",
            "Test topic",
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        )
        .with_config(SessionConfig {
            max_rounds: 3,
            round_timeout_secs: 300,
            turn_timeout_secs: 60,
            ..Default::default()
        })
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = ForumScheduler::new();
        assert_eq!(scheduler.config().default_turn_timeout_secs, 60);
        assert_eq!(scheduler.config().default_round_timeout_secs, 300);
    }

    #[test]
    fn test_scheduler_with_config() {
        let config = SchedulerConfig {
            default_turn_timeout_secs: 30,
            default_round_timeout_secs: 120,
            ..Default::default()
        };
        let scheduler = ForumScheduler::with_config(config);
        assert_eq!(scheduler.config().default_turn_timeout_secs, 30);
    }

    #[test]
    fn test_decide_wait_for_speaker() {
        let mut session = create_test_session();
        session.start().unwrap();

        let mut scheduler = ForumScheduler::new();
        scheduler.start_turn();

        let action = scheduler.decide(&session);
        match action {
            ScheduleAction::WaitForSpeaker { speaker, .. } => {
                assert_eq!(speaker, "a");
            }
            _ => panic!("Expected WaitForSpeaker action"),
        }
    }

    #[test]
    fn test_decide_complete() {
        let mut session = create_test_session();
        session.start().unwrap();
        session.complete();

        let mut scheduler = ForumScheduler::new();
        let action = scheduler.decide(&session);

        assert!(matches!(action, ScheduleAction::Complete));
    }

    #[test]
    fn test_skip_tracker() {
        let mut tracker = SkipTracker::new();

        assert_eq!(tracker.get_skip_count("a"), 0);

        tracker.record_skip("a");
        assert_eq!(tracker.get_skip_count("a"), 1);

        tracker.record_skip("a");
        assert_eq!(tracker.get_skip_count("a"), 2);

        assert!(tracker.should_remove("a", 2));
        assert!(!tracker.should_remove("a", 3));

        tracker.reset("a");
        assert_eq!(tracker.get_skip_count("a"), 0);
    }

    #[test]
    fn test_record_response() {
        let mut scheduler = ForumScheduler::new();

        scheduler.skip_tracker.record_skip("a");
        assert_eq!(scheduler.skip_tracker.get_skip_count("a"), 1);

        scheduler.record_response("a");
        assert_eq!(scheduler.skip_tracker.get_skip_count("a"), 0);
    }

    #[test]
    fn test_next_speaker() {
        let mut session = create_test_session();
        session.start().unwrap();

        let scheduler = ForumScheduler::new();

        assert_eq!(scheduler.next_speaker(&session), Some("a"));

        session.record_turn("response").unwrap();
        assert_eq!(scheduler.next_speaker(&session), Some("b"));
    }

    #[test]
    fn test_timing_stats() {
        let mut scheduler = ForumScheduler::new();

        let stats = scheduler.timing_stats();
        assert!(stats.turn_started_at.is_none());
        assert!(stats.round_started_at.is_none());

        scheduler.start_round();
        scheduler.start_turn();

        let stats = scheduler.timing_stats();
        assert!(stats.turn_started_at.is_some());
        assert!(stats.round_started_at.is_some());
    }

    #[test]
    fn test_time_remaining() {
        let session = create_test_session();
        let mut scheduler = ForumScheduler::new();

        // No turn started
        assert!(scheduler.time_remaining(&session).is_none());

        // Start turn
        scheduler.start_turn();
        let remaining = scheduler.time_remaining(&session);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().num_seconds() > 0);
    }

    #[test]
    fn test_skip_current() {
        let mut session = create_test_session();
        session.start().unwrap();

        let mut scheduler = ForumScheduler::new();

        assert_eq!(session.current_speaker(), Some("a"));

        scheduler.skip_current(&mut session).unwrap();

        assert_eq!(session.current_speaker(), Some("b"));
        assert_eq!(scheduler.skip_tracker.get_skip_count("a"), 1);
    }
}
