//! Runtime adapter traits for platform abstraction.
//!
//! This module defines traits for abstracting platform differences, enabling
//! the same agent code to run across various environments.
//!
//! # Current Implementation
//!
//! Currently only `NativeRuntime` is implemented, which provides full access
//! on Mac, Linux, Docker containers, and Raspberry Pi.
//!
//! # Future Runtimes (Not Yet Implemented)
//!
//! The trait is designed to support additional runtimes when needed:
//!
//! - **`DockerRuntime`**: For sandboxed execution of untrusted agent code.
//!   Would have restricted shell access and isolated filesystem.
//!
//! - **`WorkersRuntime`**: For Cloudflare Workers deployment.
//!   No shell or filesystem access, limited memory budget.
//!
//! - **`EmbeddedRuntime`**: For resource-constrained devices.
//!   Strict memory budgets, no long-running process support.
//!
//! # When to Add New Runtimes
//!
//! Add a new runtime implementation when:
//! 1. Sandboxing untrusted agent code is required (`DockerRuntime`)
//! 2. Deploying to a new platform with different capabilities
//! 3. Implementing resource isolation for multi-tenant scenarios

use std::path::PathBuf;

/// Runtime adapter — abstracts platform differences so the same agent
/// code runs on native, Docker, Cloudflare Workers, Raspberry Pi, etc.
///
/// # Current Status
///
/// Only `NativeRuntime` is implemented. Other runtimes are placeholder
/// designs that will be implemented when the use case arises.
///
/// # Implementation Guide
///
/// When implementing a new runtime:
///
/// 1. Implement all required methods based on platform capabilities
/// 2. Add tests verifying the runtime behaves correctly
/// 3. Update the runtime selection logic in `daemon/mod.rs`
///
/// # Example
///
/// ```ignore
/// pub struct DockerRuntime {
///     container_id: String,
///     work_dir: PathBuf,
/// }
///
/// impl RuntimeAdapter for DockerRuntime {
///     fn name(&self) -> &str { "docker" }
///     fn has_shell_access(&self) -> bool { true } // via docker exec
///     fn has_filesystem_access(&self) -> bool { true } // via mount
///     fn storage_path(&self) -> PathBuf { self.work_dir.clone() }
///     fn supports_long_running(&self) -> bool { true }
///     fn memory_budget(&self) -> u64 { 512 * 1024 * 1024 } // 512MB
/// }
/// ```
pub trait RuntimeAdapter: Send + Sync {
    /// Human-readable runtime name
    fn name(&self) -> &str;

    /// Whether this runtime supports shell access
    fn has_shell_access(&self) -> bool;

    /// Whether this runtime supports filesystem access
    fn has_filesystem_access(&self) -> bool;

    /// Base storage path for this runtime
    fn storage_path(&self) -> PathBuf;

    /// Whether long-running processes (gateway, heartbeat) are supported
    fn supports_long_running(&self) -> bool;

    /// Maximum memory budget in bytes (0 = unlimited)
    fn memory_budget(&self) -> u64 {
        0
    }
}
