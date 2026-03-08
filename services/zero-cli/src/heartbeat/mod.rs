//! Heartbeat system for periodic task execution and service health monitoring.
//!
//! This module provides two main capabilities:
//!
//! 1. **HeartbeatEngine** - Reads HEARTBEAT.md and executes tasks periodically
//! 2. **HeartbeatMonitor** - Tracks service health and handles stale services
//!
//! ## Design Principle
//!
//! The heartbeat system is a **deterministic** component:
//! - Health checks have predictable outcomes
//! - Stale actions follow configured rules
//! - No LLM reasoning required

pub mod engine;
pub mod health;
pub mod monitor;

pub use engine::HeartbeatEngine;
pub use health::{HealthStatus, ServiceHealth, SystemHealth};
pub use monitor::{HeartbeatMonitor, MonitorConfig, ServiceRegistration, StaleAction};
