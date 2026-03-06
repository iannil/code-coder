//! Autonomous mode module
//!
//! Provides state machine and task queue implementations for autonomous execution.

mod state;
mod queue;

pub use state::{
    AutonomousState, StateCategory, StateMachine, StateMachineConfig,
    StateMetadata, TransitionResult, VALID_TRANSITIONS,
};

pub use queue::{
    Task, TaskId, TaskPriority, TaskQueue, TaskQueueConfig,
    TaskQueueStats, TaskStatus,
};
