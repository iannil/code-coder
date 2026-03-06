//! State machine for autonomous mode
//!
//! Provides type-safe state transitions with compile-time validation
//! where possible and runtime validation for dynamic transitions.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Autonomous execution states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousState {
    // Core states
    Idle,
    Planning,
    PlanApproved,
    Executing,
    Testing,
    Verifying,
    Deciding,
    DecisionMade,
    Fixing,
    Retrying,
    Evaluating,
    Scoring,
    Checkpointing,
    RollingBack,
    Continuing,
    Completed,
    Failed,
    Paused,
    Blocked,
    Terminated,

    // Expansion states (BookExpander)
    ExpansionIdle,
    ExpansionAnalyzing,
    ExpansionAnalysisComplete,
    ExpansionBuilding,
    ExpansionFrameworkComplete,
    ExpansionOutlining,
    ExpansionOutlineComplete,
    ExpansionWriting,
    ExpansionChapterComplete,
    ExpansionWritingComplete,
    ExpansionValidating,
    ExpansionValidationComplete,
    ExpansionComplete,
    ExpansionFailed,
    ExpansionPaused,
}

impl AutonomousState {
    /// Get all valid transitions from this state
    pub fn valid_transitions(&self) -> &'static [AutonomousState] {
        VALID_TRANSITIONS.get(self).copied().unwrap_or(&[])
    }

    /// Check if transition to target state is valid
    pub fn can_transition_to(&self, target: AutonomousState) -> bool {
        self.valid_transitions().contains(&target)
    }

    /// Get the category of this state
    pub fn category(&self) -> StateCategory {
        match self {
            Self::Idle | Self::ExpansionIdle => StateCategory::Initial,

            Self::Planning
            | Self::PlanApproved
            | Self::Executing
            | Self::Testing
            | Self::Verifying
            | Self::Deciding
            | Self::DecisionMade
            | Self::Fixing
            | Self::Retrying
            | Self::Evaluating
            | Self::Scoring
            | Self::Continuing
            | Self::ExpansionAnalyzing
            | Self::ExpansionAnalysisComplete
            | Self::ExpansionBuilding
            | Self::ExpansionFrameworkComplete
            | Self::ExpansionOutlining
            | Self::ExpansionOutlineComplete
            | Self::ExpansionWriting
            | Self::ExpansionChapterComplete
            | Self::ExpansionWritingComplete
            | Self::ExpansionValidating
            | Self::ExpansionValidationComplete => StateCategory::Active,

            Self::Completed
            | Self::Failed
            | Self::Paused
            | Self::Blocked
            | Self::Terminated
            | Self::ExpansionComplete
            | Self::ExpansionFailed
            | Self::ExpansionPaused => StateCategory::Terminal,

            Self::Checkpointing | Self::RollingBack => StateCategory::Recovery,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Paused
                | Self::Blocked
                | Self::Terminated
                | Self::ExpansionComplete
                | Self::ExpansionFailed
                | Self::ExpansionPaused
        )
    }

    /// Check if this state is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::Paused | Self::Blocked)
    }
}

impl std::fmt::Display for AutonomousState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::PlanApproved => "plan_approved",
            Self::Executing => "executing",
            Self::Testing => "testing",
            Self::Verifying => "verifying",
            Self::Deciding => "deciding",
            Self::DecisionMade => "decision_made",
            Self::Fixing => "fixing",
            Self::Retrying => "retrying",
            Self::Evaluating => "evaluating",
            Self::Scoring => "scoring",
            Self::Checkpointing => "checkpointing",
            Self::RollingBack => "rolling_back",
            Self::Continuing => "continuing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Paused => "paused",
            Self::Blocked => "blocked",
            Self::Terminated => "terminated",
            Self::ExpansionIdle => "expansion_idle",
            Self::ExpansionAnalyzing => "expansion_analyzing",
            Self::ExpansionAnalysisComplete => "expansion_analysis_complete",
            Self::ExpansionBuilding => "expansion_building",
            Self::ExpansionFrameworkComplete => "expansion_framework_complete",
            Self::ExpansionOutlining => "expansion_outlining",
            Self::ExpansionOutlineComplete => "expansion_outline_complete",
            Self::ExpansionWriting => "expansion_writing",
            Self::ExpansionChapterComplete => "expansion_chapter_complete",
            Self::ExpansionWritingComplete => "expansion_writing_complete",
            Self::ExpansionValidating => "expansion_validating",
            Self::ExpansionValidationComplete => "expansion_validation_complete",
            Self::ExpansionComplete => "expansion_complete",
            Self::ExpansionFailed => "expansion_failed",
            Self::ExpansionPaused => "expansion_paused",
        };
        write!(f, "{}", s)
    }
}

/// State category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateCategory {
    Initial,
    Active,
    Terminal,
    Recovery,
}

/// Valid state transitions lookup table
pub static VALID_TRANSITIONS: once_cell::sync::Lazy<
    std::collections::HashMap<AutonomousState, &'static [AutonomousState]>,
> = once_cell::sync::Lazy::new(|| {
    use AutonomousState::*;
    let mut m = std::collections::HashMap::new();

    m.insert(Idle, &[Planning, Terminated, ExpansionAnalyzing][..]);
    m.insert(Planning, &[PlanApproved, Deciding, Failed, Paused][..]);
    m.insert(PlanApproved, &[Executing, Deciding, Paused][..]);
    m.insert(Executing, &[Testing, Deciding, Checkpointing, Fixing, Failed, Paused][..]);
    m.insert(Testing, &[Verifying, Fixing, Deciding, Retrying, Failed][..]);
    m.insert(Verifying, &[Evaluating, Fixing, Deciding, Retrying, Failed][..]);
    m.insert(Deciding, &[DecisionMade, Paused, Blocked][..]);
    m.insert(DecisionMade, &[Executing, Planning, Failed, Paused][..]);
    m.insert(Fixing, &[Testing, Executing, Deciding, Failed][..]);
    m.insert(Retrying, &[Planning, Executing, Failed][..]);
    m.insert(Evaluating, &[Scoring, Deciding, Failed][..]);
    m.insert(Scoring, &[Completed, Continuing, Failed, Paused][..]);
    m.insert(Checkpointing, &[Executing, Testing, Failed][..]);
    m.insert(RollingBack, &[Executing, Planning, Failed, Paused][..]);
    m.insert(Continuing, &[Planning, Executing, Paused, Completed][..]);
    m.insert(Completed, &[Idle, Terminated][..]);
    m.insert(Failed, &[Idle, Planning, Terminated][..]);
    m.insert(Paused, &[Executing, Planning, Deciding, Terminated][..]);
    m.insert(Blocked, &[Deciding, Paused, Failed, Terminated][..]);
    m.insert(Terminated, &[][..]);

    // Expansion states
    m.insert(ExpansionIdle, &[ExpansionAnalyzing, ExpansionFailed][..]);
    m.insert(ExpansionAnalyzing, &[ExpansionAnalysisComplete, ExpansionFailed][..]);
    m.insert(ExpansionAnalysisComplete, &[ExpansionBuilding, ExpansionPaused, ExpansionFailed][..]);
    m.insert(ExpansionBuilding, &[ExpansionFrameworkComplete, ExpansionPaused, ExpansionFailed][..]);
    m.insert(ExpansionFrameworkComplete, &[ExpansionOutlining, ExpansionPaused][..]);
    m.insert(ExpansionOutlining, &[ExpansionOutlineComplete, ExpansionFailed][..]);
    m.insert(ExpansionOutlineComplete, &[ExpansionWriting, ExpansionPaused][..]);
    m.insert(ExpansionWriting, &[ExpansionChapterComplete, ExpansionWritingComplete, ExpansionPaused, ExpansionFailed][..]);
    m.insert(ExpansionChapterComplete, &[ExpansionWriting, ExpansionValidating, ExpansionPaused][..]);
    m.insert(ExpansionWritingComplete, &[ExpansionValidating, ExpansionComplete][..]);
    m.insert(ExpansionValidating, &[ExpansionValidationComplete, ExpansionWriting, ExpansionFailed][..]);
    m.insert(ExpansionValidationComplete, &[ExpansionComplete, ExpansionWriting][..]);
    m.insert(ExpansionComplete, &[ExpansionIdle, Terminated][..]);
    m.insert(ExpansionFailed, &[ExpansionIdle, Terminated][..]);
    m.insert(ExpansionPaused, &[
        ExpansionAnalyzing, ExpansionBuilding, ExpansionOutlining,
        ExpansionWriting, ExpansionValidating, ExpansionComplete,
        ExpansionFailed, Terminated
    ][..]);

    m
});

/// State metadata for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub state: AutonomousState,
    pub entered_at: i64,
    pub previous_state: Option<AutonomousState>,
    pub reason: Option<String>,
}

/// Result of a state transition
#[derive(Debug, Clone)]
pub enum TransitionResult {
    Success {
        from: AutonomousState,
        to: AutonomousState,
        metadata: StateMetadata,
    },
    InvalidTransition {
        from: AutonomousState,
        to: AutonomousState,
    },
}

impl TransitionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// State machine configuration
#[derive(Debug, Clone)]
pub struct StateMachineConfig {
    pub max_history: usize,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self { max_history: 100 }
    }
}

/// State machine for autonomous mode
#[derive(Debug, Clone)]
pub struct StateMachine {
    current_state: AutonomousState,
    history: VecDeque<StateMetadata>,
    config: StateMachineConfig,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new(StateMachineConfig::default())
    }
}

impl StateMachine {
    /// Create a new state machine with configuration
    pub fn new(config: StateMachineConfig) -> Self {
        Self {
            current_state: AutonomousState::Idle,
            history: VecDeque::with_capacity(config.max_history),
            config,
        }
    }

    /// Get the current state
    pub fn state(&self) -> AutonomousState {
        self.current_state
    }

    /// Check if in a specific state
    pub fn is(&self, state: AutonomousState) -> bool {
        self.current_state == state
    }

    /// Check if in any of the given states
    pub fn is_in(&self, states: &[AutonomousState]) -> bool {
        states.contains(&self.current_state)
    }

    /// Get state history
    pub fn history(&self) -> &VecDeque<StateMetadata> {
        &self.history
    }

    /// Get the previous state
    pub fn previous_state(&self) -> Option<AutonomousState> {
        if self.history.len() < 2 {
            return None;
        }
        self.history.get(self.history.len() - 2).map(|m| m.state)
    }

    /// Count visits to a state
    pub fn state_visit_count(&self, state: AutonomousState) -> usize {
        self.history.iter().filter(|m| m.state == state).count()
    }

    /// Detect if we're in a loop
    pub fn detect_loop(&self, state: AutonomousState, threshold: usize) -> bool {
        let recent_count = threshold * 2;
        let recent: Vec<_> = self.history.iter().rev().take(recent_count).collect();
        recent.iter().filter(|m| m.state == state).count() >= threshold
    }

    /// Attempt a state transition
    pub fn transition(&mut self, to: AutonomousState, reason: Option<String>) -> TransitionResult {
        let from = self.current_state;

        // Validate transition
        if !from.can_transition_to(to) {
            return TransitionResult::InvalidTransition { from, to };
        }

        self.do_transition(from, to, reason)
    }

    /// Force a transition (bypasses validation)
    pub fn force_transition(&mut self, to: AutonomousState, reason: Option<String>) -> TransitionResult {
        let from = self.current_state;
        self.do_transition(from, to, reason)
    }

    fn do_transition(
        &mut self,
        from: AutonomousState,
        to: AutonomousState,
        reason: Option<String>,
    ) -> TransitionResult {
        let metadata = StateMetadata {
            state: to,
            entered_at: chrono::Utc::now().timestamp_millis(),
            previous_state: Some(from),
            reason,
        };

        // Update history
        self.history.push_back(metadata.clone());
        if self.history.len() > self.config.max_history {
            self.history.pop_front();
        }

        // Update current state
        self.current_state = to;

        TransitionResult::Success { from, to, metadata }
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.current_state = AutonomousState::Idle;
        self.history.clear();
    }

    /// Get time spent in current state (ms)
    pub fn time_in_current_state(&self) -> i64 {
        match self.history.back() {
            Some(metadata) => chrono::Utc::now().timestamp_millis() - metadata.entered_at,
            None => 0,
        }
    }

    /// Get total time spent in a specific state (ms)
    pub fn total_time_in_state(&self, state: AutonomousState) -> i64 {
        let mut total = 0i64;
        let history: Vec<_> = self.history.iter().collect();

        for i in 0..history.len() {
            if history[i].state != state {
                continue;
            }

            let entered = history[i].entered_at;
            let exited = if i + 1 < history.len() {
                history[i + 1].entered_at
            } else {
                chrono::Utc::now().timestamp_millis()
            };

            total += exited - entered;
        }

        total
    }

    /// Serialize the state machine
    pub fn serialize(&self) -> StateMachineSnapshot {
        StateMachineSnapshot {
            current_state: self.current_state,
            history: self.history.iter().cloned().collect(),
        }
    }

    /// Deserialize from snapshot
    pub fn deserialize(snapshot: StateMachineSnapshot, config: StateMachineConfig) -> Self {
        let mut history = VecDeque::with_capacity(config.max_history);
        for meta in snapshot.history {
            history.push_back(meta);
        }

        Self {
            current_state: snapshot.current_state,
            history,
            config,
        }
    }
}

/// Serializable snapshot of state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineSnapshot {
    pub current_state: AutonomousState,
    pub history: Vec<StateMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = StateMachine::default();
        assert_eq!(sm.state(), AutonomousState::Idle);
    }

    #[test]
    fn test_valid_transition() {
        let mut sm = StateMachine::default();

        let result = sm.transition(AutonomousState::Planning, Some("Start planning".into()));
        assert!(result.is_success());
        assert_eq!(sm.state(), AutonomousState::Planning);
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = StateMachine::default();

        // Can't go from Idle to Executing directly
        let result = sm.transition(AutonomousState::Executing, None);
        assert!(!result.is_success());
        assert_eq!(sm.state(), AutonomousState::Idle);
    }

    #[test]
    fn test_history_tracking() {
        let mut sm = StateMachine::default();

        sm.transition(AutonomousState::Planning, None);
        sm.transition(AutonomousState::PlanApproved, None);
        sm.transition(AutonomousState::Executing, None);

        assert_eq!(sm.history().len(), 3);
        assert_eq!(sm.previous_state(), Some(AutonomousState::PlanApproved));
    }

    #[test]
    fn test_state_categories() {
        assert_eq!(AutonomousState::Idle.category(), StateCategory::Initial);
        assert_eq!(AutonomousState::Executing.category(), StateCategory::Active);
        assert_eq!(AutonomousState::Completed.category(), StateCategory::Terminal);
        assert_eq!(AutonomousState::RollingBack.category(), StateCategory::Recovery);
    }

    #[test]
    fn test_is_terminal() {
        assert!(!AutonomousState::Idle.is_terminal());
        assert!(!AutonomousState::Executing.is_terminal());
        assert!(AutonomousState::Completed.is_terminal());
        assert!(AutonomousState::Failed.is_terminal());
    }

    #[test]
    fn test_force_transition() {
        let mut sm = StateMachine::default();

        // Force invalid transition
        let result = sm.force_transition(AutonomousState::Executing, Some("Forced".into()));
        assert!(result.is_success());
        assert_eq!(sm.state(), AutonomousState::Executing);
    }

    #[test]
    fn test_reset() {
        let mut sm = StateMachine::default();

        sm.transition(AutonomousState::Planning, None);
        sm.transition(AutonomousState::PlanApproved, None);
        sm.reset();

        assert_eq!(sm.state(), AutonomousState::Idle);
        assert!(sm.history().is_empty());
    }

    #[test]
    fn test_loop_detection() {
        let mut sm = StateMachine::default();

        // Simulate a loop: Planning -> PlanApproved -> Executing -> Testing -> Fixing -> Executing
        sm.force_transition(AutonomousState::Planning, None);
        sm.force_transition(AutonomousState::Executing, None);
        sm.force_transition(AutonomousState::Fixing, None);
        sm.force_transition(AutonomousState::Executing, None);
        sm.force_transition(AutonomousState::Fixing, None);
        sm.force_transition(AutonomousState::Executing, None);

        assert!(sm.detect_loop(AutonomousState::Executing, 3));
    }

    #[test]
    fn test_serialization() {
        let mut sm = StateMachine::default();
        sm.transition(AutonomousState::Planning, Some("Test".into()));

        let snapshot = sm.serialize();
        let restored = StateMachine::deserialize(snapshot, StateMachineConfig::default());

        assert_eq!(restored.state(), AutonomousState::Planning);
        assert_eq!(restored.history().len(), 1);
    }
}
