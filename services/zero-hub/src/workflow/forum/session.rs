//! Forum session state management.
//!
//! This module provides session state tracking for multi-agent forum discussions.
//! Sessions track participants, turns, rounds, and the complete discussion history.
//!
//! ## Design Principle
//!
//! Session state is **deterministic** - it tracks what has happened and manages
//! turn order according to fixed rules. No LLM reasoning is required here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a forum session.
pub type SessionId = String;

/// A single turn in the forum discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumTurn {
    /// Round number (1-indexed)
    pub round: u32,
    /// Speaker identifier (agent name)
    pub speaker: String,
    /// Content of the turn
    pub content: String,
    /// Timestamp when the turn was recorded
    pub timestamp: DateTime<Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ForumTurn {
    /// Create a new forum turn.
    pub fn new(round: u32, speaker: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            round,
            speaker: speaker.into(),
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the turn.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Status of a forum session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session is being set up
    Pending,
    /// Session is actively running
    Active,
    /// Session is paused
    Paused,
    /// Session completed normally
    Completed,
    /// Session was cancelled
    Cancelled,
    /// Session timed out
    TimedOut,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Configuration for a forum session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum number of rounds
    pub max_rounds: u32,
    /// Timeout per round in seconds
    pub round_timeout_secs: u64,
    /// Timeout per turn in seconds
    pub turn_timeout_secs: u64,
    /// Whether to allow late joiners
    #[serde(default)]
    pub allow_late_join: bool,
    /// Minimum participants to start
    #[serde(default = "default_min_participants")]
    pub min_participants: usize,
}

fn default_min_participants() -> usize {
    2
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            round_timeout_secs: 300, // 5 minutes per round
            turn_timeout_secs: 60,   // 1 minute per turn
            allow_late_join: false,
            min_participants: default_min_participants(),
        }
    }
}

/// A forum session representing a multi-agent discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumSession {
    /// Unique session identifier
    pub id: SessionId,
    /// Topic of discussion
    pub topic: String,
    /// List of participant agent identifiers
    pub participants: Vec<String>,
    /// Current round number (0 = not started)
    pub current_round: u32,
    /// Current speaker index in participants list
    current_speaker_idx: usize,
    /// Complete discussion history
    pub history: Vec<ForumTurn>,
    /// Session configuration
    pub config: SessionConfig,
    /// Session status
    pub status: SessionStatus,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional final summary
    pub summary: Option<String>,
}

impl ForumSession {
    /// Create a new forum session.
    pub fn new(
        id: impl Into<String>,
        topic: impl Into<String>,
        participants: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            topic: topic.into(),
            participants,
            current_round: 0,
            current_speaker_idx: 0,
            history: Vec::new(),
            config: SessionConfig::default(),
            status: SessionStatus::Pending,
            created_at: now,
            updated_at: now,
            summary: None,
        }
    }

    /// Create a session with custom configuration.
    pub fn with_config(mut self, config: SessionConfig) -> Self {
        self.config = config;
        self
    }

    /// Start the session.
    pub fn start(&mut self) -> Result<(), SessionError> {
        if self.status != SessionStatus::Pending {
            return Err(SessionError::InvalidState {
                expected: SessionStatus::Pending,
                actual: self.status,
            });
        }

        if self.participants.len() < self.config.min_participants {
            return Err(SessionError::NotEnoughParticipants {
                required: self.config.min_participants,
                actual: self.participants.len(),
            });
        }

        self.status = SessionStatus::Active;
        self.current_round = 1;
        self.current_speaker_idx = 0;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get the current speaker.
    pub fn current_speaker(&self) -> Option<&str> {
        if self.status != SessionStatus::Active {
            return None;
        }
        self.participants.get(self.current_speaker_idx).map(|s| s.as_str())
    }

    /// Check if the session is in a round.
    pub fn is_in_round(&self) -> bool {
        self.status == SessionStatus::Active && self.current_round > 0
    }

    /// Add a participant (if late join is allowed).
    pub fn add_participant(&mut self, participant: impl Into<String>) -> Result<(), SessionError> {
        if !self.config.allow_late_join && self.status != SessionStatus::Pending {
            return Err(SessionError::LateJoinNotAllowed);
        }

        let participant = participant.into();
        if self.participants.contains(&participant) {
            return Err(SessionError::DuplicateParticipant(participant));
        }

        self.participants.push(participant);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Record a turn from the current speaker.
    pub fn record_turn(&mut self, content: impl Into<String>) -> Result<ForumTurn, SessionError> {
        if self.status != SessionStatus::Active {
            return Err(SessionError::InvalidState {
                expected: SessionStatus::Active,
                actual: self.status,
            });
        }

        let speaker = self.participants[self.current_speaker_idx].clone();
        let turn = ForumTurn::new(self.current_round, &speaker, content);

        self.history.push(turn.clone());
        self.advance_speaker();
        self.updated_at = Utc::now();

        Ok(turn)
    }

    /// Advance to the next speaker, potentially starting a new round.
    fn advance_speaker(&mut self) {
        self.current_speaker_idx += 1;

        // If we've gone through all participants, start a new round
        if self.current_speaker_idx >= self.participants.len() {
            self.current_speaker_idx = 0;
            self.current_round += 1;

            // Check if we've exceeded max rounds
            if self.current_round > self.config.max_rounds {
                self.complete();
            }
        }
    }

    /// Get the next speaker (without advancing).
    pub fn next_speaker(&self) -> Option<&str> {
        if self.status != SessionStatus::Active {
            return None;
        }

        let mut next_idx = self.current_speaker_idx + 1;
        if next_idx >= self.participants.len() {
            // Check if another round would start
            if self.current_round < self.config.max_rounds {
                next_idx = 0;
            } else {
                return None;
            }
        }

        self.participants.get(next_idx).map(|s| s.as_str())
    }

    /// Complete the session.
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Cancel the session.
    pub fn cancel(&mut self) {
        self.status = SessionStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Mark the session as timed out.
    pub fn timeout(&mut self) {
        self.status = SessionStatus::TimedOut;
        self.updated_at = Utc::now();
    }

    /// Set the final summary.
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.summary = Some(summary.into());
        self.updated_at = Utc::now();
    }

    /// Check if the session is complete.
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed | SessionStatus::Cancelled | SessionStatus::TimedOut
        )
    }

    /// Get turns for a specific round.
    pub fn turns_for_round(&self, round: u32) -> Vec<&ForumTurn> {
        self.history.iter().filter(|t| t.round == round).collect()
    }

    /// Get turns by a specific speaker.
    pub fn turns_by_speaker(&self, speaker: &str) -> Vec<&ForumTurn> {
        self.history
            .iter()
            .filter(|t| t.speaker == speaker)
            .collect()
    }

    /// Get the total number of turns.
    pub fn turn_count(&self) -> usize {
        self.history.len()
    }
}

/// Errors that can occur during session operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    #[error("Invalid session state: expected {expected:?}, got {actual:?}")]
    InvalidState {
        expected: SessionStatus,
        actual: SessionStatus,
    },

    #[error("Not enough participants: required {required}, got {actual}")]
    NotEnoughParticipants { required: usize, actual: usize },

    #[error("Late join not allowed")]
    LateJoinNotAllowed,

    #[error("Duplicate participant: {0}")]
    DuplicateParticipant(String),

    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Turn timeout")]
    TurnTimeout,

    #[error("Round timeout")]
    RoundTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ForumSession::new(
            "test-1",
            "Should we use Rust or Go?",
            vec!["agent-a".to_string(), "agent-b".to_string()],
        );

        assert_eq!(session.id, "test-1");
        assert_eq!(session.topic, "Should we use Rust or Go?");
        assert_eq!(session.participants.len(), 2);
        assert_eq!(session.status, SessionStatus::Pending);
        assert_eq!(session.current_round, 0);
    }

    #[test]
    fn test_session_start() {
        let mut session = ForumSession::new(
            "test-1",
            "Test topic",
            vec!["a".to_string(), "b".to_string()],
        );

        assert!(session.start().is_ok());
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.current_round, 1);
        assert_eq!(session.current_speaker(), Some("a"));
    }

    #[test]
    fn test_session_start_not_enough_participants() {
        let mut session = ForumSession::new("test-1", "Test topic", vec!["a".to_string()]);

        let result = session.start();
        assert!(matches!(
            result,
            Err(SessionError::NotEnoughParticipants { required: 2, actual: 1 })
        ));
    }

    #[test]
    fn test_record_turn() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );
        session.start().unwrap();

        let turn = session.record_turn("First response").unwrap();
        assert_eq!(turn.speaker, "a");
        assert_eq!(turn.round, 1);
        assert_eq!(turn.content, "First response");

        assert_eq!(session.current_speaker(), Some("b"));
    }

    #[test]
    fn test_round_progression() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        )
        .with_config(SessionConfig {
            max_rounds: 2,
            ..Default::default()
        });

        session.start().unwrap();

        // Round 1
        session.record_turn("a-r1").unwrap();
        session.record_turn("b-r1").unwrap();

        // Should now be in round 2
        assert_eq!(session.current_round, 2);
        assert_eq!(session.current_speaker(), Some("a"));

        // Round 2
        session.record_turn("a-r2").unwrap();
        session.record_turn("b-r2").unwrap();

        // Should be complete
        assert!(session.is_complete());
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_turns_for_round() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );
        session.start().unwrap();

        session.record_turn("a-r1").unwrap();
        session.record_turn("b-r1").unwrap();
        session.record_turn("a-r2").unwrap();

        let round1_turns = session.turns_for_round(1);
        assert_eq!(round1_turns.len(), 2);
        assert_eq!(round1_turns[0].speaker, "a");
        assert_eq!(round1_turns[1].speaker, "b");

        let round2_turns = session.turns_for_round(2);
        assert_eq!(round2_turns.len(), 1);
        assert_eq!(round2_turns[0].speaker, "a");
    }

    #[test]
    fn test_turns_by_speaker() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );
        session.start().unwrap();

        session.record_turn("a-r1").unwrap();
        session.record_turn("b-r1").unwrap();
        session.record_turn("a-r2").unwrap();

        let a_turns = session.turns_by_speaker("a");
        assert_eq!(a_turns.len(), 2);
        assert_eq!(a_turns[0].content, "a-r1");
        assert_eq!(a_turns[1].content, "a-r2");
    }

    #[test]
    fn test_add_participant() {
        let mut session = ForumSession::new("test-1", "Test", vec!["a".to_string()])
            .with_config(SessionConfig {
                allow_late_join: true,
                min_participants: 1,
                ..Default::default()
            });

        session.start().unwrap();
        assert!(session.add_participant("b").is_ok());
        assert_eq!(session.participants.len(), 2);
    }

    #[test]
    fn test_add_participant_late_join_not_allowed() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );

        session.start().unwrap();
        let result = session.add_participant("c");
        assert!(matches!(result, Err(SessionError::LateJoinNotAllowed)));
    }

    #[test]
    fn test_duplicate_participant() {
        let mut session = ForumSession::new("test-1", "Test", vec!["a".to_string()])
            .with_config(SessionConfig {
                allow_late_join: true,
                min_participants: 1,
                ..Default::default()
            });

        let result = session.add_participant("a");
        assert!(matches!(result, Err(SessionError::DuplicateParticipant(_))));
    }

    #[test]
    fn test_session_cancel() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );
        session.start().unwrap();
        session.cancel();

        assert_eq!(session.status, SessionStatus::Cancelled);
        assert!(session.is_complete());
    }

    #[test]
    fn test_session_timeout() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );
        session.start().unwrap();
        session.timeout();

        assert_eq!(session.status, SessionStatus::TimedOut);
        assert!(session.is_complete());
    }

    #[test]
    fn test_set_summary() {
        let mut session = ForumSession::new(
            "test-1",
            "Test",
            vec!["a".to_string(), "b".to_string()],
        );

        session.set_summary("This is the summary");
        assert_eq!(session.summary, Some("This is the summary".to_string()));
    }
}
