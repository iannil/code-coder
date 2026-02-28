//! Auto-approval logic for tool execution.
//!
//! Determines whether a tool call should be:
//! - AutoApproved: Execute immediately without human intervention
//! - Queued: Create an approval request and wait for human decision
//! - Denied: Reject the operation outright
//!
//! Decision factors:
//! 1. Tool whitelist (explicit allowed tools)
//! 2. Risk threshold (based on risk evaluation)
//! 3. Autonomy level (from Hand configuration)

use super::manifest::{AutoApproveConfig, AutonomyLevel, RiskThreshold};
use super::risk::{RiskEvaluator, RiskEvaluation, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Decision for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    /// Auto-approve and execute immediately
    AutoApprove,
    /// Queue for human approval
    Queue,
    /// Deny execution outright
    Deny,
}

impl ApprovalDecision {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            ApprovalDecision::AutoApprove => "自动批准",
            ApprovalDecision::Queue => "等待审批",
            ApprovalDecision::Deny => "拒绝",
        }
    }
}

/// Result of approval decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    /// The decision
    pub decision: ApprovalDecision,
    /// Risk evaluation
    pub risk_evaluation: RiskEvaluation,
    /// Reasons for the decision
    pub reasons: Vec<String>,
    /// Whether timeout auto-approval applies
    pub timeout_applicable: bool,
    /// Timeout in milliseconds (if applicable)
    pub timeout_ms: Option<u64>,
}

/// Auto-approver for tool execution.
#[derive(Debug, Clone)]
pub struct AutoApprover {
    /// Configuration
    config: AutoApproveConfig,
    /// Autonomy level (affects thresholds)
    autonomy_level: AutonomyLevel,
    /// Risk evaluator
    risk_evaluator: RiskEvaluator,
    /// Allowed tools set (for fast lookup)
    allowed_tools_set: HashSet<String>,
    /// Whether unattended mode is enabled
    unattended: bool,
}

impl AutoApprover {
    /// Create a new auto-approver.
    pub fn new(
        config: AutoApproveConfig,
        autonomy_level: AutonomyLevel,
        unattended: bool,
    ) -> Self {
        let allowed_tools_set: HashSet<String> = config
            .allowed_tools
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        Self {
            config,
            autonomy_level,
            risk_evaluator: RiskEvaluator::new(),
            allowed_tools_set,
            unattended,
        }
    }

    /// Create a disabled auto-approver (all operations queue).
    pub fn disabled() -> Self {
        Self {
            config: AutoApproveConfig::default(),
            autonomy_level: AutonomyLevel::Timid,
            risk_evaluator: RiskEvaluator::new(),
            allowed_tools_set: HashSet::new(),
            unattended: false,
        }
    }

    /// Check if auto-approval is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Determine whether a tool call should be auto-approved, queued, or denied.
    pub fn should_approve(&self, tool: &str, args: &serde_json::Value) -> ApprovalResult {
        let risk_evaluation = self.risk_evaluator.evaluate(tool, args);

        // If auto-approval is disabled, everything goes to queue
        if !self.config.enabled {
            return ApprovalResult {
                decision: ApprovalDecision::Queue,
                risk_evaluation,
                reasons: vec!["Auto-approval is disabled".to_string()],
                timeout_applicable: false,
                timeout_ms: None,
            };
        }

        let mut reasons = Vec::new();

        // Check 1: Is the tool in the whitelist?
        let tool_lower = tool.to_lowercase();
        let in_whitelist = self.allowed_tools_set.contains(&tool_lower);

        if in_whitelist {
            reasons.push(format!("Tool '{}' is in allowed_tools whitelist", tool));
        }

        // Check 2: Does the risk level meet the threshold?
        let risk_meets_threshold = self.risk_meets_threshold(&risk_evaluation.risk_level);

        if risk_meets_threshold {
            reasons.push(format!(
                "Risk level {} meets threshold {:?}",
                risk_evaluation.risk_level.display_name(),
                self.config.risk_threshold
            ));
        } else {
            reasons.push(format!(
                "Risk level {} exceeds threshold {:?}",
                risk_evaluation.risk_level.display_name(),
                self.config.risk_threshold
            ));
        }

        // Decision logic:
        // - If in whitelist AND risk meets threshold: AutoApprove
        // - If in whitelist but risk exceeds threshold: Queue (whitelist doesn't override safety)
        // - If not in whitelist but risk meets threshold: AutoApprove (risk-based approval)
        // - If not in whitelist and risk exceeds threshold: Queue
        // - Critical risk always queues regardless of whitelist

        let decision = if risk_evaluation.risk_level == RiskLevel::Critical {
            reasons.push("Critical risk always requires human approval".to_string());
            ApprovalDecision::Queue
        } else if in_whitelist && risk_meets_threshold {
            ApprovalDecision::AutoApprove
        } else if !in_whitelist && risk_meets_threshold {
            // Risk-based approval (tool not in whitelist but risk is acceptable)
            ApprovalDecision::AutoApprove
        } else {
            // Risk exceeds threshold
            ApprovalDecision::Queue
        };

        // Determine if timeout auto-approval applies
        // Timeout only applies when:
        // 1. Decision is Queue
        // 2. Unattended mode is enabled
        // 3. Risk is not Critical
        // 4. timeout_ms > 0
        let timeout_applicable = decision == ApprovalDecision::Queue
            && self.unattended
            && risk_evaluation.risk_level != RiskLevel::Critical
            && self.config.timeout_ms > 0;

        let timeout_ms = if timeout_applicable {
            Some(self.config.timeout_ms)
        } else {
            None
        };

        if timeout_applicable {
            reasons.push(format!(
                "Timeout auto-approval enabled: {}ms",
                self.config.timeout_ms
            ));
        }

        ApprovalResult {
            decision,
            risk_evaluation,
            reasons,
            timeout_applicable,
            timeout_ms,
        }
    }

    /// Check if a risk level meets the configured threshold.
    fn risk_meets_threshold(&self, risk_level: &RiskLevel) -> bool {
        let threshold_value = match self.config.risk_threshold {
            RiskThreshold::Safe => 0,
            RiskThreshold::Low => 1,
            RiskThreshold::Medium => 2,
            RiskThreshold::High => 3,
        };

        risk_level.value() <= threshold_value
    }

    /// Get the risk evaluator.
    pub fn risk_evaluator(&self) -> &RiskEvaluator {
        &self.risk_evaluator
    }

    /// Get the autonomy level.
    pub fn autonomy_level(&self) -> AutonomyLevel {
        self.autonomy_level
    }

    /// Get the timeout in milliseconds.
    pub fn timeout_ms(&self) -> u64 {
        self.config.timeout_ms
    }

    /// Get allowed tools.
    pub fn allowed_tools(&self) -> &[String] {
        &self.config.allowed_tools
    }
}

/// Builder for AutoApprover with sensible defaults.
#[derive(Debug, Clone)]
pub struct AutoApproverBuilder {
    enabled: bool,
    allowed_tools: Vec<String>,
    risk_threshold: RiskThreshold,
    timeout_ms: u64,
    autonomy_level: AutonomyLevel,
    unattended: bool,
}

impl Default for AutoApproverBuilder {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tools: Vec::new(),
            risk_threshold: RiskThreshold::Low, // User's chosen default
            timeout_ms: 30000,
            autonomy_level: AutonomyLevel::Crazy,
            unattended: true,
        }
    }
}

impl AutoApproverBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable auto-approval.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set allowed tools whitelist.
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Add default safe tools to whitelist.
    pub fn with_safe_tools(mut self) -> Self {
        let safe_tools = vec![
            "Read".to_string(),
            "Glob".to_string(),
            "LS".to_string(),
            "NotebookRead".to_string(),
        ];
        self.allowed_tools.extend(safe_tools);
        self
    }

    /// Add default low-risk tools to whitelist.
    pub fn with_low_risk_tools(mut self) -> Self {
        let low_tools = vec![
            "Grep".to_string(),
            "WebSearch".to_string(),
            "WebFetch".to_string(),
            "Task".to_string(),
        ];
        self.allowed_tools.extend(low_tools);
        self
    }

    /// Set risk threshold.
    pub fn risk_threshold(mut self, threshold: RiskThreshold) -> Self {
        self.risk_threshold = threshold;
        self
    }

    /// Set timeout in milliseconds.
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }

    /// Set autonomy level.
    pub fn autonomy_level(mut self, level: AutonomyLevel) -> Self {
        self.autonomy_level = level;
        self
    }

    /// Set unattended mode.
    pub fn unattended(mut self, unattended: bool) -> Self {
        self.unattended = unattended;
        self
    }

    /// Build the AutoApprover.
    pub fn build(self) -> AutoApprover {
        let config = AutoApproveConfig {
            enabled: self.enabled,
            allowed_tools: self.allowed_tools,
            risk_threshold: self.risk_threshold,
            timeout_ms: self.timeout_ms,
        };

        AutoApprover::new(config, self.autonomy_level, self.unattended)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_disabled_approver() {
        let approver = AutoApprover::disabled();

        let result = approver.should_approve("Read", &json!({"file_path": "/test.txt"}));
        assert_eq!(result.decision, ApprovalDecision::Queue);
        assert!(!result.timeout_applicable);
    }

    #[test]
    fn test_whitelist_approval() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .allowed_tools(vec!["Read".to_string(), "Glob".to_string()])
            .risk_threshold(RiskThreshold::Low)
            .build();

        // Tool in whitelist with safe risk
        let result = approver.should_approve("Read", &json!({"file_path": "/test.txt"}));
        assert_eq!(result.decision, ApprovalDecision::AutoApprove);

        // Tool not in whitelist but safe risk
        let result = approver.should_approve("LS", &json!({}));
        assert_eq!(result.decision, ApprovalDecision::AutoApprove);
    }

    #[test]
    fn test_risk_threshold() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .risk_threshold(RiskThreshold::Low)
            .build();

        // Low risk tool should be approved
        let result = approver.should_approve("Grep", &json!({"pattern": "TODO"}));
        assert_eq!(result.decision, ApprovalDecision::AutoApprove);

        // High risk tool should queue
        let result = approver.should_approve("Bash", &json!({"command": "rm -rf /tmp"}));
        assert_eq!(result.decision, ApprovalDecision::Queue);
    }

    #[test]
    fn test_critical_always_queues() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .allowed_tools(vec!["Bash".to_string()])
            .risk_threshold(RiskThreshold::High)
            .build();

        // Even with Bash in whitelist and high threshold, critical risk queues
        let result = approver.should_approve("Bash", &json!({"command": "sudo rm -rf /"}));
        assert_eq!(result.decision, ApprovalDecision::Queue);
        assert!(result.risk_evaluation.risk_level == RiskLevel::Critical);
    }

    #[test]
    fn test_timeout_applicable() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .risk_threshold(RiskThreshold::Safe) // Only safe tools auto-approve
            .timeout_ms(30000)
            .unattended(true)
            .build();

        // High risk tool queues with timeout
        let result = approver.should_approve("Write", &json!({"file_path": "/test.txt"}));
        assert_eq!(result.decision, ApprovalDecision::Queue);
        assert!(result.timeout_applicable);
        assert_eq!(result.timeout_ms, Some(30000));
    }

    #[test]
    fn test_timeout_not_applicable_critical() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .risk_threshold(RiskThreshold::Safe)
            .timeout_ms(30000)
            .unattended(true)
            .build();

        // Critical risk never gets timeout approval
        let result = approver.should_approve("Bash", &json!({"command": "rm -rf /"}));
        assert_eq!(result.decision, ApprovalDecision::Queue);
        assert!(!result.timeout_applicable);
    }

    #[test]
    fn test_builder_with_safe_tools() {
        let approver = AutoApproverBuilder::new()
            .enabled(true)
            .with_safe_tools()
            .with_low_risk_tools()
            .risk_threshold(RiskThreshold::Low)
            .build();

        assert!(approver.allowed_tools().contains(&"Read".to_string()));
        assert!(approver.allowed_tools().contains(&"Grep".to_string()));
    }
}
