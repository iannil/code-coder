//! Auto-Approve Permission Engine
//!
//! Provides risk-based automatic approval for tool calls in autonomous/unattended mode.
//! This module implements the core auto-approval logic that was previously in TypeScript.
//!
//! # Design
//!
//! The engine evaluates tool operations against a configuration and returns approval decisions:
//! - **Approve**: Operation can proceed automatically
//! - **Reject**: Operation needs manual approval
//! - **Defer**: No decision made, fall back to other handlers
//!
//! # Adaptive Risk Assessment
//!
//! Risk levels can be adjusted based on context factors:
//! - Session error count increases risk
//! - High success rate decreases risk
//! - After-hours on high-sensitivity projects increases risk
//!
//! # Example
//!
//! ```rust
//! use zero_core::security::auto_approve::{AutoApproveConfig, AutoApproveEngine};
//! use zero_core::security::risk::RiskLevel;
//!
//! let config = AutoApproveConfig {
//!     enabled: true,
//!     allowed_tools: vec!["Read".to_string(), "Glob".to_string()],
//!     risk_threshold: RiskLevel::Low,
//!     timeout_ms: 0,
//!     unattended: false,
//! };
//!
//! let engine = AutoApproveEngine::new(config);
//! let decision = engine.evaluate("Read", None);
//! assert!(decision.approved);
//! ```

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use super::risk::{
    RiskLevel, RiskAssessment, assess_bash_risk, assess_file_risk,
    tool_base_risk, risk_at_or_below_threshold,
};

// ============================================================================
// Configuration
// ============================================================================

/// Auto-approve configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApproveConfig {
    /// Enable auto-approval
    pub enabled: bool,

    /// Tools allowed for auto-approval (whitelist).
    /// Empty list = use risk-based evaluation only.
    pub allowed_tools: Vec<String>,

    /// Maximum risk level for auto-approval
    pub risk_threshold: RiskLevel,

    /// Timeout in milliseconds before auto-approving
    /// (only applies to non-critical operations in unattended mode)
    pub timeout_ms: u64,

    /// Whether running in unattended mode
    pub unattended: bool,
}

impl Default for AutoApproveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tools: Vec::new(),
            risk_threshold: RiskLevel::Low,
            timeout_ms: 0,
            unattended: false,
        }
    }
}

impl AutoApproveConfig {
    /// Create a safe-tools-only configuration
    pub fn safe_only(unattended: bool) -> Self {
        Self {
            enabled: true,
            allowed_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "LS".to_string(),
                "WebFetch".to_string(),
                "WebSearch".to_string(),
            ],
            risk_threshold: RiskLevel::Low,
            timeout_ms: 0,
            unattended,
        }
    }

    /// Create a permissive configuration for trusted environments
    pub fn permissive(unattended: bool) -> Self {
        Self {
            enabled: true,
            allowed_tools: Vec::new(), // Use risk-based evaluation only
            risk_threshold: RiskLevel::Medium,
            timeout_ms: 30000,
            unattended,
        }
    }
}

// ============================================================================
// Approval Decision
// ============================================================================

/// Decision result for an auto-approval evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// Whether the operation is approved
    pub approved: bool,

    /// Risk level of the operation
    pub risk: RiskLevel,

    /// Reason for the decision
    pub reason: String,

    /// Whether this was a timeout-based approval
    pub timeout_approved: bool,

    /// Whether the operation can potentially be auto-approved
    /// (false for critical operations)
    pub auto_approvable: bool,
}

impl ApprovalDecision {
    /// Create an approved decision
    pub fn approve(risk: RiskLevel, reason: impl Into<String>) -> Self {
        Self {
            approved: true,
            risk,
            reason: reason.into(),
            timeout_approved: false,
            auto_approvable: true,
        }
    }

    /// Create a rejected decision
    pub fn reject(risk: RiskLevel, reason: impl Into<String>) -> Self {
        Self {
            approved: false,
            risk,
            reason: reason.into(),
            timeout_approved: false,
            auto_approvable: risk != RiskLevel::Critical,
        }
    }

    /// Create a timeout-approved decision
    pub fn timeout_approve(risk: RiskLevel, reason: impl Into<String>) -> Self {
        Self {
            approved: true,
            risk,
            reason: reason.into(),
            timeout_approved: true,
            auto_approvable: true,
        }
    }

    /// Create a decision that defers to other handlers
    pub fn defer(risk: RiskLevel, reason: impl Into<String>) -> Self {
        Self {
            approved: false,
            risk,
            reason: reason.into(),
            timeout_approved: false,
            auto_approvable: true,
        }
    }
}

// ============================================================================
// Tool Input for Risk Assessment
// ============================================================================

/// Input data for tool risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolInput {
    /// Bash command
    Bash { command: String },
    /// File operation
    File { path: String },
    /// Generic JSON input
    Json(serde_json::Value),
    /// No input
    None,
}

impl Default for ToolInput {
    fn default() -> Self {
        ToolInput::None
    }
}

// ============================================================================
// Audit Entry
// ============================================================================

/// Auto-approve audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Permission request ID (if available)
    pub permission_id: Option<String>,

    /// Tool name
    pub tool: String,

    /// Pattern(s) associated with the request
    pub pattern: Option<Vec<String>>,

    /// Risk level
    pub risk: RiskLevel,

    /// Decision made
    pub decision: String,

    /// Reason for the decision
    pub reason: String,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        tool: impl Into<String>,
        risk: RiskLevel,
        decision: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        Self {
            timestamp: format!("{}", now),
            permission_id: None,
            tool: tool.into(),
            pattern: None,
            risk,
            decision: decision.into(),
            reason: reason.into(),
        }
    }

    /// Set permission ID
    pub fn with_permission_id(mut self, id: impl Into<String>) -> Self {
        self.permission_id = Some(id.into());
        self
    }

    /// Set pattern
    pub fn with_pattern(mut self, pattern: Vec<String>) -> Self {
        self.pattern = Some(pattern);
        self
    }
}

// ============================================================================
// Execution Context for Adaptive Risk
// ============================================================================

/// Project sensitivity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectSensitivity {
    Low,
    Medium,
    High,
}

impl Default for ProjectSensitivity {
    fn default() -> Self {
        ProjectSensitivity::Low
    }
}

/// Time of day categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeOfDay {
    Business,
    AfterHours,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        TimeOfDay::Business
    }
}

/// Execution context for adaptive risk assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Session ID
    pub session_id: String,

    /// Current iteration
    pub iteration: u32,

    /// Errors in this session
    pub errors: u32,

    /// Successful operations in session
    pub successes: u32,

    /// Project path
    pub project_path: Option<String>,

    /// Is production environment
    pub is_production: bool,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            ..Default::default()
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.errors;
        if total == 0 {
            1.0
        } else {
            self.successes as f64 / total as f64
        }
    }

    /// Determine project sensitivity
    pub fn project_sensitivity(&self) -> ProjectSensitivity {
        if self.is_production {
            return ProjectSensitivity::High;
        }

        if let Some(ref path) = self.project_path {
            let lower = path.to_lowercase();
            if lower.contains("/prod")
                || lower.contains("/production")
                || lower.contains("/live")
                || lower.contains("/release")
                || lower.contains("/.env")
                || lower.contains("/secrets")
                || lower.contains("/credentials")
            {
                return ProjectSensitivity::High;
            }
            if lower.contains("/staging")
                || lower.contains("/pre-prod")
                || lower.contains("/uat")
                || lower.contains("/config")
                || lower.contains("/settings")
            {
                return ProjectSensitivity::Medium;
            }
        }

        ProjectSensitivity::Low
    }
}

// ============================================================================
// Adaptive Risk Result
// ============================================================================

/// Result of adaptive risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRiskResult {
    /// Base risk level
    pub base_risk: RiskLevel,

    /// Adjusted risk level
    pub adjusted_risk: RiskLevel,

    /// Adjustment applied (+N = increased, -N = decreased)
    pub adjustment: i32,

    /// Reason for adjustment
    pub adjustment_reason: String,
}

// ============================================================================
// Auto-Approve Engine
// ============================================================================

/// Auto-approve permission engine
#[derive(Debug, Clone)]
pub struct AutoApproveEngine {
    /// Configuration
    config: AutoApproveConfig,

    /// Allowed tools as a HashSet for fast lookup
    allowed_set: HashSet<String>,

    /// Audit log (in-memory, capped)
    audit_log: Vec<AuditEntry>,

    /// Maximum audit log entries
    max_audit_entries: usize,
}

impl AutoApproveEngine {
    /// Create a new auto-approve engine
    pub fn new(config: AutoApproveConfig) -> Self {
        let allowed_set = config.allowed_tools.iter().cloned().collect();
        Self {
            config,
            allowed_set,
            audit_log: Vec::new(),
            max_audit_entries: 1000,
        }
    }

    /// Create an engine with safe-only configuration
    pub fn safe_only(unattended: bool) -> Self {
        Self::new(AutoApproveConfig::safe_only(unattended))
    }

    /// Create an engine with permissive configuration
    pub fn permissive(unattended: bool) -> Self {
        Self::new(AutoApproveConfig::permissive(unattended))
    }

    /// Get the configuration
    pub fn config(&self) -> &AutoApproveConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: AutoApproveConfig) {
        self.allowed_set = config.allowed_tools.iter().cloned().collect();
        self.config = config;
    }

    /// Assess risk for a tool operation
    pub fn assess_risk(&self, tool: &str, input: &ToolInput) -> RiskAssessment {
        // Get base risk for the tool
        let base = tool_base_risk(tool);

        // Handle special cases based on input
        match input {
            ToolInput::Bash { command } => {
                // Use Bash-specific risk assessment
                assess_bash_risk(command)
            }
            ToolInput::File { path } => {
                // Use file-specific risk assessment, but take the higher risk
                let file_risk = assess_file_risk(path);
                if file_risk.risk > base {
                    file_risk
                } else {
                    RiskAssessment::new(base, "Base risk for tool")
                }
            }
            ToolInput::Json(value) => {
                // Try to extract command or file_path from JSON
                if tool == "Bash" {
                    if let Some(cmd) = value.get("command").and_then(|v| v.as_str()) {
                        return assess_bash_risk(cmd);
                    }
                }
                if tool == "Write" || tool == "Edit" {
                    if let Some(path) = value.get("file_path").and_then(|v| v.as_str()) {
                        let file_risk = assess_file_risk(path);
                        if file_risk.risk > base {
                            return file_risk;
                        }
                    }
                }
                RiskAssessment::new(base, "Base risk for tool")
            }
            ToolInput::None => {
                RiskAssessment::new(base, "Base risk for tool")
            }
        }
    }

    /// Evaluate adaptive risk based on context
    pub fn evaluate_adaptive_risk(
        &self,
        tool: &str,
        input: &ToolInput,
        ctx: &ExecutionContext,
    ) -> AdaptiveRiskResult {
        let assessment = self.assess_risk(tool, input);
        let base_risk = assessment.risk;

        let mut adjustment = 0i32;
        let mut reasons = Vec::new();

        // Success rate factor
        if ctx.success_rate() >= 0.95 && ctx.errors == 0 {
            adjustment -= 1;
            reasons.push("High success rate (≥95%) with no errors");
        }

        // Error factor
        if ctx.errors > 0 {
            adjustment += 1;
            reasons.push("Error(s) in session");
        }

        // Multiple errors factor
        if ctx.errors >= 3 {
            adjustment += 1;
            reasons.push("Multiple errors (≥3) in session");
        }

        // Time + sensitivity factor
        // Note: In a real implementation, we'd check actual time
        let sensitivity = ctx.project_sensitivity();
        if sensitivity == ProjectSensitivity::High {
            if adjustment == 0 {
                adjustment += 1;
                reasons.push("High-sensitivity project environment");
            }
        }

        // Apply adjustment
        let adjusted_risk = adjust_risk_level(base_risk, adjustment);

        AdaptiveRiskResult {
            base_risk,
            adjusted_risk,
            adjustment,
            adjustment_reason: if reasons.is_empty() {
                "No adjustment".to_string()
            } else {
                reasons.join("; ")
            },
        }
    }

    /// Evaluate a tool operation for auto-approval
    ///
    /// Returns an approval decision based on configuration and risk assessment.
    pub fn evaluate(&self, tool: &str, input: Option<ToolInput>) -> ApprovalDecision {
        let input = input.unwrap_or(ToolInput::None);

        // If disabled, reject
        if !self.config.enabled {
            return ApprovalDecision::reject(
                RiskLevel::Medium,
                "Auto-approve disabled",
            );
        }

        // Assess risk
        let assessment = self.assess_risk(tool, &input);

        // Critical operations are ALWAYS rejected
        if assessment.risk == RiskLevel::Critical {
            return ApprovalDecision {
                approved: false,
                risk: RiskLevel::Critical,
                reason: format!("Critical operation blocked: {}", assessment.reason),
                timeout_approved: false,
                auto_approvable: false,
            };
        }

        // Check whitelist
        let is_whitelisted = self.allowed_set.is_empty() || self.allowed_set.contains(tool);

        // Check risk threshold
        let within_threshold = risk_at_or_below_threshold(assessment.risk, self.config.risk_threshold);

        // Auto-approve if whitelisted AND within risk threshold
        if is_whitelisted && within_threshold {
            return ApprovalDecision::approve(
                assessment.risk,
                format!("Auto-approved: {}", assessment.reason),
            );
        }

        // Otherwise reject (or defer to manual approval)
        ApprovalDecision::reject(
            assessment.risk,
            format!("Not in whitelist or exceeds threshold: {}", assessment.reason),
        )
    }

    /// Evaluate with adaptive risk assessment
    pub fn evaluate_adaptive(
        &self,
        tool: &str,
        input: Option<ToolInput>,
        ctx: &ExecutionContext,
    ) -> ApprovalDecision {
        let input = input.unwrap_or(ToolInput::None);

        if !self.config.enabled {
            return ApprovalDecision::reject(RiskLevel::Medium, "Auto-approve disabled");
        }

        // Evaluate adaptive risk
        let adaptive = self.evaluate_adaptive_risk(tool, &input, ctx);

        // Critical is always rejected
        if adaptive.adjusted_risk == RiskLevel::Critical {
            return ApprovalDecision {
                approved: false,
                risk: RiskLevel::Critical,
                reason: format!(
                    "Critical operation blocked (adjusted from {:?})",
                    adaptive.base_risk
                ),
                timeout_approved: false,
                auto_approvable: false,
            };
        }

        // Check whitelist
        let is_whitelisted = self.allowed_set.is_empty() || self.allowed_set.contains(tool);
        if !is_whitelisted {
            return ApprovalDecision::defer(
                adaptive.adjusted_risk,
                "Not in whitelist",
            );
        }

        // Check adjusted risk against threshold
        let within_threshold = risk_at_or_below_threshold(
            adaptive.adjusted_risk,
            self.config.risk_threshold,
        );

        if within_threshold {
            ApprovalDecision::approve(
                adaptive.adjusted_risk,
                format!(
                    "Adaptive approved: base={:?}, adjusted={:?} ({})",
                    adaptive.base_risk, adaptive.adjusted_risk, adaptive.adjustment_reason
                ),
            )
        } else {
            ApprovalDecision::defer(
                adaptive.adjusted_risk,
                format!(
                    "Exceeds adaptive threshold: base={:?}, adjusted={:?} ({})",
                    adaptive.base_risk, adaptive.adjusted_risk, adaptive.adjustment_reason
                ),
            )
        }
    }

    /// Quick check if a tool can be auto-approved (without full evaluation)
    pub fn can_auto_approve(&self, tool: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        let base_risk = tool_base_risk(tool);

        // Critical tools can never be auto-approved
        if base_risk == RiskLevel::Critical {
            return false;
        }

        // Check whitelist
        let is_whitelisted = self.allowed_set.is_empty() || self.allowed_set.contains(tool);
        if !is_whitelisted {
            return false;
        }

        // Check risk threshold
        risk_at_or_below_threshold(base_risk, self.config.risk_threshold)
    }

    /// Record an audit entry
    pub fn record_audit(&mut self, entry: AuditEntry) {
        self.audit_log.push(entry);
        if self.audit_log.len() > self.max_audit_entries {
            self.audit_log.remove(0);
        }
    }

    /// Get the audit log
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Clear the audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Adjust risk level by delta
fn adjust_risk_level(risk: RiskLevel, delta: i32) -> RiskLevel {
    let levels = [
        RiskLevel::Safe,
        RiskLevel::Low,
        RiskLevel::Medium,
        RiskLevel::High,
        RiskLevel::Critical,
    ];

    let current_index = risk.value() as i32;
    let new_index = (current_index + delta).clamp(0, 4) as usize;
    levels[new_index]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_only_config() {
        let config = AutoApproveConfig::safe_only(false);
        assert!(config.enabled);
        assert!(config.allowed_tools.contains(&"Read".to_string()));
        assert!(config.allowed_tools.contains(&"Glob".to_string()));
        assert_eq!(config.risk_threshold, RiskLevel::Low);
    }

    #[test]
    fn test_evaluate_safe_tool() {
        let engine = AutoApproveEngine::safe_only(false);
        let decision = engine.evaluate("Read", None);
        assert!(decision.approved);
        assert_eq!(decision.risk, RiskLevel::Safe);
    }

    #[test]
    fn test_evaluate_high_risk_tool() {
        let engine = AutoApproveEngine::safe_only(false);
        let decision = engine.evaluate("Bash", None);
        assert!(!decision.approved);
    }

    #[test]
    fn test_evaluate_bash_command() {
        let engine = AutoApproveEngine::permissive(false);

        // Git status should be low risk
        let decision = engine.evaluate(
            "Bash",
            Some(ToolInput::Bash { command: "git status".to_string() }),
        );
        assert!(decision.approved);
        assert_eq!(decision.risk, RiskLevel::Low);

        // Sudo should be critical
        let decision = engine.evaluate(
            "Bash",
            Some(ToolInput::Bash { command: "sudo rm -rf /".to_string() }),
        );
        assert!(!decision.approved);
        assert_eq!(decision.risk, RiskLevel::Critical);
        assert!(!decision.auto_approvable);
    }

    #[test]
    fn test_evaluate_file_operation() {
        let engine = AutoApproveEngine::permissive(false);

        // Normal file should be medium (base for Write)
        let decision = engine.evaluate(
            "Write",
            Some(ToolInput::File { path: "src/main.rs".to_string() }),
        );
        assert!(decision.approved);

        // .env file should be high risk
        let decision = engine.evaluate(
            "Write",
            Some(ToolInput::File { path: ".env".to_string() }),
        );
        assert!(!decision.approved);
        assert_eq!(decision.risk, RiskLevel::High);
    }

    #[test]
    fn test_disabled_engine() {
        let config = AutoApproveConfig::default();
        let engine = AutoApproveEngine::new(config);

        let decision = engine.evaluate("Read", None);
        assert!(!decision.approved);
        assert!(decision.reason.contains("disabled"));
    }

    #[test]
    fn test_adaptive_risk_with_errors() {
        let engine = AutoApproveEngine::permissive(false);
        let mut ctx = ExecutionContext::new("test-session");
        ctx.errors = 3;

        // Low risk tool should be elevated due to errors
        let result = engine.evaluate_adaptive_risk("Read", &ToolInput::None, &ctx);
        assert!(result.adjustment > 0);
        assert!(result.adjusted_risk > result.base_risk);
    }

    #[test]
    fn test_adaptive_risk_with_high_success() {
        let engine = AutoApproveEngine::permissive(false);
        let mut ctx = ExecutionContext::new("test-session");
        ctx.successes = 100;
        ctx.errors = 0;

        // Risk should be decreased for high success rate
        let result = engine.evaluate_adaptive_risk("Write", &ToolInput::None, &ctx);
        assert_eq!(result.adjustment, -1);
        assert!(result.adjusted_risk < result.base_risk);
    }

    #[test]
    fn test_can_auto_approve() {
        let engine = AutoApproveEngine::safe_only(false);

        assert!(engine.can_auto_approve("Read"));
        assert!(engine.can_auto_approve("Glob"));
        assert!(!engine.can_auto_approve("Bash")); // Not in whitelist and high risk
        assert!(!engine.can_auto_approve("Write")); // Not in whitelist
    }

    #[test]
    fn test_audit_log() {
        let mut engine = AutoApproveEngine::safe_only(false);

        engine.record_audit(AuditEntry::new(
            "Read",
            RiskLevel::Safe,
            "approved",
            "Test audit",
        ));

        assert_eq!(engine.audit_log().len(), 1);
        assert_eq!(engine.audit_log()[0].tool, "Read");

        engine.clear_audit_log();
        assert!(engine.audit_log().is_empty());
    }

    #[test]
    fn test_adjust_risk_level() {
        assert_eq!(adjust_risk_level(RiskLevel::Safe, 1), RiskLevel::Low);
        assert_eq!(adjust_risk_level(RiskLevel::Safe, 2), RiskLevel::Medium);
        assert_eq!(adjust_risk_level(RiskLevel::High, -1), RiskLevel::Medium);
        assert_eq!(adjust_risk_level(RiskLevel::Safe, -1), RiskLevel::Safe); // Can't go below Safe
        assert_eq!(adjust_risk_level(RiskLevel::Critical, 1), RiskLevel::Critical); // Can't go above Critical
    }
}
