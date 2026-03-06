//! NAPI bindings for autonomous module
//!
//! Exposes state machine and task queue functionality to Node.js/TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::autonomous::{
    // State machine types
    AutonomousState as RustAutonomousState, StateCategory as RustStateCategory,
    StateMachine as RustStateMachine, StateMachineConfig as RustStateMachineConfig,
    StateMetadata as RustStateMetadata, TransitionResult as RustTransitionResult,
    // Task queue types
    Task as RustTask, TaskPriority as RustTaskPriority, TaskQueue as RustTaskQueue,
    TaskQueueConfig as RustTaskQueueConfig, TaskQueueStats as RustTaskQueueStats,
    TaskStatus as RustTaskStatus,
};

// ============================================================================
// State Machine Types (NAPI)
// ============================================================================

/// Autonomous state representation for JS
#[napi(string_enum)]
pub enum AutonomousState {
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
    // Expansion states
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

impl From<RustAutonomousState> for AutonomousState {
    fn from(s: RustAutonomousState) -> Self {
        match s {
            RustAutonomousState::Idle => AutonomousState::Idle,
            RustAutonomousState::Planning => AutonomousState::Planning,
            RustAutonomousState::PlanApproved => AutonomousState::PlanApproved,
            RustAutonomousState::Executing => AutonomousState::Executing,
            RustAutonomousState::Testing => AutonomousState::Testing,
            RustAutonomousState::Verifying => AutonomousState::Verifying,
            RustAutonomousState::Deciding => AutonomousState::Deciding,
            RustAutonomousState::DecisionMade => AutonomousState::DecisionMade,
            RustAutonomousState::Fixing => AutonomousState::Fixing,
            RustAutonomousState::Retrying => AutonomousState::Retrying,
            RustAutonomousState::Evaluating => AutonomousState::Evaluating,
            RustAutonomousState::Scoring => AutonomousState::Scoring,
            RustAutonomousState::Checkpointing => AutonomousState::Checkpointing,
            RustAutonomousState::RollingBack => AutonomousState::RollingBack,
            RustAutonomousState::Continuing => AutonomousState::Continuing,
            RustAutonomousState::Completed => AutonomousState::Completed,
            RustAutonomousState::Failed => AutonomousState::Failed,
            RustAutonomousState::Paused => AutonomousState::Paused,
            RustAutonomousState::Blocked => AutonomousState::Blocked,
            RustAutonomousState::Terminated => AutonomousState::Terminated,
            RustAutonomousState::ExpansionIdle => AutonomousState::ExpansionIdle,
            RustAutonomousState::ExpansionAnalyzing => AutonomousState::ExpansionAnalyzing,
            RustAutonomousState::ExpansionAnalysisComplete => {
                AutonomousState::ExpansionAnalysisComplete
            }
            RustAutonomousState::ExpansionBuilding => AutonomousState::ExpansionBuilding,
            RustAutonomousState::ExpansionFrameworkComplete => {
                AutonomousState::ExpansionFrameworkComplete
            }
            RustAutonomousState::ExpansionOutlining => AutonomousState::ExpansionOutlining,
            RustAutonomousState::ExpansionOutlineComplete => {
                AutonomousState::ExpansionOutlineComplete
            }
            RustAutonomousState::ExpansionWriting => AutonomousState::ExpansionWriting,
            RustAutonomousState::ExpansionChapterComplete => {
                AutonomousState::ExpansionChapterComplete
            }
            RustAutonomousState::ExpansionWritingComplete => {
                AutonomousState::ExpansionWritingComplete
            }
            RustAutonomousState::ExpansionValidating => AutonomousState::ExpansionValidating,
            RustAutonomousState::ExpansionValidationComplete => {
                AutonomousState::ExpansionValidationComplete
            }
            RustAutonomousState::ExpansionComplete => AutonomousState::ExpansionComplete,
            RustAutonomousState::ExpansionFailed => AutonomousState::ExpansionFailed,
            RustAutonomousState::ExpansionPaused => AutonomousState::ExpansionPaused,
        }
    }
}

impl From<AutonomousState> for RustAutonomousState {
    fn from(s: AutonomousState) -> Self {
        match s {
            AutonomousState::Idle => RustAutonomousState::Idle,
            AutonomousState::Planning => RustAutonomousState::Planning,
            AutonomousState::PlanApproved => RustAutonomousState::PlanApproved,
            AutonomousState::Executing => RustAutonomousState::Executing,
            AutonomousState::Testing => RustAutonomousState::Testing,
            AutonomousState::Verifying => RustAutonomousState::Verifying,
            AutonomousState::Deciding => RustAutonomousState::Deciding,
            AutonomousState::DecisionMade => RustAutonomousState::DecisionMade,
            AutonomousState::Fixing => RustAutonomousState::Fixing,
            AutonomousState::Retrying => RustAutonomousState::Retrying,
            AutonomousState::Evaluating => RustAutonomousState::Evaluating,
            AutonomousState::Scoring => RustAutonomousState::Scoring,
            AutonomousState::Checkpointing => RustAutonomousState::Checkpointing,
            AutonomousState::RollingBack => RustAutonomousState::RollingBack,
            AutonomousState::Continuing => RustAutonomousState::Continuing,
            AutonomousState::Completed => RustAutonomousState::Completed,
            AutonomousState::Failed => RustAutonomousState::Failed,
            AutonomousState::Paused => RustAutonomousState::Paused,
            AutonomousState::Blocked => RustAutonomousState::Blocked,
            AutonomousState::Terminated => RustAutonomousState::Terminated,
            AutonomousState::ExpansionIdle => RustAutonomousState::ExpansionIdle,
            AutonomousState::ExpansionAnalyzing => RustAutonomousState::ExpansionAnalyzing,
            AutonomousState::ExpansionAnalysisComplete => {
                RustAutonomousState::ExpansionAnalysisComplete
            }
            AutonomousState::ExpansionBuilding => RustAutonomousState::ExpansionBuilding,
            AutonomousState::ExpansionFrameworkComplete => {
                RustAutonomousState::ExpansionFrameworkComplete
            }
            AutonomousState::ExpansionOutlining => RustAutonomousState::ExpansionOutlining,
            AutonomousState::ExpansionOutlineComplete => {
                RustAutonomousState::ExpansionOutlineComplete
            }
            AutonomousState::ExpansionWriting => RustAutonomousState::ExpansionWriting,
            AutonomousState::ExpansionChapterComplete => {
                RustAutonomousState::ExpansionChapterComplete
            }
            AutonomousState::ExpansionWritingComplete => {
                RustAutonomousState::ExpansionWritingComplete
            }
            AutonomousState::ExpansionValidating => RustAutonomousState::ExpansionValidating,
            AutonomousState::ExpansionValidationComplete => {
                RustAutonomousState::ExpansionValidationComplete
            }
            AutonomousState::ExpansionComplete => RustAutonomousState::ExpansionComplete,
            AutonomousState::ExpansionFailed => RustAutonomousState::ExpansionFailed,
            AutonomousState::ExpansionPaused => RustAutonomousState::ExpansionPaused,
        }
    }
}

/// State category
#[napi(string_enum)]
pub enum StateCategory {
    Initial,
    Active,
    Terminal,
    Recovery,
}

impl From<RustStateCategory> for StateCategory {
    fn from(c: RustStateCategory) -> Self {
        match c {
            RustStateCategory::Initial => StateCategory::Initial,
            RustStateCategory::Active => StateCategory::Active,
            RustStateCategory::Terminal => StateCategory::Terminal,
            RustStateCategory::Recovery => StateCategory::Recovery,
        }
    }
}

/// State metadata for history tracking
#[napi(object)]
pub struct NapiStateMetadata {
    pub state: String,
    pub entered_at: i64,
    pub previous_state: Option<String>,
    pub reason: Option<String>,
}

impl From<RustStateMetadata> for NapiStateMetadata {
    fn from(m: RustStateMetadata) -> Self {
        Self {
            state: m.state.to_string(),
            entered_at: m.entered_at,
            previous_state: m.previous_state.map(|s| s.to_string()),
            reason: m.reason,
        }
    }
}

/// Transition result
#[napi(object)]
pub struct NapiTransitionResult {
    pub success: bool,
    pub from_state: String,
    pub to_state: String,
    pub metadata: Option<NapiStateMetadata>,
    pub error: Option<String>,
}

/// State machine configuration
#[napi(object)]
pub struct NapiStateMachineConfig {
    pub max_history: Option<u32>,
}

impl From<NapiStateMachineConfig> for RustStateMachineConfig {
    fn from(c: NapiStateMachineConfig) -> Self {
        Self {
            max_history: c.max_history.unwrap_or(100) as usize,
        }
    }
}

// ============================================================================
// Task Queue Types (NAPI)
// ============================================================================

/// Task priority
#[napi(string_enum)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl From<RustTaskPriority> for TaskPriority {
    fn from(p: RustTaskPriority) -> Self {
        match p {
            RustTaskPriority::Critical => TaskPriority::Critical,
            RustTaskPriority::High => TaskPriority::High,
            RustTaskPriority::Medium => TaskPriority::Medium,
            RustTaskPriority::Low => TaskPriority::Low,
        }
    }
}

impl From<TaskPriority> for RustTaskPriority {
    fn from(p: TaskPriority) -> Self {
        match p {
            TaskPriority::Critical => RustTaskPriority::Critical,
            TaskPriority::High => RustTaskPriority::High,
            TaskPriority::Medium => RustTaskPriority::Medium,
            TaskPriority::Low => RustTaskPriority::Low,
        }
    }
}

/// Task status
#[napi(string_enum)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Blocked,
}

impl From<RustTaskStatus> for TaskStatus {
    fn from(s: RustTaskStatus) -> Self {
        match s {
            RustTaskStatus::Pending => TaskStatus::Pending,
            RustTaskStatus::Running => TaskStatus::Running,
            RustTaskStatus::Completed => TaskStatus::Completed,
            RustTaskStatus::Failed => TaskStatus::Failed,
            RustTaskStatus::Skipped => TaskStatus::Skipped,
            RustTaskStatus::Blocked => TaskStatus::Blocked,
        }
    }
}

/// Task definition for NAPI
#[napi(object)]
pub struct NapiTask {
    pub id: String,
    pub session_id: String,
    pub subject: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub agent: Option<String>,
}

impl From<&RustTask> for NapiTask {
    fn from(t: &RustTask) -> Self {
        Self {
            id: t.id.clone(),
            session_id: t.session_id.clone(),
            subject: t.subject.clone(),
            description: t.description.clone(),
            status: format!("{:?}", t.status).to_lowercase(),
            priority: format!("{:?}", t.priority).to_lowercase(),
            dependencies: t.dependencies.clone(),
            dependents: t.dependents.clone(),
            created_at: t.created_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
            error: t.error.clone(),
            retry_count: t.retry_count,
            max_retries: t.max_retries,
            agent: t.agent.clone(),
        }
    }
}

/// Task queue statistics
#[napi(object)]
pub struct NapiTaskQueueStats {
    pub total: u32,
    pub pending: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub blocked: u32,
}

impl From<RustTaskQueueStats> for NapiTaskQueueStats {
    fn from(s: RustTaskQueueStats) -> Self {
        Self {
            total: s.total as u32,
            pending: s.pending as u32,
            running: s.running as u32,
            completed: s.completed as u32,
            failed: s.failed as u32,
            skipped: s.skipped as u32,
            blocked: s.blocked as u32,
        }
    }
}

/// Task queue configuration
#[napi(object)]
pub struct NapiTaskQueueConfig {
    pub max_concurrent: Option<u32>,
    pub max_retries: Option<u32>,
    pub retry_delay_ms: Option<u32>,
}

impl From<NapiTaskQueueConfig> for RustTaskQueueConfig {
    fn from(c: NapiTaskQueueConfig) -> Self {
        Self {
            max_concurrent: c.max_concurrent.unwrap_or(3) as usize,
            max_retries: c.max_retries.unwrap_or(2),
            retry_delay_ms: c.retry_delay_ms.unwrap_or(1000) as u64,
        }
    }
}

// ============================================================================
// State Machine Handle
// ============================================================================

/// Handle to a state machine
#[napi]
pub struct StateMachineHandle {
    inner: Arc<Mutex<RustStateMachine>>,
}

/// Create a new state machine
#[napi]
pub fn create_state_machine(config: Option<NapiStateMachineConfig>) -> StateMachineHandle {
    let rust_config = config.map(Into::into).unwrap_or_default();
    StateMachineHandle {
        inner: Arc::new(Mutex::new(RustStateMachine::new(rust_config))),
    }
}

#[napi]
impl StateMachineHandle {
    /// Get the current state
    #[napi]
    pub fn state(&self) -> String {
        let sm = self.inner.lock().unwrap();
        sm.state().to_string()
    }

    /// Get state category
    #[napi]
    pub fn category(&self) -> StateCategory {
        let sm = self.inner.lock().unwrap();
        sm.state().category().into()
    }

    /// Check if in a specific state
    #[napi]
    pub fn is_state(&self, state: AutonomousState) -> bool {
        let sm = self.inner.lock().unwrap();
        sm.is(state.into())
    }

    /// Check if state is terminal
    #[napi]
    pub fn is_terminal(&self) -> bool {
        let sm = self.inner.lock().unwrap();
        sm.state().is_terminal()
    }

    /// Check if state is recoverable
    #[napi]
    pub fn is_recoverable(&self) -> bool {
        let sm = self.inner.lock().unwrap();
        sm.state().is_recoverable()
    }

    /// Get valid transitions from current state
    #[napi]
    pub fn valid_transitions(&self) -> Vec<String> {
        let sm = self.inner.lock().unwrap();
        sm.state()
            .valid_transitions()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if transition is valid
    #[napi]
    pub fn can_transition_to(&self, target: AutonomousState) -> bool {
        let sm = self.inner.lock().unwrap();
        sm.state().can_transition_to(target.into())
    }

    /// Attempt a state transition
    #[napi]
    pub fn transition(
        &self,
        to: AutonomousState,
        reason: Option<String>,
    ) -> NapiTransitionResult {
        let mut sm = self.inner.lock().unwrap();
        let to_rust: RustAutonomousState = to.into();

        match sm.transition(to_rust, reason) {
            RustTransitionResult::Success { from, to, metadata } => {
                NapiTransitionResult {
                    success: true,
                    from_state: from.to_string(),
                    to_state: to.to_string(),
                    metadata: Some(metadata.into()),
                    error: None,
                }
            }
            RustTransitionResult::InvalidTransition { from, to } => {
                NapiTransitionResult {
                    success: false,
                    from_state: from.to_string(),
                    to_state: to.to_string(),
                    metadata: None,
                    error: Some(format!(
                        "Invalid transition from {} to {}",
                        from, to
                    )),
                }
            }
        }
    }

    /// Force a transition (bypasses validation)
    #[napi]
    pub fn force_transition(
        &self,
        to: AutonomousState,
        reason: Option<String>,
    ) -> NapiTransitionResult {
        let mut sm = self.inner.lock().unwrap();
        let to_rust: RustAutonomousState = to.into();

        match sm.force_transition(to_rust, reason) {
            RustTransitionResult::Success { from, to, metadata } => {
                NapiTransitionResult {
                    success: true,
                    from_state: from.to_string(),
                    to_state: to.to_string(),
                    metadata: Some(metadata.into()),
                    error: None,
                }
            }
            RustTransitionResult::InvalidTransition { from, to } => {
                NapiTransitionResult {
                    success: false,
                    from_state: from.to_string(),
                    to_state: to.to_string(),
                    metadata: None,
                    error: Some(format!(
                        "Invalid transition from {} to {}",
                        from, to
                    )),
                }
            }
        }
    }

    /// Get state history
    #[napi]
    pub fn history(&self) -> Vec<NapiStateMetadata> {
        let sm = self.inner.lock().unwrap();
        sm.history().iter().cloned().map(Into::into).collect()
    }

    /// Get previous state
    #[napi]
    pub fn previous_state(&self) -> Option<String> {
        let sm = self.inner.lock().unwrap();
        sm.previous_state().map(|s| s.to_string())
    }

    /// Get visit count for a state
    #[napi]
    pub fn state_visit_count(&self, state: AutonomousState) -> u32 {
        let sm = self.inner.lock().unwrap();
        sm.state_visit_count(state.into()) as u32
    }

    /// Detect if we're in a loop
    #[napi]
    pub fn detect_loop(&self, state: AutonomousState, threshold: u32) -> bool {
        let sm = self.inner.lock().unwrap();
        sm.detect_loop(state.into(), threshold as usize)
    }

    /// Reset to initial state
    #[napi]
    pub fn reset(&self) {
        let mut sm = self.inner.lock().unwrap();
        sm.reset();
    }

    /// Get time spent in current state (ms)
    #[napi]
    pub fn time_in_current_state(&self) -> i64 {
        let sm = self.inner.lock().unwrap();
        sm.time_in_current_state()
    }

    /// Get total time spent in a specific state (ms)
    #[napi]
    pub fn total_time_in_state(&self, state: AutonomousState) -> i64 {
        let sm = self.inner.lock().unwrap();
        sm.total_time_in_state(state.into())
    }

    /// Serialize state machine to JSON
    #[napi]
    pub fn serialize(&self) -> String {
        let sm = self.inner.lock().unwrap();
        let snapshot = sm.serialize();
        serde_json::to_string(&snapshot).unwrap_or_default()
    }
}

// ============================================================================
// Task Queue Handle
// ============================================================================

/// Handle to a task queue
#[napi]
pub struct TaskQueueHandle {
    inner: Arc<Mutex<RustTaskQueue>>,
}

/// Create a new task queue
#[napi]
pub fn create_task_queue(
    session_id: String,
    config: Option<NapiTaskQueueConfig>,
) -> TaskQueueHandle {
    let rust_config = config.map(Into::into).unwrap_or_default();
    TaskQueueHandle {
        inner: Arc::new(Mutex::new(RustTaskQueue::new(session_id, rust_config))),
    }
}

#[napi]
impl TaskQueueHandle {
    /// Add a task to the queue
    #[napi]
    pub fn add_task(
        &self,
        subject: String,
        description: String,
        priority: TaskPriority,
    ) -> String {
        let mut queue = self.inner.lock().unwrap();
        queue.add_task(subject, description, priority.into())
    }

    /// Add task with dependencies
    #[napi]
    pub fn add_task_with_deps(
        &self,
        subject: String,
        description: String,
        priority: TaskPriority,
        dependencies: Vec<String>,
    ) -> String {
        let mut queue = self.inner.lock().unwrap();
        queue.add_with_deps(subject, description, priority.into(), dependencies)
    }

    /// Get a task by ID
    #[napi]
    pub fn get_task(&self, id: String) -> Option<NapiTask> {
        let queue = self.inner.lock().unwrap();
        queue.get(&id).map(|t| t.into())
    }

    /// Get all tasks
    #[napi]
    pub fn all_tasks(&self) -> Vec<NapiTask> {
        let queue = self.inner.lock().unwrap();
        queue.all().into_iter().map(|t| t.into()).collect()
    }

    /// Get runnable tasks (pending with satisfied dependencies)
    #[napi]
    pub fn runnable_tasks(&self) -> Vec<NapiTask> {
        let queue = self.inner.lock().unwrap();
        queue.runnable().into_iter().map(|t| t.into()).collect()
    }

    /// Start a task
    #[napi]
    pub fn start_task(&self, id: String) -> Result<()> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .start(&id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Complete a task successfully
    #[napi]
    pub fn complete_task(&self, id: String) -> Result<i64> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .complete(&id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Fail a task
    #[napi]
    pub fn fail_task(&self, id: String, error: String, retryable: bool) -> Result<bool> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .fail(&id, error, retryable)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Skip a task
    #[napi]
    pub fn skip_task(&self, id: String, reason: Option<String>) -> Result<()> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .skip(&id, reason)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Block a task
    #[napi]
    pub fn block_task(&self, id: String, reason: Option<String>) -> Result<()> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .block(&id, reason)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Unblock a task
    #[napi]
    pub fn unblock_task(&self, id: String) -> Result<()> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .unblock(&id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Retry a failed task
    #[napi]
    pub fn retry_task(&self, id: String) -> Result<()> {
        let mut queue = self.inner.lock().unwrap();
        queue
            .retry(&id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get queue statistics
    #[napi]
    pub fn stats(&self) -> NapiTaskQueueStats {
        let queue = self.inner.lock().unwrap();
        queue.stats().into()
    }

    /// Check if all tasks are complete
    #[napi]
    pub fn is_complete(&self) -> bool {
        let queue = self.inner.lock().unwrap();
        queue.is_complete()
    }

    /// Check if queue has failures
    #[napi]
    pub fn has_failures(&self) -> bool {
        let queue = self.inner.lock().unwrap();
        queue.has_failures()
    }

    /// Get failed tasks
    #[napi]
    pub fn failed_tasks(&self) -> Vec<NapiTask> {
        let queue = self.inner.lock().unwrap();
        queue.failed().into_iter().map(|t| t.into()).collect()
    }

    /// Get task chain (all dependencies and dependents)
    #[napi]
    pub fn task_chain(&self, id: String) -> Vec<NapiTask> {
        let queue = self.inner.lock().unwrap();
        queue.chain(&id).into_iter().map(|t| t.into()).collect()
    }

    /// Clear all tasks
    #[napi]
    pub fn clear(&self) {
        let mut queue = self.inner.lock().unwrap();
        queue.clear();
    }

    /// Get session ID
    #[napi]
    pub fn session_id(&self) -> String {
        let queue = self.inner.lock().unwrap();
        queue.session_id().to_string()
    }

    /// Serialize queue to JSON
    #[napi]
    pub fn serialize(&self) -> String {
        let queue = self.inner.lock().unwrap();
        queue.serialize().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_handle() {
        let sm = create_state_machine(None);
        assert_eq!(sm.state(), "idle");
        assert!(!sm.is_terminal());

        let result = sm.transition(AutonomousState::Planning, Some("Start".into()));
        assert!(result.success);
        assert_eq!(sm.state(), "planning");
    }

    #[test]
    fn test_task_queue_handle() {
        let queue = create_task_queue("test-session".into(), None);

        let id = queue.add_task(
            "Test task".into(),
            "Description".into(),
            TaskPriority::Medium,
        );

        let task = queue.get_task(id.clone()).unwrap();
        assert_eq!(task.subject, "Test task");
        assert_eq!(task.status, "pending");

        queue.start_task(id.clone()).unwrap();
        let task = queue.get_task(id.clone()).unwrap();
        assert_eq!(task.status, "running");

        let duration = queue.complete_task(id.clone()).unwrap();
        assert!(duration >= 0);

        let stats = queue.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.completed, 1);
    }
}
