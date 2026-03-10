//! Safety Constraints for Resource Budget Management
//!
//! Monitors and enforces resource constraints during autonomous execution:
//! - Token usage limits
//! - Cost limits (USD)
//! - Duration limits
//! - File change limits
//! - Action limits
//!
//! # Philosophy
//!
//! Following the "可用余量" (Surplus) principle, constraints ensure we maintain
//! sufficient margin to recover from errors and complete objectives.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// Resource budget limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBudget {
    /// Maximum tokens to use
    pub max_tokens: usize,
    /// Maximum cost in USD
    pub max_cost_usd: f64,
    /// Maximum duration in minutes
    pub max_duration_minutes: f64,
    /// Maximum files changed
    pub max_files_changed: usize,
    /// Maximum actions performed
    pub max_actions: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_tokens: 1_000_000,
            max_cost_usd: 10.0,
            max_duration_minutes: 30.0,
            max_files_changed: 50,
            max_actions: 100,
        }
    }
}

/// Current resource usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    /// Tokens used so far
    pub tokens_used: usize,
    /// Cost in USD so far
    pub cost_usd: f64,
    /// Duration in minutes
    pub duration_minutes: f64,
    /// Files changed
    pub files_changed: usize,
    /// Actions performed
    pub actions_performed: usize,
}

impl ResourceUsage {
    /// Add usage from another ResourceUsage
    pub fn add(&mut self, other: &ResourceUsage) {
        self.tokens_used += other.tokens_used;
        self.cost_usd += other.cost_usd;
        self.files_changed += other.files_changed;
        self.actions_performed += other.actions_performed;
        // Note: duration_minutes is calculated from start time, not added
    }
}

/// Resource type that exceeded limit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
    Tokens,
    Cost,
    Duration,
    Files,
    Actions,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tokens => write!(f, "tokens"),
            Self::Cost => write!(f, "cost"),
            Self::Duration => write!(f, "time"),
            Self::Files => write!(f, "files"),
            Self::Actions => write!(f, "actions"),
        }
    }
}

/// Safety check result for resource constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintCheckResult {
    /// Whether the check passed (safe to continue)
    pub safe: bool,
    /// Reason for failure (if not safe)
    pub reason: Option<String>,
    /// Resource type that exceeded limit
    pub resource: Option<ResourceType>,
    /// Current usage value
    pub current: Option<f64>,
    /// Limit value
    pub limit: Option<f64>,
}

impl ConstraintCheckResult {
    /// Create a passing result
    pub fn ok() -> Self {
        Self {
            safe: true,
            reason: None,
            resource: None,
            current: None,
            limit: None,
        }
    }

    /// Create a failing result
    pub fn fail(resource: ResourceType, current: f64, limit: f64) -> Self {
        Self {
            safe: false,
            reason: Some(format!(
                "Resource limit exceeded for {}: {:.2}/{:.2}",
                resource, current, limit
            )),
            resource: Some(resource),
            current: Some(current),
            limit: Some(limit),
        }
    }
}

/// Resource warning (approaching limit)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWarning {
    pub session_id: String,
    pub resource: ResourceType,
    pub current: f64,
    pub limit: f64,
    pub percentage: u32,
}

/// Safety guard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyConfig {
    /// Resource budget
    pub budget: ResourceBudget,
    /// Percentage (0-100) at which to warn
    pub warn_threshold: u32,
    /// Whether to enforce hard limits
    pub hard_limit: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            budget: ResourceBudget::default(),
            warn_threshold: 80,
            hard_limit: true,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SafetyGuard
// ══════════════════════════════════════════════════════════════════════════════

/// Safety guard for monitoring and enforcing resource constraints
#[derive(Debug)]
pub struct SafetyGuard {
    session_id: String,
    config: SafetyConfig,
    usage: ResourceUsage,
    start_time: Instant,
    warnings_sent: HashSet<ResourceType>,
}

impl SafetyGuard {
    /// Create a new safety guard
    pub fn new(session_id: impl Into<String>, budget: ResourceBudget) -> Self {
        Self {
            session_id: session_id.into(),
            config: SafetyConfig {
                budget,
                ..Default::default()
            },
            usage: ResourceUsage::default(),
            start_time: Instant::now(),
            warnings_sent: HashSet::new(),
        }
    }

    /// Create with default budget
    pub fn with_defaults(session_id: impl Into<String>) -> Self {
        Self::new(session_id, ResourceBudget::default())
    }

    /// Create with full configuration
    pub fn with_config(session_id: impl Into<String>, config: SafetyConfig) -> Self {
        Self {
            session_id: session_id.into(),
            config,
            usage: ResourceUsage::default(),
            start_time: Instant::now(),
            warnings_sent: HashSet::new(),
        }
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get configuration
    pub fn config(&self) -> &SafetyConfig {
        &self.config
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Resource Checking
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if an operation is safe to perform
    ///
    /// Optionally accepts additional cost to add before checking.
    /// Returns warnings if approaching limits.
    pub fn check(&mut self, additional: Option<&ResourceUsage>) -> (ConstraintCheckResult, Vec<ResourceWarning>) {
        // Update duration
        self.usage.duration_minutes = self.start_time.elapsed().as_secs_f64() / 60.0;

        // Add additional costs
        if let Some(cost) = additional {
            self.usage.add(cost);
        }

        let mut warnings = Vec::new();
        let budget = &self.config.budget;

        // Check each resource
        let checks = [
            (
                ResourceType::Tokens,
                self.usage.tokens_used as f64,
                budget.max_tokens as f64,
            ),
            (
                ResourceType::Cost,
                self.usage.cost_usd,
                budget.max_cost_usd,
            ),
            (
                ResourceType::Duration,
                self.usage.duration_minutes,
                budget.max_duration_minutes,
            ),
            (
                ResourceType::Files,
                self.usage.files_changed as f64,
                budget.max_files_changed as f64,
            ),
            (
                ResourceType::Actions,
                self.usage.actions_performed as f64,
                budget.max_actions as f64,
            ),
        ];

        for (resource, current, limit) in checks {
            // Check hard limit
            if current >= limit && self.config.hard_limit {
                return (ConstraintCheckResult::fail(resource, current, limit), warnings);
            }

            // Check warning threshold
            let percentage = ((current / limit) * 100.0) as u32;
            if percentage >= self.config.warn_threshold && !self.warnings_sent.contains(&resource) {
                self.warnings_sent.insert(resource);
                warnings.push(ResourceWarning {
                    session_id: self.session_id.clone(),
                    resource,
                    current,
                    limit,
                    percentage,
                });
            }
        }

        (ConstraintCheckResult::ok(), warnings)
    }

    /// Quick check without warnings (does not modify state)
    pub fn quick_check(&self) -> ConstraintCheckResult {
        let budget = &self.config.budget;
        let duration = self.start_time.elapsed().as_secs_f64() / 60.0;

        let checks = [
            (
                ResourceType::Tokens,
                self.usage.tokens_used as f64,
                budget.max_tokens as f64,
            ),
            (
                ResourceType::Cost,
                self.usage.cost_usd,
                budget.max_cost_usd,
            ),
            (ResourceType::Duration, duration, budget.max_duration_minutes),
            (
                ResourceType::Files,
                self.usage.files_changed as f64,
                budget.max_files_changed as f64,
            ),
            (
                ResourceType::Actions,
                self.usage.actions_performed as f64,
                budget.max_actions as f64,
            ),
        ];

        for (resource, current, limit) in checks {
            if current >= limit && self.config.hard_limit {
                return ConstraintCheckResult::fail(resource, current, limit);
            }
        }

        ConstraintCheckResult::ok()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Usage Tracking
    // ─────────────────────────────────────────────────────────────────────────

    /// Record resource usage
    pub fn record(&mut self, usage: &ResourceUsage) {
        self.usage.add(usage);
    }

    /// Add tokens used
    pub fn add_tokens(&mut self, tokens: usize) {
        self.usage.tokens_used += tokens;
    }

    /// Add cost
    pub fn add_cost(&mut self, cost: f64) {
        self.usage.cost_usd += cost;
    }

    /// Add files changed
    pub fn add_files_changed(&mut self, count: usize) {
        self.usage.files_changed += count;
    }

    /// Add actions performed
    pub fn add_actions(&mut self, count: usize) {
        self.usage.actions_performed += count;
    }

    /// Get current usage
    pub fn get_usage(&self) -> ResourceUsage {
        let mut usage = self.usage.clone();
        usage.duration_minutes = self.start_time.elapsed().as_secs_f64() / 60.0;
        usage
    }

    /// Get remaining budget
    pub fn get_remaining(&self) -> ResourceBudget {
        let usage = self.get_usage();
        let budget = &self.config.budget;

        ResourceBudget {
            max_tokens: budget.max_tokens.saturating_sub(usage.tokens_used),
            max_cost_usd: (budget.max_cost_usd - usage.cost_usd).max(0.0),
            max_duration_minutes: (budget.max_duration_minutes - usage.duration_minutes).max(0.0),
            max_files_changed: budget.max_files_changed.saturating_sub(usage.files_changed),
            max_actions: budget.max_actions.saturating_sub(usage.actions_performed),
        }
    }

    /// Get surplus ratio (0.0-1.0, higher = more resources remaining)
    pub fn get_surplus_ratio(&self) -> f64 {
        let remaining = self.get_remaining();
        let budget = &self.config.budget;

        let ratios = [
            remaining.max_tokens as f64 / budget.max_tokens as f64,
            remaining.max_cost_usd / budget.max_cost_usd,
            remaining.max_duration_minutes / budget.max_duration_minutes,
            remaining.max_files_changed as f64 / budget.max_files_changed as f64,
            remaining.max_actions as f64 / budget.max_actions as f64,
        ];

        ratios.iter().sum::<f64>() / ratios.len() as f64
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Configuration
    // ─────────────────────────────────────────────────────────────────────────

    /// Update budget
    pub fn update_budget(&mut self, budget: ResourceBudget) {
        self.config.budget = budget;
    }

    /// Reset usage (restart tracking)
    pub fn reset(&mut self) {
        self.usage = ResourceUsage::default();
        self.start_time = Instant::now();
        self.warnings_sent.clear();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Serialization
    // ─────────────────────────────────────────────────────────────────────────

    /// Serialize current state (for persistence/restore)
    pub fn serialize(&self) -> serde_json::Value {
        let usage = self.get_usage();
        serde_json::json!({
            "sessionId": self.session_id,
            "usage": usage,
            "config": self.config,
            "startTime": chrono::Utc::now().timestamp_millis()
        })
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_guard_default() {
        let guard = SafetyGuard::with_defaults("test-session");
        assert_eq!(guard.session_id(), "test-session");
        assert!(guard.quick_check().safe);
    }

    #[test]
    fn test_usage_tracking() {
        let mut guard = SafetyGuard::with_defaults("test");

        guard.add_tokens(1000);
        guard.add_cost(0.5);
        guard.add_files_changed(1);
        guard.add_actions(2);

        let usage = guard.get_usage();
        assert_eq!(usage.tokens_used, 1000);
        assert!((usage.cost_usd - 0.5).abs() < 0.001);
        assert_eq!(usage.files_changed, 1);
        assert_eq!(usage.actions_performed, 2);
    }

    #[test]
    fn test_limit_exceeded() {
        let budget = ResourceBudget {
            max_tokens: 100,
            ..Default::default()
        };
        let mut guard = SafetyGuard::new("test", budget);

        guard.add_tokens(100);
        let result = guard.quick_check();

        assert!(!result.safe);
        assert_eq!(result.resource, Some(ResourceType::Tokens));
    }

    #[test]
    fn test_warning_threshold() {
        let budget = ResourceBudget {
            max_tokens: 100,
            ..Default::default()
        };
        let config = SafetyConfig {
            budget,
            warn_threshold: 80,
            hard_limit: true,
        };
        let mut guard = SafetyGuard::with_config("test", config);

        guard.add_tokens(80);
        let (result, warnings) = guard.check(None);

        assert!(result.safe);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, ResourceType::Tokens);
    }

    #[test]
    fn test_remaining_budget() {
        let budget = ResourceBudget {
            max_tokens: 1000,
            max_cost_usd: 10.0,
            ..Default::default()
        };
        let mut guard = SafetyGuard::new("test", budget);

        guard.add_tokens(300);
        guard.add_cost(3.0);

        let remaining = guard.get_remaining();
        assert_eq!(remaining.max_tokens, 700);
        assert!((remaining.max_cost_usd - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_surplus_ratio() {
        let budget = ResourceBudget {
            max_tokens: 1000,
            max_cost_usd: 10.0,
            max_duration_minutes: 30.0,
            max_files_changed: 50,
            max_actions: 100,
        };
        let guard = SafetyGuard::new("test", budget);

        // Fresh guard should have high surplus
        let ratio = guard.get_surplus_ratio();
        assert!(ratio > 0.9);
    }

    #[test]
    fn test_reset() {
        let mut guard = SafetyGuard::with_defaults("test");

        guard.add_tokens(1000);
        guard.add_cost(5.0);

        guard.reset();

        let usage = guard.get_usage();
        assert_eq!(usage.tokens_used, 0);
        assert!((usage.cost_usd).abs() < 0.001);
    }

    #[test]
    fn test_serialization() {
        let mut guard = SafetyGuard::with_defaults("test");
        guard.add_tokens(500);

        let json = guard.serialize();
        assert!(json.get("usage").is_some());
        assert!(json.get("config").is_some());
    }
}
