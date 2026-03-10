//! Autonomous mode module
//!
//! Provides state machine, task queue, safety guardrails, resource constraint,
//! and CLOSE decision framework implementations for autonomous execution.

mod constraints;
mod decision;
mod guardrails;
mod queue;
mod state;

pub use constraints::{
    ConstraintCheckResult, ResourceBudget, ResourceType, ResourceUsage, ResourceWarning,
    SafetyConfig, SafetyGuard,
};

pub use decision::{
    evaluate_close, CLOSEDimension, CLOSEEvaluation, CLOSEInput, CLOSETrend, CLOSEWeights,
    GearRecommendation,
};

pub use guardrails::{
    DecisionRecord, GuardrailConfig, GuardrailStats, LimitType, LoopDetection, LoopType,
    SafetyCheckResult, SafetyGuardrails, StateTransition, ToolCall, ToolResult,
};

pub use queue::{
    Task, TaskId, TaskPriority, TaskQueue, TaskQueueConfig, TaskQueueStats, TaskStatus,
};

pub use state::{
    AutonomousState, StateCategory, StateMachine, StateMachineConfig, StateMetadata,
    TransitionResult, VALID_TRANSITIONS,
};
