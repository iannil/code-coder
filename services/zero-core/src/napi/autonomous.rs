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

// ============================================================================
// Safety Guardrails Types (NAPI)
// ============================================================================

use crate::autonomous::{
    GuardrailConfig as RustGuardrailConfig, GuardrailStats as RustGuardrailStats,
    LoopDetection as RustLoopDetection, LoopType as RustLoopType, LimitType as RustLimitType,
    SafetyCheckResult as RustSafetyCheckResult, SafetyGuardrails as RustSafetyGuardrails,
    ToolResult as RustToolResult,
};

/// Loop type detected
#[napi(string_enum)]
pub enum LoopType {
    State,
    Tool,
    Decision,
}

impl From<RustLoopType> for LoopType {
    fn from(t: RustLoopType) -> Self {
        match t {
            RustLoopType::State => LoopType::State,
            RustLoopType::Tool => LoopType::Tool,
            RustLoopType::Decision => LoopType::Decision,
        }
    }
}

/// Tool result
#[napi(string_enum)]
pub enum NapiToolResult {
    Success,
    Error,
}

impl From<NapiToolResult> for RustToolResult {
    fn from(r: NapiToolResult) -> Self {
        match r {
            NapiToolResult::Success => RustToolResult::Success,
            NapiToolResult::Error => RustToolResult::Error,
        }
    }
}

/// Guardrail configuration
#[napi(object)]
pub struct NapiGuardrailConfig {
    pub max_state_transitions: Option<u32>,
    pub max_tool_retries: Option<u32>,
    pub max_decision_hesitation: Option<u32>,
    pub loop_detection_enabled: Option<bool>,
    pub loop_threshold: Option<u32>,
    pub auto_break_loops: Option<bool>,
}

impl From<NapiGuardrailConfig> for RustGuardrailConfig {
    fn from(c: NapiGuardrailConfig) -> Self {
        Self {
            max_state_transitions: c.max_state_transitions.unwrap_or(100) as usize,
            max_tool_retries: c.max_tool_retries.unwrap_or(3) as usize,
            max_decision_hesitation: c.max_decision_hesitation.unwrap_or(5) as usize,
            loop_detection_enabled: c.loop_detection_enabled.unwrap_or(true),
            loop_threshold: c.loop_threshold.unwrap_or(3) as usize,
            auto_break_loops: c.auto_break_loops.unwrap_or(true),
        }
    }
}

/// Loop detection result
#[napi(object)]
pub struct NapiLoopDetection {
    pub loop_type: String,
    pub pattern: Vec<String>,
    pub count: u32,
    pub broken: bool,
}

impl From<RustLoopDetection> for NapiLoopDetection {
    fn from(d: RustLoopDetection) -> Self {
        Self {
            loop_type: d.loop_type.to_string(),
            pattern: d.pattern,
            count: d.count as u32,
            broken: d.broken,
        }
    }
}

/// Safety check result
#[napi(object)]
pub struct NapiSafetyCheckResult {
    pub safe: bool,
    pub reason: Option<String>,
    pub limit_type: Option<String>,
}

impl From<RustSafetyCheckResult> for NapiSafetyCheckResult {
    fn from(r: RustSafetyCheckResult) -> Self {
        Self {
            safe: r.safe,
            reason: r.reason,
            limit_type: r.limit_type.map(|t| match t {
                RustLimitType::Transitions => "transitions".to_string(),
                RustLimitType::ToolRetries => "toolRetries".to_string(),
                RustLimitType::DecisionHesitation => "decisionHesitation".to_string(),
            }),
        }
    }
}

/// Guardrail statistics
#[napi(object)]
pub struct NapiGuardrailStats {
    pub state_transitions: u32,
    pub tool_calls: u32,
    pub decisions: u32,
    pub loops_broken: u32,
}

impl From<RustGuardrailStats> for NapiGuardrailStats {
    fn from(s: RustGuardrailStats) -> Self {
        Self {
            state_transitions: s.state_transitions as u32,
            tool_calls: s.tool_calls as u32,
            decisions: s.decisions as u32,
            loops_broken: s.loops_broken as u32,
        }
    }
}

// ============================================================================
// SafetyGuardrails Handle
// ============================================================================

/// Handle to safety guardrails
#[napi]
pub struct SafetyGuardrailsHandle {
    inner: Arc<Mutex<RustSafetyGuardrails>>,
}

/// Create new safety guardrails
#[napi]
pub fn create_safety_guardrails(
    session_id: String,
    config: Option<NapiGuardrailConfig>,
) -> SafetyGuardrailsHandle {
    let rust_config = config.map(Into::into).unwrap_or_default();
    SafetyGuardrailsHandle {
        inner: Arc::new(Mutex::new(RustSafetyGuardrails::new(session_id, rust_config))),
    }
}

#[napi]
impl SafetyGuardrailsHandle {
    /// Get session ID
    #[napi]
    pub fn session_id(&self) -> String {
        let guard = self.inner.lock().unwrap();
        guard.session_id().to_string()
    }

    /// Record a state transition
    #[napi]
    pub fn record_state_transition(&self, from: String, to: String) -> Option<NapiLoopDetection> {
        let mut guard = self.inner.lock().unwrap();
        guard.record_state_transition(&from, &to).map(Into::into)
    }

    /// Record a tool call
    #[napi]
    pub fn record_tool_call(
        &self,
        tool: String,
        input: String,
        result: NapiToolResult,
    ) -> Option<NapiLoopDetection> {
        let mut guard = self.inner.lock().unwrap();
        guard.record_tool_call(&tool, &input, result.into()).map(Into::into)
    }

    /// Record a decision
    #[napi]
    pub fn record_decision(
        &self,
        id: String,
        decision_type: String,
        result: String,
    ) -> Option<NapiLoopDetection> {
        let mut guard = self.inner.lock().unwrap();
        guard.record_decision(&id, &decision_type, &result).map(Into::into)
    }

    /// Detect all loops
    #[napi]
    pub fn detect_loops(&self) -> Vec<NapiLoopDetection> {
        let guard = self.inner.lock().unwrap();
        guard.detect_loops().into_iter().map(Into::into).collect()
    }

    /// Check safety limits
    #[napi]
    pub fn check_limits(&self) -> NapiSafetyCheckResult {
        let guard = self.inner.lock().unwrap();
        guard.check_limits().into()
    }

    /// Get statistics
    #[napi]
    pub fn get_stats(&self) -> NapiGuardrailStats {
        let guard = self.inner.lock().unwrap();
        guard.get_stats().into()
    }

    /// Clear all records
    #[napi]
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.clear();
    }

    /// Serialize to JSON
    #[napi]
    pub fn serialize(&self) -> String {
        let guard = self.inner.lock().unwrap();
        guard.serialize().to_string()
    }
}

// ============================================================================
// Safety Constraints Types (NAPI)
// ============================================================================

use crate::autonomous::{
    ConstraintCheckResult as RustConstraintCheckResult, ResourceBudget as RustResourceBudget,
    ResourceUsage as RustResourceUsage, ResourceWarning as RustResourceWarning,
    SafetyGuard as RustSafetyGuard,
};

/// Resource budget
#[napi(object)]
pub struct NapiResourceBudget {
    pub max_tokens: Option<u32>,
    pub max_cost_usd: Option<f64>,
    pub max_duration_minutes: Option<f64>,
    pub max_files_changed: Option<u32>,
    pub max_actions: Option<u32>,
}

impl From<NapiResourceBudget> for RustResourceBudget {
    fn from(b: NapiResourceBudget) -> Self {
        Self {
            max_tokens: b.max_tokens.unwrap_or(1_000_000) as usize,
            max_cost_usd: b.max_cost_usd.unwrap_or(10.0),
            max_duration_minutes: b.max_duration_minutes.unwrap_or(30.0),
            max_files_changed: b.max_files_changed.unwrap_or(50) as usize,
            max_actions: b.max_actions.unwrap_or(100) as usize,
        }
    }
}

impl From<RustResourceBudget> for NapiResourceBudget {
    fn from(b: RustResourceBudget) -> Self {
        Self {
            max_tokens: Some(b.max_tokens as u32),
            max_cost_usd: Some(b.max_cost_usd),
            max_duration_minutes: Some(b.max_duration_minutes),
            max_files_changed: Some(b.max_files_changed as u32),
            max_actions: Some(b.max_actions as u32),
        }
    }
}

/// Resource usage
#[napi(object)]
pub struct NapiResourceUsage {
    pub tokens_used: Option<u32>,
    pub cost_usd: Option<f64>,
    pub duration_minutes: Option<f64>,
    pub files_changed: Option<u32>,
    pub actions_performed: Option<u32>,
}

impl From<NapiResourceUsage> for RustResourceUsage {
    fn from(u: NapiResourceUsage) -> Self {
        Self {
            tokens_used: u.tokens_used.unwrap_or(0) as usize,
            cost_usd: u.cost_usd.unwrap_or(0.0),
            duration_minutes: u.duration_minutes.unwrap_or(0.0),
            files_changed: u.files_changed.unwrap_or(0) as usize,
            actions_performed: u.actions_performed.unwrap_or(0) as usize,
        }
    }
}

impl From<RustResourceUsage> for NapiResourceUsage {
    fn from(u: RustResourceUsage) -> Self {
        Self {
            tokens_used: Some(u.tokens_used as u32),
            cost_usd: Some(u.cost_usd),
            duration_minutes: Some(u.duration_minutes),
            files_changed: Some(u.files_changed as u32),
            actions_performed: Some(u.actions_performed as u32),
        }
    }
}

/// Constraint check result
#[napi(object)]
pub struct NapiConstraintCheckResult {
    pub safe: bool,
    pub reason: Option<String>,
    pub resource: Option<String>,
    pub current: Option<f64>,
    pub limit: Option<f64>,
}

impl From<RustConstraintCheckResult> for NapiConstraintCheckResult {
    fn from(r: RustConstraintCheckResult) -> Self {
        Self {
            safe: r.safe,
            reason: r.reason,
            resource: r.resource.map(|t| t.to_string()),
            current: r.current,
            limit: r.limit,
        }
    }
}

/// Resource warning
#[napi(object)]
pub struct NapiResourceWarning {
    pub session_id: String,
    pub resource: String,
    pub current: f64,
    pub limit: f64,
    pub percentage: u32,
}

impl From<RustResourceWarning> for NapiResourceWarning {
    fn from(w: RustResourceWarning) -> Self {
        Self {
            session_id: w.session_id,
            resource: w.resource.to_string(),
            current: w.current,
            limit: w.limit,
            percentage: w.percentage,
        }
    }
}

/// Check result with warnings
#[napi(object)]
pub struct NapiCheckWithWarnings {
    pub result: NapiConstraintCheckResult,
    pub warnings: Vec<NapiResourceWarning>,
}

// ============================================================================
// SafetyGuard Handle
// ============================================================================

/// Handle to safety guard
#[napi]
pub struct SafetyGuardHandle {
    inner: Mutex<RustSafetyGuard>,
}

/// Create new safety guard
#[napi]
pub fn create_safety_guard(
    session_id: String,
    budget: Option<NapiResourceBudget>,
) -> SafetyGuardHandle {
    let rust_budget = budget.map(Into::into).unwrap_or_default();
    SafetyGuardHandle {
        inner: Mutex::new(RustSafetyGuard::new(session_id, rust_budget)),
    }
}

#[napi]
impl SafetyGuardHandle {
    /// Get session ID
    #[napi]
    pub fn session_id(&self) -> String {
        let guard = self.inner.lock().unwrap();
        guard.session_id().to_string()
    }

    /// Check if operation is safe (with optional additional cost)
    #[napi]
    pub fn check(&self, additional: Option<NapiResourceUsage>) -> NapiCheckWithWarnings {
        let mut guard = self.inner.lock().unwrap();
        let rust_additional = additional.map(Into::into);
        let (result, warnings) = guard.check(rust_additional.as_ref());
        NapiCheckWithWarnings {
            result: result.into(),
            warnings: warnings.into_iter().map(Into::into).collect(),
        }
    }

    /// Quick check without warnings
    #[napi]
    pub fn quick_check(&self) -> NapiConstraintCheckResult {
        let guard = self.inner.lock().unwrap();
        guard.quick_check().into()
    }

    /// Record resource usage
    #[napi]
    pub fn record(&self, usage: NapiResourceUsage) {
        let mut guard = self.inner.lock().unwrap();
        let rust_usage: RustResourceUsage = usage.into();
        guard.record(&rust_usage);
    }

    /// Add tokens used
    #[napi]
    pub fn add_tokens(&self, tokens: u32) {
        let mut guard = self.inner.lock().unwrap();
        guard.add_tokens(tokens as usize);
    }

    /// Add cost
    #[napi]
    pub fn add_cost(&self, cost: f64) {
        let mut guard = self.inner.lock().unwrap();
        guard.add_cost(cost);
    }

    /// Add files changed
    #[napi]
    pub fn add_files_changed(&self, count: u32) {
        let mut guard = self.inner.lock().unwrap();
        guard.add_files_changed(count as usize);
    }

    /// Add actions performed
    #[napi]
    pub fn add_actions(&self, count: u32) {
        let mut guard = self.inner.lock().unwrap();
        guard.add_actions(count as usize);
    }

    /// Get current usage
    #[napi]
    pub fn get_usage(&self) -> NapiResourceUsage {
        let guard = self.inner.lock().unwrap();
        guard.get_usage().into()
    }

    /// Get remaining budget
    #[napi]
    pub fn get_remaining(&self) -> NapiResourceBudget {
        let guard = self.inner.lock().unwrap();
        guard.get_remaining().into()
    }

    /// Get surplus ratio (0.0-1.0)
    #[napi]
    pub fn get_surplus_ratio(&self) -> f64 {
        let guard = self.inner.lock().unwrap();
        guard.get_surplus_ratio()
    }

    /// Update budget
    #[napi]
    pub fn update_budget(&self, budget: NapiResourceBudget) {
        let mut guard = self.inner.lock().unwrap();
        guard.update_budget(budget.into());
    }

    /// Reset usage tracking
    #[napi]
    pub fn reset(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.reset();
    }

    /// Serialize to JSON
    #[napi]
    pub fn serialize(&self) -> String {
        let guard = self.inner.lock().unwrap();
        guard.serialize().to_string()
    }
}

// ============================================================================
// CLOSE Decision Framework Types (NAPI)
// ============================================================================

use crate::autonomous::{
    evaluate_close as rust_evaluate_close, CLOSEDimension as RustCLOSEDimension,
    CLOSEEvaluation as RustCLOSEEvaluation, CLOSEInput as RustCLOSEInput,
    CLOSEWeights as RustCLOSEWeights,
};

/// CLOSE dimension evaluation
#[napi(object)]
pub struct NapiCLOSEDimension {
    pub score: f64,
    pub confidence: f64,
    pub factors: Vec<String>,
}

impl From<RustCLOSEDimension> for NapiCLOSEDimension {
    fn from(d: RustCLOSEDimension) -> Self {
        Self {
            score: d.score,
            confidence: d.confidence,
            factors: d.factors,
        }
    }
}

/// CLOSE evaluation result
#[napi(object)]
pub struct NapiCLOSEEvaluation {
    pub convergence: NapiCLOSEDimension,
    pub leverage: NapiCLOSEDimension,
    pub optionality: NapiCLOSEDimension,
    pub surplus: NapiCLOSEDimension,
    pub evolution: NapiCLOSEDimension,
    pub total: f64,
    pub risk: f64,
    pub confidence: f64,
    pub recommended_gear: String,
    pub timestamp: i64,
}

impl From<RustCLOSEEvaluation> for NapiCLOSEEvaluation {
    fn from(e: RustCLOSEEvaluation) -> Self {
        Self {
            convergence: e.convergence.into(),
            leverage: e.leverage.into(),
            optionality: e.optionality.into(),
            surplus: e.surplus.into(),
            evolution: e.evolution.into(),
            total: e.total,
            risk: e.risk,
            confidence: e.confidence,
            recommended_gear: e.recommended_gear.to_string(),
            timestamp: e.timestamp,
        }
    }
}

/// CLOSE weights
#[napi(object)]
pub struct NapiCLOSEWeights {
    pub convergence: Option<f64>,
    pub leverage: Option<f64>,
    pub optionality: Option<f64>,
    pub surplus: Option<f64>,
    pub evolution: Option<f64>,
}

impl From<NapiCLOSEWeights> for RustCLOSEWeights {
    fn from(w: NapiCLOSEWeights) -> Self {
        let defaults = RustCLOSEWeights::default();
        Self {
            convergence: w.convergence.unwrap_or(defaults.convergence),
            leverage: w.leverage.unwrap_or(defaults.leverage),
            optionality: w.optionality.unwrap_or(defaults.optionality),
            surplus: w.surplus.unwrap_or(defaults.surplus),
            evolution: w.evolution.unwrap_or(defaults.evolution),
        }
    }
}

/// CLOSE input data
#[napi(object)]
pub struct NapiCLOSEInput {
    // Convergence factors
    pub snapshot_confidence: Option<f64>,
    pub build_status: Option<String>,
    pub session_health: Option<String>,
    pub critical_anomalies: Option<u32>,
    pub strong_patterns: Option<u32>,

    // Leverage factors
    pub high_impact_opportunities: Option<u32>,
    pub medium_impact_opportunities: Option<u32>,
    pub external_opportunities: Option<u32>,
    pub external_risks: Option<u32>,

    // Optionality factors
    pub total_opportunities: Option<u32>,
    pub pattern_types: Option<u32>,
    pub anomaly_count: Option<u32>,
    pub decision_quality: Option<f64>,
    pub recent_errors: Option<u32>,

    // Surplus factors
    pub token_usage: Option<u32>,
    pub cost: Option<f64>,
    pub consensus_strength: Option<f64>,
    pub coverage_gaps: Option<u32>,

    // Evolution factors
    pub learning_opportunities: Option<u32>,
    pub recent_changes: Option<u32>,
    pub tech_debt_level: Option<String>,
    pub dismissed_anomalies: Option<u32>,
    pub active_anomalies: Option<u32>,

    // Trend
    pub recent_trend_avg: Option<f64>,
    pub older_trend_avg: Option<f64>,
}

impl From<NapiCLOSEInput> for RustCLOSEInput {
    fn from(i: NapiCLOSEInput) -> Self {
        Self {
            snapshot_confidence: i.snapshot_confidence.unwrap_or(0.5),
            build_status: i.build_status.unwrap_or_default(),
            session_health: i.session_health.unwrap_or_default(),
            critical_anomalies: i.critical_anomalies.unwrap_or(0) as usize,
            strong_patterns: i.strong_patterns.unwrap_or(0) as usize,
            high_impact_opportunities: i.high_impact_opportunities.unwrap_or(0) as usize,
            medium_impact_opportunities: i.medium_impact_opportunities.unwrap_or(0) as usize,
            external_opportunities: i.external_opportunities.unwrap_or(0) as usize,
            external_risks: i.external_risks.unwrap_or(0) as usize,
            total_opportunities: i.total_opportunities.unwrap_or(0) as usize,
            pattern_types: i.pattern_types.unwrap_or(0) as usize,
            anomaly_count: i.anomaly_count.unwrap_or(0) as usize,
            decision_quality: i.decision_quality.unwrap_or(0.5),
            recent_errors: i.recent_errors.unwrap_or(0) as usize,
            token_usage: i.token_usage.unwrap_or(0) as usize,
            cost: i.cost.unwrap_or(0.0),
            consensus_strength: i.consensus_strength.unwrap_or(0.5),
            coverage_gaps: i.coverage_gaps.unwrap_or(0) as usize,
            learning_opportunities: i.learning_opportunities.unwrap_or(0) as usize,
            recent_changes: i.recent_changes.unwrap_or(0) as usize,
            tech_debt_level: i.tech_debt_level.unwrap_or_default(),
            dismissed_anomalies: i.dismissed_anomalies.unwrap_or(0) as usize,
            active_anomalies: i.active_anomalies.unwrap_or(0) as usize,
            recent_trend_avg: i.recent_trend_avg,
            older_trend_avg: i.older_trend_avg,
        }
    }
}

/// Evaluate CLOSE decision framework
#[napi]
pub fn evaluate_close(input: NapiCLOSEInput, weights: Option<NapiCLOSEWeights>) -> NapiCLOSEEvaluation {
    let rust_input: RustCLOSEInput = input.into();
    let rust_weights = weights.map(Into::into);
    rust_evaluate_close(&rust_input, rust_weights).into()
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

    #[test]
    fn test_safety_guardrails_handle() {
        let guardrails = create_safety_guardrails("test-session".into(), None);
        assert_eq!(guardrails.session_id(), "test-session");

        // Record state transitions
        let result = guardrails.record_state_transition("idle".into(), "planning".into());
        assert!(result.is_none()); // No loop on first transition

        let stats = guardrails.get_stats();
        assert_eq!(stats.state_transitions, 1);
    }

    #[test]
    fn test_safety_guard_handle() {
        let guard = create_safety_guard("test-session".into(), None);
        assert_eq!(guard.session_id(), "test-session");

        // Record usage
        guard.add_tokens(1000);
        guard.add_cost(0.5);

        let usage = guard.get_usage();
        assert_eq!(usage.tokens_used, Some(1000));

        // Check should pass
        let result = guard.quick_check();
        assert!(result.safe);
    }

    #[test]
    fn test_close_evaluation() {
        let input = NapiCLOSEInput {
            snapshot_confidence: Some(0.8),
            build_status: Some("passing".to_string()),
            session_health: Some("healthy".to_string()),
            high_impact_opportunities: Some(2),
            total_opportunities: Some(5),
            consensus_strength: Some(0.7),
            critical_anomalies: None,
            strong_patterns: None,
            medium_impact_opportunities: None,
            external_opportunities: None,
            external_risks: None,
            pattern_types: None,
            anomaly_count: None,
            decision_quality: None,
            recent_errors: None,
            token_usage: None,
            cost: None,
            coverage_gaps: None,
            learning_opportunities: None,
            recent_changes: None,
            tech_debt_level: None,
            dismissed_anomalies: None,
            active_anomalies: None,
            recent_trend_avg: None,
            older_trend_avg: None,
        };

        let result = evaluate_close(input, None);
        assert!(result.total >= 0.0 && result.total <= 10.0);
        assert!(result.risk >= 0.0 && result.risk <= 10.0);
        assert!(!result.recommended_gear.is_empty());
    }
}
