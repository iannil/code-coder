//! Docker-based sandbox for secure code execution.
//!
//! This module provides isolated execution of untrusted code using Docker containers.
//! Security features:
//! - Network disabled by default
//! - Memory and CPU limits
//! - No host filesystem access
//! - Automatic container cleanup

pub mod docker;
pub mod types;

pub use docker::DockerSandbox;
pub use types::{ExecutionAttempt, ExecutionResult, Language, SandboxConfig};
